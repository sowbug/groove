// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate provides the [crate::Orchestrator] struct, which coordinates the
//! generation of audio from all the [Entities](entities::Entity) in the
//! system.

pub use entities::{Entity, EntityNano, OtherEntityMessage};
pub use orchestrator::{Orchestrator, Performance};

pub mod helpers;
pub mod messages;

mod entities;
mod orchestrator;
mod util;

#[cfg(feature = "metrics")]
mod metrics;

use groove_core::ParameterType;

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: ParameterType = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;

#[cfg(test)]
mod tests {
    use groove_core::traits::Controllable;
    use groove_core::{control::F32ControlValue, traits::HasUid};
    use groove_proc_macros::{Everything, Nano, Uid};
    use std::{marker::PhantomData, str::FromStr};
    use strum::EnumCount;
    use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

    enum AppMessages {
        Wrapper(usize, OtherEntityMessage),
    }

    // This one has no from/into for F32ControlValue. It's a test for #[nano(control=false)]
    #[derive(Clone, Copy, Debug, Default, EnumCountMacro, FromRepr, PartialEq)]
    pub enum Abnormal {
        #[default]
        Foo,
        Bar,
    }
    impl Abnormal {
        fn next_abnormal(&self) -> Self {
            Abnormal::from_repr((*self as usize + 1) % Abnormal::COUNT).unwrap()
        }
    }
    impl From<F32ControlValue> for Abnormal {
        fn from(value: F32ControlValue) -> Self {
            Abnormal::from_repr((value.0 * Abnormal::COUNT as f32) as usize).unwrap_or_default()
        }
    }

    #[derive(Clone, Copy, Debug, Default, EnumCountMacro, FromRepr, PartialEq)]
    pub enum Cherry {
        #[default]
        Bing,
        Black,
        Cornelian,
        Maraschino,
        QueenAnne,
        Ranier,
        Sour,
        Sweet,
        Van,
        Yellow,
    }
    impl Cherry {
        fn next_cherry(&self) -> Self {
            Cherry::from_repr((*self as usize + 1) % Cherry::COUNT).unwrap()
        }
    }
    impl From<F32ControlValue> for Cherry {
        fn from(value: F32ControlValue) -> Self {
            Cherry::from_repr((value.0 * Cherry::COUNT as f32) as usize).unwrap_or_default()
        }
    }
    impl Into<F32ControlValue> for Cherry {
        fn into(self) -> F32ControlValue {
            F32ControlValue((self as usize as f32) / Cherry::COUNT as f32)
        }
    }

    impl StuffNano {
        fn make_fake() -> Self {
            use rand::Rng;

            let mut rng = rand::thread_rng();
            Self {
                apple_count: rng.gen_range(5..1000),
                banana_quality: rng.gen_range(0.0..1.0),
                cherry: Cherry::from_repr(rng.gen_range(0..Cherry::COUNT)).unwrap(),
                abnormal: Abnormal::from_repr(rng.gen_range(0..Abnormal::COUNT)).unwrap(),
            }
        }

        fn make_different_from(other: &Self) -> Self {
            Self {
                apple_count: other.apple_count() + 1,
                banana_quality: (other.banana_quality() + 0.777).fract(),
                cherry: other.cherry().next_cherry(),
                abnormal: other.abnormal().next_abnormal(),
            }
        }
    }

    #[derive(Debug, Nano, PartialEq, Uid)]
    pub struct Stuff<T> {
        uid: usize,

        #[nano]
        apple_count: usize,
        #[nano]
        banana_quality: f32,
        #[nano]
        cherry: Cherry,
        #[nano(control = false)]
        abnormal: Abnormal,

        _phantom: PhantomData<T>,
    }

    impl<T> Stuff<T> {
        pub fn new(nano: StuffNano) -> Self {
            let mut r = Self {
                uid: Default::default(),
                apple_count: nano.apple_count(),
                banana_quality: nano.banana_quality(),
                cherry: nano.cherry(),
                abnormal: nano.abnormal(),
                _phantom: Default::default(),
            };
            r.precompute();
            r
        }
        pub fn update(&mut self, message: StuffMessage) {
            match message {
                StuffMessage::Stuff(s) => *self = Self::new(s),
                _ => self.derived_update(message),
            }
        }

