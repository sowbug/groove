// Copyright (c) 2023 Mike Tsao. All rights reserved.

use atomic_counter::{AtomicCounter, RelaxedCounter};
use derive_more::Display;
use groove_core::{
    traits::{IsController, IsEffect, IsInstrument},
    Uid,
};
use groove_entities::EntityMessage;

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};
use strum_macros::EnumIter;

#[derive(Debug, EnumIter)]
pub enum EntityType {
    None,
    Controller,
    Effect,
    Instrument,
}

/// A globally unique identifier for a kind of thing, such as an arpeggiator
/// controller, an FM synthesizer, or a reverb effect.
#[derive(Clone, Debug, Display, Eq, Hash, PartialEq)]
pub struct Key(String);
impl From<&String> for Key {
    fn from(value: &String) -> Self {
        Key(value.to_string())
    }
}
impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key(value.to_string())
    }
}

#[typetag::serde(tag = "type")]
pub trait NewIsController: IsController<Message = EntityMessage> {}

#[typetag::serde(tag = "type")]
pub trait NewIsInstrument: IsInstrument {}

#[typetag::serde(tag = "type")]
pub trait NewIsEffect: IsEffect {}

type ControllerEntityFactoryFn = fn() -> Box<dyn NewIsController>;
type InstrumentEntityFactoryFn = fn() -> Box<dyn NewIsInstrument>;
type EffectEntityFactoryFn = fn() -> Box<dyn NewIsEffect>;

/// [EntityFactory] creates instruments, controllers, and effects when given a
/// [Key]. It makes sure every entity has a proper [Uid].
#[derive(Debug, Default)]
pub struct EntityFactory {
    next_id: RelaxedCounter,

    controllers: HashMap<Key, ControllerEntityFactoryFn>,
    instruments: HashMap<Key, InstrumentEntityFactoryFn>,
    effects: HashMap<Key, EffectEntityFactoryFn>,
    keys: HashSet<Key>,
}
impl EntityFactory {
    /// Registers a new controller type for the given [Key] using the given closure.
    pub fn register_controller(&mut self, key: Key, f: ControllerEntityFactoryFn) {
        if self.keys.insert(key.clone()) {
            self.controllers.insert(key, f);
        } else {
            panic!("register_controller({}): duplicate key. Exiting.", key);
        }
    }
    /// Creates a new controller of the type corresponding to the given [Key].
    pub fn new_controller(&self, key: &Key) -> Option<Box<dyn NewIsController>> {
        if let Some(f) = self.controllers.get(key) {
            let mut r = f();
            r.set_uid(Uid(self.next_id.inc()));
            Some(r)
        } else {
            None
        }
    }
    /// Registers a new instrument type for the given [Key] using the given closure.
    pub fn register_instrument(&mut self, key: Key, f: InstrumentEntityFactoryFn) {
        if self.keys.insert(key.clone()) {
            self.instruments.insert(key, f);
        } else {
            panic!("register_instrument({}): duplicate key. Exiting.", key);
        }
    }
    /// Creates a new instrument of the type corresponding to the given [Key].
    pub fn new_instrument(&self, key: &Key) -> Option<Box<dyn NewIsInstrument>> {
        if let Some(f) = self.instruments.get(key) {
            let mut r = f();
            r.set_uid(Uid(self.next_id.inc()));
            Some(r)
        } else {
            None
        }
    }
    /// Registers a new effect type for the given [Key] using the given closure.
    pub fn register_effect(&mut self, key: Key, f: EffectEntityFactoryFn) {
        if self.keys.insert(key.clone()) {
            self.effects.insert(key, f);
        } else {
            panic!("register_effect({}): duplicate key. Exiting.", key);
        }
    }
    /// Creates a new effect of the type corresponding to the given [Key].
    pub fn new_effect(&self, key: &Key) -> Option<Box<dyn NewIsEffect>> {
        if let Some(f) = self.effects.get(key) {
            let mut r = f();
            r.set_uid(Uid(self.next_id.inc()));
            Some(r)
        } else {
            None
        }
    }

    /// Returns an iterator for all the [Key]s for registered controllers.
    pub fn controller_keys(
        &self,
    ) -> std::collections::hash_map::Keys<Key, fn() -> Box<dyn NewIsController>> {
        self.controllers.keys()
    }

    /// Returns an iterator for all the [Key]s for registered instruments.
    pub fn instrument_keys(
        &self,
    ) -> std::collections::hash_map::Keys<Key, fn() -> Box<dyn NewIsInstrument>> {
        self.instruments.keys()
    }

    /// Returns an iterator for all the [Key]s for registered effects.
    pub fn effect_keys(
        &self,
    ) -> std::collections::hash_map::Keys<Key, fn() -> Box<dyn NewIsEffect>> {
        self.effects.keys()
    }

    /// Returns the [HashSet] of all [Key]s.
    pub fn keys(&self) -> &HashSet<Key> {
        &self.keys
    }

    /// Returns the [HashMap] for all [Key] and controller pairs.
    pub fn controllers(&self) -> &HashMap<Key, ControllerEntityFactoryFn> {
        &self.controllers
    }

    /// Returns the [HashMap] for all [Key] and instrument pairs.
    pub fn instruments(&self) -> &HashMap<Key, InstrumentEntityFactoryFn> {
        &self.instruments
    }

    /// Returns the [HashMap] for all [Key] and effect pairs.
    pub fn effects(&self) -> &HashMap<Key, EffectEntityFactoryFn> {
        &self.effects
    }
}
