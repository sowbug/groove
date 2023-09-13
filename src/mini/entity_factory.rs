// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::anyhow;
use atomic_counter::{AtomicCounter, RelaxedCounter};
use derive_more::Display;
use groove_core::{
    time::{SampleRate, Tempo, TimeSignature},
    traits::{Configurable, ControlEventsFn, Controls, Entity, Serializable, Ticks},
    Uid,
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{collections::hash_map, fmt::Debug, option::Option};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

/// A globally unique identifier for a kind of entity, such as an arpeggiator
/// controller, an FM synthesizer, or a reverb effect.
#[derive(Clone, Debug, Display, Eq, Hash, PartialEq, PartialOrd, Ord)]
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

type EntityFactoryFn = fn() -> Box<dyn Entity>;

/// The one and only EntityFactory. Access it with `EntityFactory::global()`.
static FACTORY: OnceCell<EntityFactory> = OnceCell::new();

/// [EntityFactory] accepts [Key]s and creates instruments, controllers, and
/// effects. It makes sure every entity has a proper [Uid].
#[derive(Debug)]
pub struct EntityFactory {
    next_uid: RelaxedCounter,
    entities: HashMap<Key, EntityFactoryFn>,
    keys: HashSet<Key>,

    is_registration_complete: bool,
    sorted_keys: Vec<Key>,
}
impl Default for EntityFactory {
    fn default() -> Self {
        Self {
            next_uid: RelaxedCounter::new(Self::MAX_RESERVED_UID + 1),
            entities: Default::default(),
            keys: Default::default(),
            is_registration_complete: Default::default(),
            sorted_keys: Default::default(),
        }
    }
}
impl EntityFactory {
    /// Specifies the range of [Uid]s that [EntityFactory] will never issue. 
    pub const MAX_RESERVED_UID: usize = 1023;

    /// Provides the one and only [EntityFactory].
    pub fn global() -> &'static Self {
        FACTORY
            .get()
            .expect("EntityFactory has not been initialized")
    }

    /// Set the next [Uid]. This is needed if we're deserializing a project and
    /// need to reset the [EntityFactory] to mint unique [Uid]s.
    ///
    /// Note that the specified [Uid] is not necessarily the next one that will
    /// be issued; we guarantee only that subsequent [Uid]s won't be lower than
    /// it. This is because we're using [RelaxedCounter] under the hood to allow
    /// entirely immutable usage of this factory after creation and
    /// configuration.
    pub fn set_next_uid(&self, next_uid_value: usize) {
        self.next_uid.reset();
        self.next_uid
            .add(next_uid_value.max(Self::MAX_RESERVED_UID + 1));
    }

    /// Registers a new type for the given [Key] using the given closure.
    pub fn register_entity(&mut self, key: Key, f: EntityFactoryFn) {
        if self.is_registration_complete {
            panic!("attempt to register an entity after registration completed");
        }
        if self.keys.insert(key.clone()) {
            self.entities.insert(key, f);
        } else {
            panic!("register_entity({}): duplicate key. Exiting.", key);
        }
    }

    /// Tells the factory that we won't be registering any more entities,
    /// allowing it to do some final housekeeping.
    pub fn complete_registration(&mut self) {
        self.is_registration_complete = true;
        self.sorted_keys = self.keys().iter().cloned().collect();
        self.sorted_keys.sort();
    }

    /// Creates a new entity of the type corresponding to the given [Key].
    pub fn new_entity(&self, key: &Key) -> Option<Box<dyn Entity>> {
        if let Some(f) = self.entities.get(key) {
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
    pub fn entities(&self) -> &HashMap<Key, EntityFactoryFn> {
        &self.entities
    }

    /// Returns a [Uid] that is guaranteed to be unique among all [Uid]s minted
    /// by this factory. This method is exposed if someone wants to create an
    /// entity outside this factory, but still refer to it by [Uid]. An example
    /// is [super::Transport], which is an entity that [super::Orchestrator]
    /// treats specially.
    pub fn mint_uid(&self) -> Uid {
        Uid(self.next_uid.inc())
    }

    /// Returns all the [Key]s in sorted order for consistent display in the UI.
    pub fn sorted_keys(&self) -> &[Key] {
        if !self.is_registration_complete {
            panic!("sorted_keys() can be called only after registration is complete.")
        }
        &self.sorted_keys
    }

    /// Sets the singleton [EntityFactory].
    pub fn initialize(entity_factory: Self) -> Result<(), Self> {
        FACTORY.set(entity_factory)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct EntityStore {
    #[serde(skip)]
    sample_rate: SampleRate,
    entities: HashMap<Uid, Box<dyn Entity>>,
}
impl EntityStore {
    pub fn add(&mut self, mut entity: Box<dyn Entity>) -> anyhow::Result<Uid> {
        let uid = entity.uid();
        if self.entities.contains_key(&uid) {
            return Err(anyhow!("Entity Uid {uid} already exists"));
        }
        entity.update_sample_rate(self.sample_rate);
        self.entities.insert(entity.uid(), entity);
        Ok(uid)
    }
    pub fn get(&self, uid: &Uid) -> Option<&Box<dyn Entity>> {
        self.entities.get(uid)
    }
    pub fn get_mut(&mut self, uid: &Uid) -> Option<&mut Box<dyn Entity>> {
        self.entities.get_mut(uid)
    }
    pub fn remove(&mut self, uid: &Uid) -> Option<Box<dyn Entity>> {
        self.entities.remove(uid)
    }
    #[allow(dead_code)]
    pub fn uids(&self) -> hash_map::Keys<'_, Uid, Box<dyn Entity>> {
        self.entities.keys()
    }
    pub fn iter(&self) -> hash_map::Values<'_, Uid, Box<dyn Entity>> {
        self.entities.values()
    }
    pub fn iter_mut(&mut self) -> hash_map::ValuesMut<'_, Uid, Box<dyn Entity>> {
        self.entities.values_mut()
    }
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub(crate) fn calculate_max_entity_uid(&self) -> Option<Uid> {
        // TODO: keep an eye on this in case it gets expensive. It's currently
        // used only after loading from disk, and it's O(number of things in
        // system), so it's unlikely to matter.
        self.entities.keys().max().copied()
    }
}
impl Ticks for EntityStore {
    fn tick(&mut self, tick_count: usize) {
        self.iter_mut().for_each(|t| {
            if let Some(t) = t.as_instrument_mut() {
                t.tick(tick_count)
            }
        });
    }
}
impl Configurable for EntityStore {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
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
impl Controls for EntityStore {
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
impl Serializable for EntityStore {
    fn after_deser(&mut self) {
        self.entities.iter_mut().for_each(|(_, t)| t.after_deser());
    }
}

#[cfg(test)]
mod tests {
    use super::EntityStore;
    use crate::mini::{register_test_factory_entities, EntityFactory, Key};
    use groove_core::{
        time::SampleRate,
        traits::{Configurable, HasUid},
        Uid,
    };
    use groove_toys::ToySynth;
    use std::collections::HashSet;

    #[test]
    fn entity_creation() {
        assert!(
            EntityFactory::default().entities().is_empty(),
            "A new EntityFactory should be empty"
        );

        let factory = register_test_factory_entities(EntityFactory::default());
        assert!(
            !factory.entities().is_empty(),
            "after registering test entities, factory should contain at least one"
        );

        // After registration, rebind as immutable
        let factory = factory;

        assert!(factory.new_entity(&Key::from(".9-#$%)@#)")).is_none());

        let mut ids: HashSet<Uid> = HashSet::default();
        for key in factory.keys().iter() {
            let e = factory.new_entity(key);
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
                    key
                );
            }
        }
    }

    #[test]
    fn entity_factory_uid_uniqueness() {
        let ef = EntityFactory::default();
        let uid = ef.mint_uid();

        let ef = EntityFactory::default();
        let uid2 = ef.mint_uid();

        assert_eq!(uid, uid2);

        let ef = EntityFactory::default();
        ef.set_next_uid(uid.0 + 1);
        let uid2 = ef.mint_uid();
        assert_ne!(uid, uid2);
    }

    #[test]
    fn store_is_responsible_for_sample_rate() {
        let mut t = EntityStore::default();
        assert_eq!(t.sample_rate, SampleRate::DEFAULT);
        t.update_sample_rate(SampleRate(44444));
        let factory = register_test_factory_entities(EntityFactory::default());

        let entity = factory.new_entity(&Key::from("instrument")).unwrap();
        assert_eq!(
            entity.sample_rate(),
            SampleRate::DEFAULT,
            "before adding to store, sample rate should be untouched"
        );

        let uid = t.add(entity).unwrap();
        let entity = t.remove(&uid).unwrap();
        assert_eq!(
            entity.sample_rate(),
            SampleRate(44444),
            "after adding/removing to/from store, sample rate should match"
        );
    }

    #[test]
    fn disallow_duplicate_uids() {
        let mut t = EntityStore::default();
        assert_eq!(t.calculate_max_entity_uid(), None);

        let mut one = Box::new(ToySynth::default());
        one.set_uid(Uid(9999));
        assert!(t.add(one).is_ok(), "adding a unique UID should succeed");
        assert_eq!(t.calculate_max_entity_uid(), Some(Uid(9999)));

        let mut two = Box::new(ToySynth::default());
        two.set_uid(Uid(9999));
        assert!(t.add(two).is_err(), "adding a duplicate UID should fail");

        let max_uid = t.calculate_max_entity_uid().unwrap();
        // Though the add() was sure to fail, it's still considered mutably
        // borrowed at compile time.
        let mut two = Box::new(ToySynth::default());
        two.set_uid(Uid(max_uid.0 + 1));
        assert!(
            t.add(two).is_ok(),
            "using Orchestrator's max_entity_uid as a guide should work."
        );
        assert_eq!(t.calculate_max_entity_uid(), Some(Uid(10000)));
    }
}