        fn precompute(&mut self) {
            // This is here as a demo of logic depending on setters/getters
        }

        fn clear_precomputed(&mut self) {
            // This is here as a demo of logic depending on setters/getters
        }

        pub fn apple_count(&self) -> usize {
            self.apple_count
        }

        fn set_apple_count(&mut self, count: usize) {
            self.apple_count = count;
            self.clear_precomputed();
        }

        fn banana_quality(&self) -> f32 {
            self.banana_quality
        }

        fn set_banana_quality(&mut self, banana_quality: f32) {
            self.banana_quality = banana_quality;
            self.clear_precomputed();
        }

        fn cherry(&self) -> Cherry {
            self.cherry
        }

        fn set_cherry(&mut self, cherry: Cherry) {
            self.cherry = cherry;
            self.clear_precomputed();
        }

        pub fn abnormal(&self) -> Abnormal {
            self.abnormal
        }

        pub fn set_abnormal(&mut self, abnormal: Abnormal) {
            self.abnormal = abnormal;
        }
    }

    impl MiscNano {
        fn make_fake() -> Self {
            use rand::Rng;

            let mut rng = rand::thread_rng();
            Self {
                cat_count: rng.gen_range(5..1000),
                dog_count: rng.gen_range(5..1000),
                stuff: StuffNano::make_fake(),
            }
        }
    }

    #[derive(Debug, Nano, Uid)]
    pub struct Misc {
        uid: usize,

        #[nano]
        cat_count: usize,
        #[nano]
        dog_count: usize,

        #[nano(control = false, non_copy = true)]
        stuff: StuffNano,
    }
    impl Misc {
        pub fn new_with(params: MiscNano) -> Self {
            Self {
                uid: Default::default(),
                cat_count: params.cat_count(),
                dog_count: params.dog_count(),
                stuff: params.stuff().clone(),
            }
        }
        pub fn update(&mut self, message: MiscMessage) {
            match message {
                MiscMessage::Misc(s) => *self = Self::new_with(s),
                _ => self.derived_update(message),
            }
        }

        #[allow(dead_code)]
        pub fn cat_count(&self) -> usize {
            self.cat_count
        }

        pub fn set_cat_count(&mut self, cat_count: usize) {
            self.cat_count = cat_count;
        }

        #[allow(dead_code)]
        pub fn dog_count(&self) -> usize {
            self.dog_count
        }

        pub fn set_dog_count(&mut self, dog_count: usize) {
            self.dog_count = dog_count;
        }

        #[allow(dead_code)]
        pub fn stuff(&self) -> &StuffNano {
            &self.stuff
        }

        pub fn set_stuff(&mut self, stuff: StuffNano) {
            self.stuff = stuff;
        }
    }

    #[allow(dead_code)]
    type MsgType = OtherEntityMessage;
    #[allow(dead_code)]
    #[derive(Everything)]
    enum Models {
        Stuff(Stuff<OtherEntityMessage>),
        Misc(Misc),
    }

    #[test]
    fn update_full() {
        let a = StuffNano::make_fake();
        let mut b = StuffNano::make_different_from(&a);
        assert_ne!(a, b);
        b.update(StuffMessage::Stuff(a.clone()));
        assert_eq!(a, b);
    }

    #[test]
    fn update_incrementally() {
        let mut a = StuffNano::make_fake();
        let mut b = StuffNano::make_different_from(&a);
        assert_ne!(a, b);

        let message = StuffMessage::AppleCount(a.apple_count() + 1);
        a.update(message.clone());
        b.update(message);
        assert_ne!(a, b);

        let message = StuffMessage::BananaQuality(b.banana_quality() / 3.0);
        a.update(message.clone());
        b.update(message);
        assert_ne!(a, b);

        let message = StuffMessage::Cherry(a.cherry().next_cherry());
        a.update(message.clone());
        b.update(message);
        assert_ne!(a, b);

        let message = StuffMessage::Abnormal(a.abnormal().next_abnormal());
        a.update(message.clone());
        b.update(message);

        assert_eq!(a, b);
    }

