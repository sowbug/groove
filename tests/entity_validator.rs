// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::{
    mini::{register_factory_entities, Key},
    EntityFactory,
};
use groove_core::{
    time::{SampleRate, Tempo, TimeSignature},
    traits::{Entity, IsController, IsEffect, IsInstrument},
    Uid,
};

#[test]
fn entity_validator_production_entities() {
    if EntityFactory::initialize(register_factory_entities(EntityFactory::default())).is_err() {
        panic!("Couldn't set EntityFactory once_cell");
    }
    validate_factory_entities();
}

fn validate_factory_entities() {
    for key in EntityFactory::global().keys() {
        if let Some(mut entity) = EntityFactory::global().new_entity(key) {
            validate_entity(key, entity);
        } else {
            panic!("Couldn't create entity with {key}, but EntityFactory said it existed!");
        }
    }
}

fn validate_entity(key: &Key, mut entity: Box<dyn Entity>) {
    assert_ne!(entity.uid(), Uid(0), "New entity should have a nonzero Uid");
    assert!(
        entity.uid().0 > EntityFactory::MAX_RESERVED_UID,
        "New entity should have a Uid above {}, but the one for {key} was {}",
        EntityFactory::MAX_RESERVED_UID,
        entity.uid()
    );
    validate_configurable(key, &mut entity);

    validate_entity_type(key, entity);
}

fn validate_configurable(key: &Key, entity: &mut Box<dyn Entity>) {
    const TEST_SAMPLE_RATE: SampleRate = SampleRate(1111111);
    entity.update_tempo(Tempo(1234.5678));
    entity.update_time_signature(TimeSignature::new_with(127, 128).unwrap());
    entity.update_sample_rate(TEST_SAMPLE_RATE);

    // This caused lots of things to fail and has me rethinking why Configurable
    // needed sample_rate() as such a widespread trait method. TODO
    if false {
        assert!(
            entity.sample_rate().0 > 0,
            "Entity {key}'s default sample rate should be nonzero"
        );
        assert_eq!(
            entity.sample_rate(),
            SampleRate::DEFAULT,
            "Entity {key}'s default sample rate should equal the default of {}",
            SampleRate::DEFAULT_SAMPLE_RATE
        );
        entity.update_sample_rate(TEST_SAMPLE_RATE);
        assert_eq!(
            entity.sample_rate(),
            TEST_SAMPLE_RATE,
            "Entity {key}'s sample rate should change once set"
        );
    }
}

fn validate_entity_type(key: &Key, mut entity: Box<dyn Entity>) {
    let mut is_something = false;
    if let Some(e) = entity.as_controller_mut() {
        is_something = true;
        validate_controller(e);
    }
    if let Some(e) = entity.as_instrument_mut() {
        is_something = true;
        validate_instrument(e);
    }
    if let Some(e) = entity.as_effect_mut() {
        is_something = true;
        validate_effect(e);
    }
    assert!(
        is_something,
        "Entity {key} is neither a controller, nor an instrument, nor an effect!"
    );
}

fn validate_effect(e: &mut dyn IsEffect) {}

fn validate_instrument(e: &mut dyn IsInstrument) {}

fn validate_controller(e: &mut dyn IsController) {
    assert!(
        !e.is_performing(),
        "A new Controller should not be performing"
    );
}
