// Copyright (c) 2023 Mike Tsao. All rights reserved.

use atomic_counter::{AtomicCounter, RelaxedCounter};
use derive_more::Display;
use groove_core::{
    time::{SampleRate, Tempo, TimeSignature},
    traits::{Configurable, ControlEventsFn, Controls, Performs, Serializable, Thing, Ticks},
    Uid,
};
use serde::{Deserialize, Serialize};
use std::{collections::hash_map, fmt::Debug};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

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

type ThingFactoryFn = fn() -> Box<dyn Thing>;

/// [EntityFactory] accepts [Key]s and creates instruments, controllers, and
/// effects. It makes sure every entity has a proper [Uid].
//
// TODO: I'm not sure how Serde will handle EntityFactory's Uids. When it
// deserializes a saved thing, EntityFactory won't know about it, so it seems
// that the next time it creates an entity, the Uid will overwrite it. Do we
// need a special step to refresh the unique Uid trackers?
#[derive(Debug)]
pub struct EntityFactory {
    next_id: RelaxedCounter,

    things: HashMap<Key, ThingFactoryFn>,
    keys: HashSet<Key>,
}
impl Default for EntityFactory {
    fn default() -> Self {
        Self {
            next_id: RelaxedCounter::new(Self::MAX_RESERVED_UID),
            things: Default::default(),
            keys: Default::default(),
        }
    }
}
impl EntityFactory {
    pub(crate) const MAX_RESERVED_UID: usize = 1023;

    /// Registers a new type for the given [Key] using the given closure.
    pub fn register_thing(&mut self, key: Key, f: ThingFactoryFn) {
        if self.keys.insert(key.clone()) {
            self.things.insert(key, f);
        } else {
            panic!("register_thing({}): duplicate key. Exiting.", key);
        }
    }
    /// Creates a new thing of the type corresponding to the given [Key].
    pub fn new_thing(&self, key: &Key) -> Option<Box<dyn Thing>> {
        if let Some(f) = self.things.get(key) {
            let mut r = f();
            r.set_uid(self.mint_uid());
            Some(r)
        } else {
            None
        }
    }

    /// Returns the [HashSet] of all [Key]s.
    pub fn keys(&self) -> &HashSet<Key> {
        &self.keys
    }

    /// Returns the [HashMap] for all [Key] and entity pairs.
    pub fn entities(&self) -> &HashMap<Key, ThingFactoryFn> {
        &self.things
    }

    /// Returns a [Uid] that is guaranteed to be unique among all [Uid]s minted
    /// by this factory. This method is exposed if someone wants to create an
    /// entity outside this factory, but still refer to it by [Uid]. An example
    /// is [super::Transport], which is an entity that [super::MiniOrchestrator]
    /// treats specially.
    pub fn mint_uid(&self) -> Uid {
        Uid(self.next_id.inc())
    }