    fn painful_equality_test(a: &Entity, b: &Entity) -> bool {
        match a {
            Entity::Stuff(a) => match b {
                Entity::Stuff(b) => return a == b,
                Entity::Misc(_) => todo!(),
            },
            Entity::Misc(_) => todo!(),
        }
    }

    #[test]
    fn update_incrementally_with_entity_wrappers() {
        let a_params = StuffNano::make_fake();
        let b_params = StuffNano::make_different_from(&a_params);
        let a = Stuff::<OtherEntityMessage>::new(a_params);
        let b = Stuff::<OtherEntityMessage>::new(b_params);
        assert_ne!(a, b);

        // Do these before the boxes take them away
        let next_apple_count = a.apple_count() + 1;
        let next_banana_quality = b.banana_quality() / 3.0;
        let next_cherry = a.cherry().next_cherry();
        let next_abnormal = a.abnormal().next_abnormal();

        let mut ea = Entity::Stuff(Box::new(a));
        let mut eb = Entity::Stuff(Box::new(b));

        let message = OtherEntityMessage::Stuff(StuffMessage::AppleCount(next_apple_count));
        ea.update(message.clone());
        eb.update(message);
        assert!(!painful_equality_test(&ea, &eb));

        let message = OtherEntityMessage::Stuff(StuffMessage::BananaQuality(next_banana_quality));
        ea.update(message.clone());
        eb.update(message);
        assert!(!painful_equality_test(&ea, &eb));

        let message = OtherEntityMessage::Stuff(StuffMessage::Cherry(next_cherry));
        ea.update(message.clone());
        eb.update(message);
        assert!(!painful_equality_test(&ea, &eb));

        let message = OtherEntityMessage::Stuff(StuffMessage::Abnormal(next_abnormal));
        ea.update(message.clone());
        eb.update(message);

        assert!(painful_equality_test(&ea, &eb));
    }

    #[test]
    fn control_params_by_name() {
        let a_params = StuffNano::make_fake();
        let b_params = StuffNano::make_different_from(&a_params);
        let a = Stuff::<OtherEntityMessage>::new(a_params);
        let mut b = Stuff::<OtherEntityMessage>::new(b_params);
        assert_ne!(a, b);

        // We're going to cheat and manually set a/b Abnormal to be the same.
        b.set_abnormal(a.abnormal());

        if let Some(message) = b.message_for_name("apple-count", a.apple_count().into()) {
            b.update(message);
        }
        assert_ne!(a, b);
        if let Some(message) = b.message_for_name("banana-quality", a.banana_quality().into()) {
            b.update(message);
        }
        assert_ne!(a, b);
        if let Some(message) = b.message_for_name("cherry", a.cherry().into()) {
            b.update(message);
        }
        assert_eq!(a, b);
    }

    #[test]
    fn control_params_by_index() {
        let a_params = StuffNano::make_fake();
        let b_params = StuffNano::make_different_from(&a_params);
        let a = Stuff::<OtherEntityMessage>::new(a_params);
        let mut b = Stuff::<OtherEntityMessage>::new(b_params);
        assert_ne!(a, b);

        // We exclude the full message from the index.
        assert_eq!(a.control_index_count(), 3);

        if let Some(message) = b.message_for_index(0, a.apple_count().into()) {
            b.update(message);
        }
        assert_ne!(a, b);
        if let Some(message) = b.message_for_index(1, a.banana_quality().into()) {
            b.update(message);
        }
        assert_ne!(a, b);
        if let Some(message) = b.message_for_index(2, a.cherry().into()) {
            b.update(message);
        }
        assert_ne!(a, b);

        // This one is odd, because we can't ask the system to make the message
        // for us (since the point of the Abnormal type is that there is no
        // <F32ControlValue>::into(abnormal)). So we have to do it manually.
        let message = StuffMessage::Abnormal(a.abnormal());
        b.update(message);

        assert_eq!(a, b);
    }

    #[test]
    fn control_ergonomics() {
        let a = Stuff::<OtherEntityMessage>::new(StuffNano::make_fake());

        assert_eq!(a.control_name_for_index(2), Some("cherry"));
        assert_eq!(a.control_index_count(), 3);
        assert_eq!(a.control_name_for_index(a.control_index_count()), None);

        let a = MiscNano::make_fake();

        assert_eq!(a.control_name_for_index(0), Some("cat-count"));
        assert_eq!(a.control_index_count(), 2);
        assert_eq!(a.control_name_for_index(a.control_index_count()), None);
    }

