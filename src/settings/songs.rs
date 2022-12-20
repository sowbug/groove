use super::{
    controllers::{ControlPathSettings, ControlTripSettings},
    ClockSettings, DeviceSettings, PatternSettings, TrackSettings,
};
use crate::{
    common::DeviceId,
    controllers::{orchestrator::GrooveOrchestrator, ControlPath, ControlTrip},
    entities::BoxedEntity,
    messages::EntityMessage,
    midi::{
        patterns::{Note, Pattern},
        programmers::PatternProgrammer,
    },
    TimeSignature,
};
use anyhow::Result;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

type PatchCable = Vec<DeviceId>; // first is source, last is sink

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct SongSettings {
    /// The user-visible name of this project
    pub title: Option<String>,

    /// Information about timing. BPM, time signature, etc.
    pub clock: ClockSettings,

    /// Controllers, Effects, and Instruments
    pub devices: Vec<DeviceSettings>,

    /// Virtual audio cables connecting a series of devices
    pub patch_cables: Vec<PatchCable>,

    /// Tracker-style note patterns
    #[serde(default)]
    pub patterns: Vec<PatternSettings>,

    /// Sequences of Patterns making up a track
    #[serde(default)]
    pub tracks: Vec<TrackSettings>,

    // Patterns for automation
    #[serde(default)]
    pub paths: Vec<ControlPathSettings>,

    // Sequences of automation patterns
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

    pub fn instantiate(&self, load_only_test_entities: bool) -> Result<GrooveOrchestrator> {
        let mut o = GrooveOrchestrator::default();
        o.set_title(self.title.clone());
        o.set_clock_settings(&self.clock);
        self.instantiate_devices(&mut o, load_only_test_entities);
        self.instantiate_patch_cables(&mut o);
        self.instantiate_tracks(&mut o);
        self.instantiate_control_trips(&mut o, &self.clock.time_signature());
        Ok(o)
    }

    fn instantiate_devices(
        &self,
        orchestrator: &mut GrooveOrchestrator,
        load_only_test_entities: bool,
    ) {
        let sample_rate = self.clock.sample_rate();

        for device in &self.devices {
            match device {
                DeviceSettings::Instrument(id, settings) => {
                    let (channel, entity) =
                        settings.instantiate(sample_rate, load_only_test_entities);
                    let uid = orchestrator.add(Some(id), entity);
                    orchestrator.connect_midi_downstream(uid, channel);
                }
                DeviceSettings::Controller(id, settings) => {
                    let (channel_in, _channel_out, entity) =
                        settings.instantiate(load_only_test_entities);
                    let uid = orchestrator.add(Some(id), entity);
                    // TODO: do we care about channel_out?
                    orchestrator.connect_midi_downstream(uid, channel_in);
                }
                DeviceSettings::Effect(id, settings) => {
                    let entity = settings.instantiate(sample_rate, load_only_test_entities);
                    let _uid = orchestrator.add(Some(id), entity);
                }
            }
        }
    }

    fn instantiate_patch_cables(&self, orchestrator: &mut GrooveOrchestrator) {
        for patch_cable in &self.patch_cables {
            if patch_cable.len() < 2 {
                eprintln!("Warning: ignoring patch cable with only one ID.");
                continue;
            }
            let mut last_device_uvid: Option<DeviceId> = None;
            for device_id in patch_cable {
                if let Some(last_device_uvid) = last_device_uvid {
                    if let Some(last_device_uid) = orchestrator.get_uid(&last_device_uvid) {
                        if let Some(device_uid) = orchestrator.get_uid(device_id) {
                            let _ = orchestrator.patch(last_device_uid, device_uid);
                        } else {
                            eprintln!("Warning: input patch ID '{}' not found.", device_id);
                        }
                    } else {
                        eprintln!("Warning: output patch ID '{}' not found.", last_device_uvid);
                    }
                }
                last_device_uvid = Some(device_id.to_string());
            }
        }
    }

    // TODO: for now, a track has a single time signature. Each pattern can have its
    // own to override the track's, but that's unwieldy compared to a single signature
    // change as part of the overall track sequence. Maybe a pattern can be either
    // a pattern or a TS change...
    //
    // TODO - should PatternSequencers be able to change their base time signature? Probably
    fn instantiate_tracks(&self, orchestrator: &mut GrooveOrchestrator) {
        if self.tracks.is_empty() {
            return;
        }

        let mut ids_to_patterns = FxHashMap::default();
        let pattern_manager_uid = orchestrator.pattern_manager_uid();
        if let Some(BoxedEntity::PatternManager(pattern_manager)) =
            orchestrator.store_mut().get_mut(pattern_manager_uid)
        {
            for pattern_settings in &self.patterns {
                let pattern = Pattern::<Note>::from_settings(pattern_settings);
                ids_to_patterns.insert(pattern_settings.id.clone(), pattern.clone());
                pattern_manager.register(pattern);
            }
        }

        let beat_sequencer_uid = orchestrator.beat_sequencer_uid();
        if let Some(BoxedEntity::BeatSequencer(sequencer)) =
            orchestrator.store_mut().get_mut(beat_sequencer_uid)
        {
            let mut programmer =
                PatternProgrammer::<EntityMessage>::new_with(&self.clock.time_signature);

            for track in &self.tracks {
                let channel = track.midi_channel;
                programmer.reset_cursor();
                for pattern_id in &track.pattern_ids {
                    if let Some(pattern) = ids_to_patterns.get(pattern_id) {
                        programmer.insert_pattern_at_cursor(sequencer, &channel, pattern);
                    }
                }
            }
        }
    }

    fn instantiate_control_trips(
        &self,
        orchestrator: &mut GrooveOrchestrator,
        time_signature: &TimeSignature,
    ) {
        if self.trips.is_empty() {
            // There's no need to instantiate the paths if there are no trips to use them.
            return;
        }

        let mut ids_to_paths = FxHashMap::default();
        for path_settings in &self.paths {
            ids_to_paths.insert(
                path_settings.id.clone(),
                ControlPath::from_settings(path_settings),
            );
        }
        for control_trip_settings in &self.trips {
            if let Some(target_uid) = orchestrator.get_uid(&control_trip_settings.target.id) {
                let mut control_trip = Box::new(ControlTrip::<EntityMessage>::default());
                for path_id in &control_trip_settings.path_ids {
                    if let Some(control_path) = ids_to_paths.get(path_id) {
                        control_trip.add_path(time_signature, control_path);
                    } else {
                        eprintln!(
                            "Warning: trip {} refers to nonexistent path {}",
                            control_trip_settings.id, path_id
                        );
                    }
                }
                let controller_uid = orchestrator.add(
                    Some(&control_trip_settings.id),
                    BoxedEntity::ControlTrip(control_trip),
                );
                orchestrator.link_control(
                    controller_uid,
                    target_uid,
                    &control_trip_settings.target.param,
                );
            } else {
                eprintln!(
                    "Warning: trip {} controls nonexistent entity {}",
                    control_trip_settings.id, control_trip_settings.target.id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SongSettings;
    use crate::{clock::Clock, IOHelper, Paths};

    #[test]
    fn test_yaml_loads_and_parses() {
        let mut path = Paths::project_path();
        path.push("kitchen-sink.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let mut orchestrator = song_settings
            .instantiate(false)
            .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));
        let mut clock = Clock::default();
        let performance = orchestrator
            .run_performance(&mut clock, false)
            .unwrap_or_else(|err| panic!("performance failed: {:?}", err));

        // TODO: maybe make a Paths:: function for out/
        assert!(IOHelper::send_performance_to_file(
            performance,
            "out/test_yaml_loads_and_parses-kitchen-sink.wav",
        )
        .is_ok());
    }

    #[test]
    fn test_empty_file_fails_with_proper_error() {
        let r = SongSettings::new_from_yaml("");
        assert_eq!(r.unwrap_err().to_string(), "EOF while parsing a value");
    }

    #[test]
    fn test_garbage_file_fails_with_proper_error() {
        let r = SongSettings::new_from_yaml("da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert!(r
            .unwrap_err()
            .to_string()
            .contains("expected struct SongSettings at line 1 column 1"));
    }

    #[test]
    fn test_valid_yaml_bad_song_file_fails_with_proper_error() {
        let r = SongSettings::new_from_yaml(
            "---\ndo: \"a deer, a female deer\"\nre: \"a drop of golden sun\"",
        );
        assert_eq!(
            r.unwrap_err().to_string(),
            "missing field `clock` at line 2 column 3"
        );
    }
}
