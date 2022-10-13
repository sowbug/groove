use crate::{
    common::{DeviceId, Rrc, Ww},
    control::ControlPath,
    patterns::Pattern,
    traits::{IsEffect, IsMidiEffect, MakesControlSink, SinksAudio, SourcesAudio, WatchesClock},
};
use std::rc::Rc;
use std::{collections::HashMap, rc::Weak};

#[derive(Debug, Default)]
pub(crate) struct IdStore {
    next_id: usize,

    // These are all Weaks. That means someone else owns them.
    id_to_clock_watcher: HashMap<DeviceId, Ww<dyn WatchesClock>>,
    id_to_audio_source: HashMap<DeviceId, Ww<dyn SourcesAudio>>,
    id_to_effect: HashMap<DeviceId, Ww<dyn IsEffect>>,
    id_to_midi_effect: HashMap<DeviceId, Ww<dyn IsMidiEffect>>,
    id_to_pattern: HashMap<DeviceId, Ww<Pattern>>,
    id_to_control_path: HashMap<DeviceId, Ww<ControlPath>>,
}

impl IdStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn assign_id_if_none(&mut self, id: Option<&str>) -> String {
        match id {
            Some(id) => id.to_string(),
            None => {
                let id = format!("temp-{}", self.next_id);
                self.next_id += 1;
                id
            }
        }
    }

    pub fn add_clock_watcher_by_id(
        &mut self,
        id: Option<&str>,
        clock_watcher: &Rrc<dyn WatchesClock>,
    ) -> String {
        let id = self.assign_id_if_none(id);
        self.id_to_clock_watcher
            .insert(id.to_string(), Rc::downgrade(clock_watcher));
        id
    }

    pub fn add_audio_source_by_id(
        &mut self,
        id: Option<&str>,
        audio_source: &Rrc<dyn SourcesAudio>,
    ) -> String {
        let id = self.assign_id_if_none(id);
        self.id_to_audio_source
            .insert(id.to_string(), Rc::downgrade(audio_source));
        id
    }

    pub fn add_effect_by_id(&mut self, id: Option<&str>, effect: &Rrc<dyn IsEffect>) -> String {
        let id = self.assign_id_if_none(id);
        self.id_to_effect
            .insert(id.to_string(), Rc::downgrade(effect));
        id
    }

    pub fn add_midi_effect_by_id(
        &mut self,
        id: Option<&str>,
        midi_effect: &Rrc<dyn IsMidiEffect>,
    ) -> String {
        let id = self.assign_id_if_none(id);
        self.id_to_midi_effect
            .insert(id.to_string(), Rc::downgrade(&midi_effect));
        id
    }

    pub fn add_control_path_by_id(&mut self, id: Option<&str>, path: &Rrc<ControlPath>) -> String {
        let id = self.assign_id_if_none(id);
        self.id_to_control_path
            .insert(id.to_string(), Rc::downgrade(path));
        id
    }

    pub fn add_pattern_by_id(&mut self, id: Option<&str>, pattern: &Rrc<Pattern>) -> String {
        let id = self.assign_id_if_none(id);
        self.id_to_pattern
            .insert(id.to_string(), Rc::downgrade(pattern));
        id
    }

    pub fn get_audio_source_by_id(&self, id: &str) -> Option<Ww<dyn SourcesAudio>> {
        if let Some(item) = self.id_to_audio_source.get(id) {
            return Some(Weak::clone(item));
        }
        if let Some(item) = self.id_to_effect.get(id) {
            let clone = Weak::clone(&item);
            return Some(clone);
        }
        None
    }

    pub fn get_audio_sink_by_id(&self, id: &str) -> Option<Ww<dyn SinksAudio>> {
        if let Some(item) = self.id_to_effect.get(id) {
            let clone = Weak::clone(&item);
            return Some(clone);
        }
        None
    }

    pub fn get_makes_control_sink_by_id(&self, id: &str) -> Option<Ww<dyn MakesControlSink>> {
        if let Some(item) = self.id_to_effect.get(id) {
            let clone = Weak::clone(item);
            return Some(clone);
        }
        None
    }

    pub fn get_pattern_by_id(&self, id: &str) -> Option<Ww<Pattern>> {
        if let Some(item) = self.id_to_pattern.get(id) {
            return Some(Weak::clone(item));
        }
        None
    }

    pub fn get_control_path_by_id(&self, id: &str) -> Option<Ww<ControlPath>> {
        if let Some(item) = self.id_to_control_path.get(id) {
            return Some(Weak::clone(item));
        }
        None
    }
}
