use super::{
    control::{ControlPathSettings, ControlTripSettings},
    ClockSettings, DeviceSettings, LoadError, PatternSettings, TrackSettings,
};
use crate::{
    clock::WatchedClock,
    common::{rrc, rrc_clone, rrc_downgrade, DeviceId},
    control::{ControlPath, ControlTrip},
    patterns::{Pattern, PatternSequencer},
    Orchestrator,
};
use anyhow::{Error, Ok};
use serde::{Deserialize, Serialize};

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

    pub fn new_from_yaml(yaml: &str) -> Result<Self, LoadError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            println!("{e}");
            LoadError::FormatError
        })
    }

    pub fn instantiate(&self) -> Result<Orchestrator, Error> {
        let mut o = Orchestrator::new();
        o.set_watched_clock(WatchedClock::new_with(&self.clock));
        self.instantiate_devices(&mut o);
        self.instantiate_patch_cables(&mut o);
        self.instantiate_tracks(&mut o);
        self.instantiate_control_trips(&mut o);
        Ok(o)
    }

    fn instantiate_devices(&self, orchestrator: &mut Orchestrator) {
        let sample_rate = self.clock.sample_rate();

        for device in &self.devices {
            match device {
                DeviceSettings::Instrument(id, instrument_settings) => {
                    let instrument = instrument_settings.instantiate(sample_rate);
                    let midi_channel = instrument.borrow().midi_channel();
                    let instrument_weak = rrc_downgrade(&instrument);
                    orchestrator.connect_to_downstream_midi_bus(midi_channel, instrument_weak);
                    let audio_source = rrc_clone(&instrument);
                    orchestrator.register_audio_source(Some(id), audio_source);
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
                    orchestrator.register_viewable(effect);
                }
            }
        }
    }

    fn instantiate_patch_cables(&self, orchestrator: &mut Orchestrator) {
        for patch_cable in &self.patch_cables {
            if patch_cable.len() < 2 {
                dbg!("ignoring patch cable of length < 2");
                continue;
            }
            let mut last_device_id: Option<DeviceId> = None;
            for device_id in patch_cable {
                if let Some(ldi) = last_device_id {
                    let output = orchestrator.audio_source_by(&ldi);
                    if device_id == "main-mixer" {
                        orchestrator.add_main_mixer_source(output);
                    } else {
                        let input = orchestrator.audio_sink_by(device_id);
                        if let Some(input) = input.upgrade() {
                            input.borrow_mut().add_audio_source(output);
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
    fn instantiate_tracks(&self, orchestrator: &mut Orchestrator) {
        if self.tracks.is_empty() {
            return;
        }

        let pattern_sequencer = rrc(PatternSequencer::new(&self.clock.time_signature));
        for pattern_settings in &self.patterns {
            let pattern = rrc(Pattern::from_settings(pattern_settings));
            orchestrator.register_pattern(Some(&pattern_settings.id), pattern);
        }

        for track in &self.tracks {
            let channel = track.midi_channel;
            pattern_sequencer.borrow_mut().reset_cursor();
            for pattern_id in &track.pattern_ids {
                if let Some(pattern) = orchestrator.pattern_by(pattern_id).upgrade() {
                    pattern_sequencer
                        .borrow_mut()
                        .add_pattern(&pattern.borrow(), channel);
                }
            }
        }

        let instrument = rrc_clone(&pattern_sequencer);
        orchestrator.connect_to_upstream_midi_bus(instrument);
        orchestrator.register_clock_watcher(None, pattern_sequencer);
    }

    fn instantiate_control_trips(&self, orchestrator: &mut Orchestrator) {
        if self.trips.is_empty() {
            // There's no need to instantiate the paths if there are no trips to use them.
            return;
        }

        for path_settings in &self.paths {
            let v = rrc(ControlPath::from_settings(path_settings));
            orchestrator.register_control_path(Some(&path_settings.id), v);
        }
        for control_trip_settings in &self.trips {
            if let Some(target) = orchestrator
                .makes_control_sink_by(&control_trip_settings.target.id)
                .upgrade()
            {
                if let Some(controller) = target
                    .borrow()
                    .make_control_sink(&control_trip_settings.target.param)
                {
                    let control_trip = rrc(ControlTrip::new(controller));
                    control_trip.borrow_mut().reset_cursor();
                    for path_id in &control_trip_settings.path_ids {
                        let control_path = orchestrator.control_path_by(path_id);
                        if let Some(control_path) = control_path.upgrade() {
                            control_trip.borrow_mut().add_path(&control_path.borrow());
                        }
                    }
                    orchestrator
                        .register_clock_watcher(Some(&control_trip_settings.id), control_trip);
                } else {
                    panic!(
                        "someone instantiated a MakesControlSink without proper wrapping: {:?}.",
                        target
                    );
                };
            } else {
                panic!("an upgrade failed. YOU HAD ONE JOB");
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::SongSettings;

    #[test]
    fn test_yaml_loads_and_parses() {
        let yaml = std::fs::read_to_string("test_data/kitchen-sink.yaml").unwrap();
        if let Ok(song_settings) = SongSettings::new_from_yaml(yaml.as_str()) {
            if let Ok(mut orchestrator) = song_settings.instantiate() {
                if let Ok(_performance) = orchestrator.perform() {
                    // cool
                } else {
                    panic!("performance failed");
                }
            } else {
                panic!("instantiation failed");
            }
        } else {
            panic!("loading settings failed");
        }
    }
}
