use super::{
    control::{ControlPathSettings, ControlTripSettings},
    ClockSettings, DeviceSettings, PatternSettings, TrackSettings,
};
use crate::{
    clock::WatchedClock,
    common::{rrc, rrc_clone, rrc_downgrade, DeviceId},
    control::{ControlPath, ControlTrip},
    messages::GrooveMessage,
    midi::{
        patterns::{Note, Pattern},
        programmers::PatternProgrammer,
        sequencers::BeatSequencer,
    },
    traits::{IsEffect, IsMidiInstrument},
    OldOrchestrator,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type PatchCable = Vec<DeviceId>; // first is source, last is sink

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct SongSettings {
    pub clock: ClockSettings,

    pub devices: Vec<DeviceSettings>,
    pub patch_cables: Vec<PatchCable>,
    #[serde(default)]
    pub patterns: Vec<PatternSettings>,
    #[serde(default)]
    pub tracks: Vec<TrackSettings>,
    #[serde(default)]
    pub paths: Vec<ControlPathSettings>,
    #[serde(default)]
    pub trips: Vec<ControlTripSettings>,
}

impl SongSettings {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_from_yaml(yaml: &str) -> anyhow::Result<Self> {
        let settings = serde_yaml::from_str(yaml)?;
        Ok(settings)
    }

    pub fn instantiate(&self) -> Result<OldOrchestrator> {
        let mut o = OldOrchestrator::new();
        o.set_watched_clock(WatchedClock::new_with(&self.clock));
        self.instantiate_devices(&mut o);
        self.instantiate_patch_cables(&mut o);
        self.instantiate_tracks(&mut o);
        self.instantiate_control_trips(&mut o);
        Ok(o)
    }

    fn instantiate_devices(&self, orchestrator: &mut OldOrchestrator) {
        let sample_rate = self.clock.sample_rate();

        for device in &self.devices {
            match device {
                DeviceSettings::Instrument(id, instrument_settings) => {
                    let instrument = instrument_settings.instantiate(sample_rate);
                    let midi_channel = instrument.borrow().midi_channel();
                    orchestrator.connect_to_downstream_midi_bus(
                        midi_channel,
                        rrc_downgrade::<dyn IsMidiInstrument>(&instrument),
                    );
                    orchestrator.register_audio_source(
                        Some(id),
                        rrc_clone::<dyn IsMidiInstrument>(&instrument),
                    );
                    orchestrator.register_viewable(instrument);
                }
                DeviceSettings::MidiInstrument(id, midi_instrument_settings) => {
                    let midi_instrument = midi_instrument_settings.instantiate(sample_rate);
                    let midi_channel = midi_instrument.borrow().midi_channel();
                    orchestrator.register_midi_effect(
                        Some(id),
                        rrc_clone(&midi_instrument),
                        midi_channel,
                    );
                    orchestrator.register_viewable(midi_instrument);
                }
                DeviceSettings::Effect(id, effect_settings) => {
                    let effect = effect_settings.instantiate(sample_rate);
                    orchestrator.register_effect(Some(id), rrc_clone(&effect));
                    orchestrator.register_updateable(Some(id), rrc_clone::<dyn IsEffect>(&effect));
                    orchestrator.register_viewable(rrc_clone::<dyn IsEffect>(&effect));
                }
            }
        }
    }

    fn instantiate_patch_cables(&self, orchestrator: &mut OldOrchestrator) {
        for patch_cable in &self.patch_cables {
            if patch_cable.len() < 2 {
                dbg!("ignoring patch cable of length < 2");
                continue;
            }
            let mut last_device_id: Option<DeviceId> = None;
            for device_id in patch_cable {
                if let Some(ldi) = last_device_id {
                    if let Ok(output) = orchestrator.audio_source_by(&ldi) {
                        if device_id == "main-mixer" {
                            orchestrator.add_main_mixer_source(output);
                        } else {
                            if let Ok(input) = orchestrator.audio_sink_by(device_id) {
                                if let Some(input) = input.upgrade() {
                                    input.borrow_mut().add_audio_source(output);
                                }
                            }
                        }
                    }
                }
                last_device_id = Some(device_id.to_string());
            }
        }
    }

    // TODO: for now, a track has a single time signature. Each pattern can have its
    // own to override the track's, but that's unwieldy compared to a single signature
    // change as part of the overall track sequence. Maybe a pattern can be either
    // a pattern or a TS change...
    //
    // TODO - should PatternSequencers be able to change their base time signature? Probably
    fn instantiate_tracks(&self, orchestrator: &mut OldOrchestrator) {
        if self.tracks.is_empty() {
            return;
        }

        let mut ids_to_patterns: HashMap<String, Pattern<Note>> = HashMap::new();
        let pattern_manager = orchestrator.pattern_manager_mut();
        for pattern_settings in &self.patterns {
            let pattern = Pattern::<Note>::from_settings(pattern_settings);
            ids_to_patterns.insert(pattern_settings.id.clone(), pattern.clone());
            pattern_manager.register(pattern);
        }
        let sequencer = BeatSequencer::new_wrapped();
        let mut programmer =
            PatternProgrammer::<GrooveMessage>::new_with(&self.clock.time_signature);

        for track in &self.tracks {
            let channel = track.midi_channel;
            programmer.reset_cursor();
            for pattern_id in &track.pattern_ids {
                if let Some(pattern) = ids_to_patterns.get(pattern_id) {
                    programmer.insert_pattern_at_cursor(rrc_clone(&sequencer), &channel, &pattern);
                }
            }
        }

        orchestrator
            .connect_to_upstream_midi_bus(rrc_clone::<BeatSequencer<GrooveMessage>>(&sequencer));
        orchestrator
            .register_clock_watcher(None, rrc_clone::<BeatSequencer<GrooveMessage>>(&sequencer));
        orchestrator.register_viewable(sequencer);
    }

    fn instantiate_control_trips(&self, orchestrator: &mut OldOrchestrator) {
        if self.trips.is_empty() {
            // There's no need to instantiate the paths if there are no trips to use them.
            return;
        }

        for path_settings in &self.paths {
            let v = rrc(ControlPath::from_settings(path_settings));
            orchestrator.register_control_path(Some(&path_settings.id), v);
        }
        for control_trip_settings in &self.trips {
            if let Ok(uid) = orchestrator.updateable_uid_by(&control_trip_settings.target.id) {
                if let Ok(updateable) = orchestrator.updateable_by(uid) {
                    if let Some(updateable) = updateable.upgrade() {
                        let m = updateable
                            .borrow()
                            .message_for(&control_trip_settings.target.param);
                        let control_trip = rrc(ControlTrip::new(uid, m));
                        control_trip.borrow_mut().reset_cursor();
                        for path_id in &control_trip_settings.path_ids {
                            if let Ok(control_path) = orchestrator.control_path_by(path_id) {
                                if let Some(control_path) = control_path.upgrade() {
                                    control_trip.borrow_mut().add_path(&control_path.borrow());
                                }
                            }
                        }
                        orchestrator
                            .register_clock_watcher(Some(&control_trip_settings.id), control_trip);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::SongSettings;

    #[test]
    fn test_yaml_loads_and_parses() {
        if let Ok(yaml) = std::fs::read_to_string("test_data/kitchen-sink.yaml") {
            if let Ok(song_settings) = SongSettings::new_from_yaml(yaml.as_str()) {
                if let Ok(mut orchestrator) = song_settings.instantiate() {
                    if let Ok(_performance) = orchestrator.perform() {
                        // cool
                    } else {
                        dbg!("performance failed");
                    }
                } else {
                    dbg!("instantiation failed");
                }
            } else {
                dbg!("parsing settings failed");
            }
        } else {
            dbg!("loading YAML failed");
        }
    }
}