    /// Naively increments the RelaxedCounter until it is higher than or equal to
    /// the provided [Uid].
    pub fn set_next_uid_expensively(&self, uid: &Uid) {
        self.next_id.reset();
        if self.next_id.get() < uid.0 {
            while self.next_id.inc() <= uid.0 {}
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ThingStore {
    things: HashMap<Uid, Box<dyn Thing>>,
}
impl ThingStore {
    pub fn add(&mut self, thing: Box<dyn Thing>) -> Uid {
        let uid = thing.uid();
        self.things.insert(thing.uid(), thing);
        uid
    }
    #[allow(dead_code)]
    pub fn get(&self, uid: &Uid) -> Option<&Box<dyn Thing>> {
        self.things.get(uid)
    }
    pub fn get_mut(&mut self, uid: &Uid) -> Option<&mut Box<dyn Thing>> {
        self.things.get_mut(uid)
    }
    pub fn remove(&mut self, uid: &Uid) -> Option<Box<dyn Thing>> {
        self.things.remove(uid)
    }
    #[allow(dead_code)]
    pub fn uids(&self) -> hash_map::Keys<'_, Uid, Box<dyn Thing>> {
        self.things.keys()
    }
    pub fn iter(&self) -> hash_map::Values<'_, Uid, Box<dyn Thing>> {
        self.things.values()
    }
    pub fn iter_mut(&mut self) -> hash_map::ValuesMut<'_, Uid, Box<dyn Thing>> {
        self.things.values_mut()
    }
}
impl Ticks for ThingStore {
    fn tick(&mut self, tick_count: usize) {
        self.iter_mut().for_each(|t| {
            if let Some(t) = t.as_instrument_mut() {
                t.tick(tick_count)
            }
        });
    }
}
impl Configurable for ThingStore {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.iter_mut().for_each(|t| {
            t.update_sample_rate(sample_rate);
        });
    }

    fn update_tempo(&mut self, tempo: Tempo) {
        self.iter_mut().for_each(|t| {
            t.update_tempo(tempo);
        });
    }

    fn update_time_signature(&mut self, time_signature: TimeSignature) {
        self.iter_mut().for_each(|t| {
            t.update_time_signature(time_signature);
        });
    }
}
impl Controls for ThingStore {
    fn update_time(&mut self, range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.iter_mut().for_each(|t| {
            if let Some(t) = t.as_controller_mut() {
                t.update_time(range);
            }
        });
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        self.iter_mut().for_each(|t| {
            if let Some(t) = t.as_controller_mut() {
                let tuid = t.uid();
                t.work(&mut |claimed_uid, message| {
                    control_events_fn(tuid, message);
                    if tuid != claimed_uid {
                        eprintln!("Warning: entity {tuid} is sending control messages with incorrect uid {claimed_uid}");
                    }
                });
            }
        });
    }

    fn is_finished(&self) -> bool {
        self.iter().all(|t| {
            if let Some(t) = t.as_controller() {
                t.is_finished()
            } else {
                true
            }
        })
    }
}
impl Performs for ThingStore {
    fn play(&mut self) {
        // TODO: measure whether it's faster to speed through everything and
        // check type than to look up each UID in self.controllers
        self.iter_mut().for_each(|t| {
            if let Some(t) = t.as_controller_mut() {
                t.play();
            }
        });
    }

    fn stop(&mut self) {
        self.iter_mut().for_each(|t| {
            if let Some(t) = t.as_controller_mut() {
                t.stop();
            }
        });
    }

    fn skip_to_start(&mut self) {
        self.iter_mut().for_each(|t| {
            if let Some(t) = t.as_controller_mut() {
                t.skip_to_start();
            }
        });
    }

    fn is_performing(&self) -> bool {
        self.iter().any(|t| {
            if let Some(t) = t.as_controller() {
                t.is_performing()
            } else {
                true
            }
        })
    }
}
impl Serializable for ThingStore {
    fn after_deser(&mut self) {
        self.things.iter_mut().for_each(|(_, t)| t.after_deser());
    }
}

#[cfg(test)]
mod tests {
    use crate::mini::{register_test_factory_entities, EntityFactory, Key};
    use groove_core::Uid;
    use std::collections::HashSet;

    #[test]
    fn entity_creation() {
        let mut factory = EntityFactory::default();
        assert!(factory.entities().is_empty());

        register_test_factory_entities(&mut factory);
        assert!(
            !factory.entities().is_empty(),
            "after registering test entities, factory should contain at least one"
        );

        // After registration, rebind as immutable
        let factory = factory;

        assert!(factory.new_thing(&Key::from(".9-#$%)@#)")).is_none());

        let mut ids: HashSet<Uid> = HashSet::default();
        for key in factory.keys().iter() {
            let e = factory.new_thing(key);
            assert!(e.is_some());
            if let Some(e) = e {
                assert!(!e.name().is_empty());
                assert!(!ids.contains(&e.uid()));
                ids.insert(e.uid());
                assert!(
                    e.as_controller().is_some()
                        || e.as_instrument().is_some()
                        || e.as_effect().is_some(),
                    "Entity '{}' is missing its entity type",
                    key.to_string()
                );
            }
        }
    }
}