    #[test]
    fn core_struct_gets_notifications() {
        // This test used to do something intricate with the precompute logic in
        // Stuff. It got more complicated than necessary for this small test
        // suite. This is a memorial of that idea.
    }

    #[test]
    fn build_views() {
        let entities = vec![
            EntityNano::Stuff(Box::new(StuffNano::make_fake())),
            EntityNano::Misc(Box::new(MiscNano::make_fake())),
            EntityNano::Misc(Box::new(MiscNano::make_fake())),
        ];

        // Build custom views from entity getters
        for entity in entities.iter() {
            match entity {
                EntityNano::Stuff(_e) => {}
                EntityNano::Misc(_e) => {}
            }
        }

        // Build an automation matrix
        for _ in entities.iter().filter(|e| e.is_controller()) {
            // if entity implements controller trait, add it to sources
            eprintln!("adding controller");
        }
        for entity in entities.iter().filter(|e| e.is_controllable()) {
            eprintln!("adding controllable");
            let controllable = entity.as_controllable().unwrap();
            for index in 0..controllable.control_index_count() {
                if let Some(point_name) = controllable.control_name_for_index(index) {
                    eprintln!("adding control point {}", point_name);
                } else {
                    eprintln!("couldn't find name for control point #{}", index);
                }
            }
        }
    }

    #[test]
    fn handle_app_updates() {
        let mut entities = vec![
            EntityNano::Stuff(Box::new(StuffNano::make_fake())),
            EntityNano::Misc(Box::new(MiscNano::make_fake())),
            EntityNano::Misc(Box::new(MiscNano::make_fake())),
        ];

        // Connect two things
        // send message: connect(source, dest, index)
        // send message: disconnect(source, dest, index)

        // Handle an incoming message
        let message = StuffMessage::AppleCount(45);
        let wrapped_message = AppMessages::Wrapper(1, OtherEntityMessage::Stuff(message));

        let AppMessages::Wrapper(uid, message) = wrapped_message;
        let entity = &mut entities[uid];
        match message {
            OtherEntityMessage::Stuff(message) => {
                if let EntityNano::Stuff(entity) = entity {
                    entity.update(message);
                }
            }
            OtherEntityMessage::Misc(message) => {
                if let EntityNano::Misc(entity) = entity {
                    entity.update(message);
                }
            }
        }
    }

    #[test]
    fn engine_usage() {
        {
            // This is here just to compare generic and non-generic structs.
            let _misc = Misc::new_with(MiscNano::make_fake());
            let _misc_entity = Entity::Misc(Box::new(_misc));
        }
        let a = Stuff::<OtherEntityMessage>::new(StuffNano::make_fake());
        let next_cherry = a.cherry().next_cherry();
        let mut ea = Entity::Stuff(Box::new(a));

        if let Some(message) = ea.message_for(0, 50.0.into()) {
            ea.update(message);
        }
        if let Some(message) = ea.message_for(1, 0.14159265.into()) {
            ea.update(message);
        }
        if let Some(message) = ea.message_for(2, next_cherry.into()) {
            ea.update(message);
        }

        if let Entity::Stuff(a) = ea {
            assert_eq!(a.apple_count(), 50);
            assert_eq!(a.banana_quality(), 0.14159265);
            assert_eq!(a.cherry(), next_cherry);
        }
    }

    #[test]
    fn control_false() {
        let a = Stuff::<OtherEntityMessage>::new(StuffNano::make_fake());

        assert_eq!(a.control_index_count(), 3); // apple/banana/cherry but not abnormal
        assert_eq!(a.control_index_for_name("abnormal"), usize::MAX); // apple/banana/cherry but not abnormal
        let message = StuffMessage::Abnormal(Abnormal::Foo); // Should still be able to instantiate this

        let mut ea = Entity::Stuff(Box::new(a));
        ea.update(OtherEntityMessage::Stuff(message)); // Should be able to handle this

        let _full_message = ea.full_message(); // This shouldn't change

        assert!(ea.message_for(4, 1.0.into()).is_none()); // But this is meaningless
    }
}
