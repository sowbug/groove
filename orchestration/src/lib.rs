// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate provides the [crate::Orchestrator] struct, which coordinates the
//! generation of audio from all the [Entities](entities::Entity) in the
//! system.

//pub use entities::EntityObsolete;
//pub use orchestrator::Performance;

pub mod helpers;
pub mod messages;

mod entities;
#[cfg(obsolete)]
mod orchestrator;
mod util;

#[cfg(feature = "metrics")]
mod metrics;

#[cfg(test)]
mod tests {
    mod params {
        use ensnare::{prelude::*, traits::prelude::*};
        use ensnare_proc_macros::{Control, Params, Uid};
        use strum::EnumCount;
        use strum_macros::{EnumCount as EnumCountMacro, FromRepr};

        #[cfg(feature = "serialization")]
        use serde::{Deserialize, Serialize};

        #[derive(Clone, Copy, Debug, Default, EnumCountMacro, FromRepr, PartialEq)]
        #[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
        impl From<ControlValue> for Abnormal {
            fn from(value: ControlValue) -> Self {
                Abnormal::from_repr((value.0 * Abnormal::COUNT as f64) as usize).unwrap_or_default()
            }
        }

        #[derive(Clone, Copy, Debug, Default, EnumCountMacro, FromRepr, PartialEq)]
        #[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
            // TODO: this is a hack that needs to be automated in the Control derive macro.
            // As a workaround, structs that incorporate enums can declare them #[params(leaf=true)]
            #[allow(dead_code)]
            pub const STRUCT_SIZE: usize = 1;

            fn next_cherry(&self) -> Self {
                Cherry::from_repr((*self as usize + 1) % Cherry::COUNT).unwrap()
            }
        }
        impl From<ControlValue> for Cherry {
            fn from(value: ControlValue) -> Self {
                Cherry::from_repr((value.0 * Cherry::COUNT as f64) as usize).unwrap_or_default()
            }
        }
        impl Into<ControlValue> for Cherry {
            fn into(self) -> ControlValue {
                ControlValue((self as usize as f64) / Cherry::COUNT as f64)
            }
        }

        impl StuffParams {
            fn make_fake() -> Self {
                let mut rng = oorandom::Rand32::new(0);
                Self {
                    apple_count: rng.rand_range(5..1000) as usize,
                    banana_quality: rng.rand_float(),
                    cherry: Cherry::from_repr(rng.rand_range(0..Cherry::COUNT as u32) as usize)
                        .unwrap(),
                    abnormal: Abnormal::from_repr(
                        rng.rand_range(0..Abnormal::COUNT as u32) as usize
                    )
                    .unwrap(),
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

        #[derive(Control, Debug, Params, PartialEq, Uid)]
        pub struct Stuff {
            uid: Uid,

            #[params]
            #[control]
            apple_count: usize,

            #[params]
            #[control]
            banana_quality: f32,

            #[params(leaf = true)]
            #[control(leaf = true)]
            cherry: Cherry,

            #[params(leaf = true)]
            abnormal: Abnormal,
        }
        impl Configurable for Stuff {}

        impl Stuff {
            pub fn new(params: StuffParams) -> Self {
                let mut r = Self {
                    uid: Default::default(),
                    apple_count: params.apple_count(),
                    banana_quality: params.banana_quality(),
                    cherry: params.cherry(),
                    abnormal: params.abnormal(),
                };
                r.precompute();
                r
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

            #[allow(dead_code)]
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

        impl MiscParams {
            fn make_fake() -> Self {
                let mut rng = oorandom::Rand32::new(0);
                Self {
                    cat_count: rng.rand_range(5..1000) as usize,
                    dog_count: rng.rand_range(5..1000) as usize,
                    stuff: StuffParams::make_fake(),
                }
            }
        }

        #[derive(Control, Debug, Params, Uid)]
        pub struct Misc {
            uid: Uid,

            #[params]
            #[control]
            cat_count: usize,
            #[params]
            #[control]
            dog_count: usize,

            #[params]
            #[control]
            stuff: Stuff,
        }
        impl Configurable for Misc {}
        impl Misc {
            pub fn new(params: MiscParams) -> Self {
                Self {
                    uid: Default::default(),
                    cat_count: params.cat_count(),
                    dog_count: params.dog_count(),
                    stuff: Stuff::new(params.stuff),
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
            pub fn stuff(&self) -> &Stuff {
                &self.stuff
            }

            #[allow(dead_code)]
            pub fn set_stuff(&mut self, stuff: Stuff) {
                self.stuff = stuff;
            }
        }

        #[test]
        fn control_params_by_name() {
            let a_params = StuffParams::make_fake();
            let b_params = StuffParams::make_different_from(&a_params);
            assert_ne!(a_params, b_params);
            let a: Stuff = Stuff::new(a_params);
            let mut b: Stuff = Stuff::new(b_params);
            assert_ne!(a, b);

            // We're going to cheat and manually set a/b Abnormal to be the same.
            b.set_abnormal(a.abnormal());

            b.control_set_param_by_name("apple-count", a.apple_count().into());
            assert_ne!(a, b);
            b.control_set_param_by_name("banana-quality", a.banana_quality().into());
            assert_ne!(a, b);
            b.control_set_param_by_name("cherry", a.cherry().into());
            assert_eq!(a, b);
        }

        #[test]
        fn control_params_by_index() {
            let a_params = StuffParams::make_fake();
            let b_params = StuffParams::make_different_from(&a_params);
            assert_ne!(a_params, b_params);
            let a: Stuff = Stuff::new(a_params);
            let mut b: Stuff = Stuff::new(b_params);
            assert_ne!(a, b);

            assert_eq!(a.control_index_count(), 3);

            b.control_set_param_by_index(ControlIndex(0), a.apple_count().into());
            assert_ne!(a, b);
            b.control_set_param_by_index(ControlIndex(1), a.banana_quality().into());
            assert_ne!(a, b);
            b.control_set_param_by_index(ControlIndex(2), a.cherry().into());
            assert_ne!(a, b);

            b.set_abnormal(a.abnormal());

            assert_eq!(a, b);
        }

        #[test]
        fn control_ergonomics() {
            let a: Stuff = Stuff::new(StuffParams::make_fake());

            assert_eq!(
                a.control_name_for_index(ControlIndex(2)),
                Some("cherry".to_string())
            );
            assert_eq!(a.control_index_count(), 3);
            assert_eq!(
                a.control_name_for_index(ControlIndex(a.control_index_count())),
                None
            );

            let a = Misc::new(MiscParams::make_fake());

            assert_eq!(
                a.control_name_for_index(ControlIndex(0)),
                Some("cat-count".to_string())
            );
            assert_eq!(a.control_index_count(), 2 + 3);
            assert_eq!(
                a.control_name_for_index(ControlIndex(a.control_index_count())),
                None
            );
        }
    }
}
