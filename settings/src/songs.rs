// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    controllers::{ControlPathSettings, ControlTripSettings},
    ClockSettings, ControlSettings, DeviceId, DeviceSettings, PatternSettings, TrackSettings,
};
use anyhow::Result;
use groove_core::{time::TimeSignature, ParameterType};
use groove_entities::controllers::{ControlPath, ControlTrip, Note, Pattern, PatternProgrammer};
use groove_orchestration::{Entity, Orchestrator};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

type PatchCable = Vec<DeviceId>; // first is source, last is sink

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct SongSettings {
    /// The user-visible name of this project
    pub title: Option<String>,

    /// Information about timing. BPM, time signature, etc.
    #[serde(rename = "clock")]
    pub clock_settings: ClockSettings,

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

    // TODO: this should take a PathBuf so it's easier to tell YAML from filenames
    pub fn new_from_yaml_file(filename: &str) -> anyhow::Result<Self> {
        Self::new_from_yaml(std::fs::read_to_string(filename)?.as_str())
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
        if settings.clock_settings.sample_rate == 0 {
            settings.clock_settings.sample_rate = 44100;
        }
        Ok(settings)
    }

    pub fn instantiate(
        &self,
        base_path: &PathBuf,
        load_only_test_entities: bool,
    ) -> Result<Orchestrator> {
        let mut o: Orchestrator = self.clock_settings.into();
        o.set_title(self.title.clone());
        self.instantiate_devices(
            &mut o,
            &self.clock_settings,
            base_path,
            load_only_test_entities,
        );
        self.instantiate_patch_cables(&mut o)?;
        self.instantiate_controls(&mut o)?;
        self.instantiate_tracks(&mut o);
        self.instantiate_control_trips(&mut o, &self.clock_settings.time_signature.into());
        Ok(o)
    }

    fn instantiate_devices(
        &self,
        orchestrator: &mut Orchestrator,
        clock_settings: &ClockSettings,
        base_path: &PathBuf,
        load_only_test_entities: bool,
    ) {
        let sample_rate = self.clock_settings.sample_rate;

        for device in &self.devices {
            match device {
                DeviceSettings::Instrument(uvid, settings) => {
                    let (channel, entity) =
                        settings.instantiate(sample_rate, base_path, load_only_test_entities);
                    let uid = orchestrator.add_with_uvid(entity, uvid);
                    orchestrator.connect_midi_downstream(uid, channel);
                }
                DeviceSettings::Controller(uvid, settings) => {
                    let (channel_in, _channel_out, entity) = settings.instantiate(
                        clock_settings.sample_rate,
                        clock_settings.beats_per_minute as ParameterType,
                        load_only_test_entities,
                    );
                    let uid = orchestrator.add_with_uvid(entity, uvid);
                    // TODO: do we care about channel_out?
                    orchestrator.connect_midi_downstream(uid, channel_in);
                }
                DeviceSettings::Effect(uvid, settings) => {
                    let entity = settings.instantiate(sample_rate, load_only_test_entities);
                    let _uid = orchestrator.add_with_uvid(entity, uvid);
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
                    match orchestrator.get_uid_by_uvid(&last_device_uvid) {
                        Some(last_device_uid) => match orchestrator.get_uid_by_uvid(device_uvid) {
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
            if let Some(uid) = orchestrator.get_uid_by_uvid(source_uvid) {
                controller_uid = uid;
            } else {
                eprintln!(
                    "Warning: couldn't find control source ID {}. Skipping automation ID {}",
                    source_uvid, control.id
                );
                continue;
            }
            let target_uid;
            if let Some(uid) = orchestrator.get_uid_by_uvid(target_uvid) {
                target_uid = uid;
            } else {
                eprintln!(
                    "Warning: couldn't find control target ID {}. Skipping automation ID {}",
                    target_uvid, control.id
                );
                continue;
            }
            let result = orchestrator.link_control_by_name(controller_uid, target_uid, target_param_name);
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
        if let Some(Entity::PatternManager(pattern_manager)) =
            orchestrator.get_mut(pattern_manager_uid)
        {
            for pattern_settings in self.patterns.iter() {
                let id = pattern_settings.id.clone();
                let pattern: Pattern<Note> = (*pattern_settings).into_pattern();
                if ids_to_patterns.contains_key(&id) {
                    eprintln!(
                        "WARNING: duplicate pattern ID {}. Skipping all but one!",
                        id
                    );
                    continue;
                }
                ids_to_patterns.insert(pattern_settings.id.clone(), pattern.clone());
                pattern_manager.register(pattern);
            }
        }

        let sequencer_uid = orchestrator.sequencer_uid();
        if let Some(Entity::Sequencer(sequencer)) = orchestrator.get_mut(sequencer_uid) {
            let mut programmer =
                PatternProgrammer::new_with(&self.clock_settings.time_signature.into());

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

        let mut ids_to_paths: FxHashMap<String, ControlPath> = FxHashMap::default();
        for path_settings in &self.paths {
            ids_to_paths.insert(
                path_settings.id.clone(),
                (*path_settings).derive_control_path(),
            );
        }
        for control_trip_settings in &self.trips {
            let trip_uvid = control_trip_settings.id.as_str();
            if let Some(target_uid) = orchestrator.get_uid_by_uvid(&control_trip_settings.target.id)
            {
                let mut control_trip = Box::new(ControlTrip::new_with(
                    orchestrator.sample_rate(),
                    orchestrator.time_signature(),
                    orchestrator.bpm(),
                ));
                for path_id in &control_trip_settings.path_ids {
                    if let Some(control_path) = ids_to_paths.get(path_id) {
                        control_trip.add_path(time_signature, control_path);
                    } else {
                        eprintln!(
                            "Warning: trip {} refers to nonexistent path {}",
                            trip_uvid, path_id
                        );
                    }
                }
                let controller_uid =
                    orchestrator.add_with_uvid(Entity::ControlTrip(control_trip), trip_uvid);
                if let Err(err_result) = orchestrator.link_control_by_name(
                    controller_uid,
                    target_uid,
                    &control_trip_settings.target.param,
                ) {
                    eprintln!(
                        "Warning: trip {} not added because of error '{}'",
                        trip_uvid, err_result
                    );
                }
            } else {
                eprintln!(
                    "Warning: trip {} controls nonexistent entity {}",
                    trip_uvid, control_trip_settings.target.id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SongSettings;

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
