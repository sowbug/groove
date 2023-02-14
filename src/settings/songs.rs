use super::{
    controllers::{ControlPathSettings, ControlTripSettings},
    ClockSettings, ControlSettings, DeviceSettings, PatternSettings, TrackSettings,
};
use crate::{
    common::DeviceId,
    controllers::{orchestrator::Orchestrator, ControlPath, ControlTrip},
    entities::BoxedEntity,
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

    // TODO: it would be nice if zero patch cables automatically hooked up all
    // the audio sources.
    //
    /// Virtual audio cables connecting a series of devices
    #[serde(default)]
    pub patch_cables: Vec<PatchCable>,

    /// Automation links between a source device and a target device's
    /// controllable parameter
    #[serde(default)]
    pub controls: Vec<ControlSettings>,

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
        let mut settings: SongSettings = serde_yaml::from_str(yaml)?;

        // TODO: this is a hack that seems to be necessary because if you set a
        // #[serde(skip)] on a field, then it doesn't seem to pick up the value
        // from the Default impl. So we were getting 0 as the default sample
        // rate once we dropped that field from serialization.
        //
        // TODO: think (again) about whether Serde structs should be closer or
        // farther to the heart of the model.
        if settings.clock.sample_rate() == 0 {
            settings.clock.set_sample_rate(44100);
        }
        Ok(settings)
    }

    pub fn instantiate(&self, load_only_test_entities: bool) -> Result<Orchestrator> {
        let mut o = Orchestrator::default();
        o.set_title(self.title.clone());
        o.set_clock_settings(&self.clock);
        self.instantiate_devices(&mut o, load_only_test_entities);
        self.instantiate_patch_cables(&mut o)?;
        self.instantiate_controls(&mut o)?;
        self.instantiate_tracks(&mut o);
        self.instantiate_control_trips(&mut o, &self.clock.time_signature());
        Ok(o)
    }

    fn instantiate_devices(&self, orchestrator: &mut Orchestrator, load_only_test_entities: bool) {
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
                        settings.instantiate(sample_rate, load_only_test_entities);
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

    fn instantiate_patch_cables(&self, orchestrator: &mut Orchestrator) -> anyhow::Result<()> {
        for patch_cable in &self.patch_cables {
            if patch_cable.len() < 2 {
                eprintln!("Warning: ignoring patch cable with only one ID.");
                continue;
            }
            let mut last_device_uvid: Option<DeviceId> = None;
            for device_uvid in patch_cable {
                if let Some(last_device_uvid) = last_device_uvid {
                    match orchestrator.get_uid(&last_device_uvid) {
                        Some(last_device_uid) => match orchestrator.get_uid(device_uvid) {
                            Some(device_uid) => {
                                if let Err(e) = orchestrator.patch(last_device_uid, device_uid) {
                                    eprintln!("Error when patching input {device_uvid} to output {last_device_uvid}: {e}");
                                    return Err(e);
                                }
                            }
                            None => {
                                eprintln!("Warning: input patch ID '{device_uvid}' not found.");
                            }
                        },
                        None => {
                            eprintln!("Warning: output patch ID '{last_device_uvid}' not found.");
                        }
                    }
                }
                last_device_uvid = Some(device_uvid.to_string());
            }
        }
        Ok(())
    }

    fn instantiate_controls(&self, orchestrator: &mut Orchestrator) -> anyhow::Result<()> {
        for control in self.controls.iter() {
            let source_uvid = &control.source;
            let target_uvid = &control.target.id;
            let target_param_name = control.target.param.as_str();

            let controller_uid;
            if let Some(uid) = orchestrator.store().get_uid(&source_uvid) {
                controller_uid = uid;
            } else {
                eprintln!(
                    "Warning: couldn't find control source ID {}. Skipping automation ID {}",
                    source_uvid, control.id
                );
                continue;
            }
            let target_uid;
            if let Some(uid) = orchestrator.store().get_uid(&target_uvid) {
                target_uid = uid;
            } else {
                eprintln!(
                    "Warning: couldn't find control target ID {}. Skipping automation ID {}",
                    target_uvid, control.id
                );
                continue;
            }
            let result = orchestrator.link_control(controller_uid, target_uid, &target_param_name);
            if let Err(error_text) = result {
                eprintln!(
                    "Warning: skipping automation ID {} because of error '{}'",
                    control.id, error_text
                );
            }
        }
        Ok(())
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
            let mut programmer = PatternProgrammer::new_with(&self.clock.time_signature);

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
        orchestrator: &mut Orchestrator,
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
            let trip_id = control_trip_settings.id.as_str();
            if let Some(target_uid) = orchestrator.get_uid(&control_trip_settings.target.id) {
                let mut control_trip = Box::new(ControlTrip::default());
                for path_id in &control_trip_settings.path_ids {
                    if let Some(control_path) = ids_to_paths.get(path_id) {
                        control_trip.add_path(time_signature, control_path);
                    } else {
                        eprintln!(
                            "Warning: trip {} refers to nonexistent path {}",
                            trip_id, path_id
                        );
                    }
                }
                let controller_uid =
                    orchestrator.add(Some(trip_id), BoxedEntity::ControlTrip(control_trip));
                if let Err(err_result) = orchestrator.link_control(
                    controller_uid,
                    target_uid,
                    &control_trip_settings.target.param,
                ) {
                    eprintln!(
                        "Warning: trip {} not added because of error '{}'",
                        trip_id, err_result
                    );
                }
            } else {
                eprintln!(
                    "Warning: trip {} controls nonexistent entity {}",
                    trip_id, control_trip_settings.target.id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SongSettings;
    use crate::git_hash;
    use crate::{clock::Clock, IOHelper, Paths, StereoSample};
    use crossbeam::deque::Steal;
    use std::fs::File;
    use std::io::prelude::*;
    use std::time::Instant;

    #[test]
    fn yaml_loads_and_parses() {
        let mut path = Paths::test_data_path();
        path.push("kitchen-sink.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let mut orchestrator = song_settings
            .instantiate(false)
            .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));
        let mut clock = Clock::new_with(orchestrator.clock_settings());
        let performance = orchestrator
            .run_performance(&mut clock, false)
            .unwrap_or_else(|err| panic!("performance failed: {:?}", err));

        assert!(
            !performance.worker.is_empty(),
            "Orchestrator reported successful performance, but performance is empty."
        );

        let stealer = performance.worker.stealer();
        let mut found_non_silence = false;
        loop {
            if let Steal::Success(sample) = stealer.steal() {
                if sample != StereoSample::SILENCE {
                    found_non_silence = true;
                    continue;
                }
            }
            break;
        }
        assert!(found_non_silence, "Performance contains only silence.");

        // TODO: maybe make a Paths:: function for out/
        assert!(IOHelper::send_performance_to_file(
            &performance,
            "out/test_yaml_loads_and_parses-kitchen-sink.wav",
        )
        .is_ok());
    }

    #[test]
    fn spit_out_perf_data() {
        let mut path = Paths::test_data_path();
        path.push("perf-1.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let mut orchestrator = song_settings
            .instantiate(false)
            .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));
        let mut clock = Clock::new_with(orchestrator.clock_settings());

        let start_instant = Instant::now();
        let performance = orchestrator
            .run_performance(&mut clock, false)
            .unwrap_or_else(|err| panic!("performance failed: {:?}", err));
        let elapsed = start_instant.elapsed();
        let frame_count = performance.worker.len();

        let mut file = File::create("perf-output.txt").unwrap();
        let output = format!(
            "Version    : {}\n\
Git hash   : {:.8}\n\
\n\
Elapsed    : {:0.3}s\n\
Frames     : {}\n\
Frames/msec: {:.2?} (goal >{:.2?})\n\
usec/frame : {:.2?} (goal <{:.2?})",
            env!("CARGO_PKG_VERSION"),
            git_hash(),
            elapsed.as_secs_f32(),
            frame_count,
            frame_count as f32 / start_instant.elapsed().as_millis() as f32,
            performance.sample_rate as f32 / 1000.0,
            start_instant.elapsed().as_micros() as f32 / frame_count as f32,
            1000000.0 / performance.sample_rate as f32
        );
        let _ = file.write(output.as_bytes());

        assert!(IOHelper::send_performance_to_file(&performance, "out/perf-1.wav",).is_ok());
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

    #[test]
    fn test_patching_to_device_with_no_input_fails_with_proper_error() {
        let mut path = Paths::project_path();
        path.push("tests/instruments-have-no-inputs.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let r = song_settings.instantiate(false);
        assert_eq!(
            r.unwrap_err().to_string(),
            "Input device doesn't transform audio and can't be patched from output device"
        );
    }
}
