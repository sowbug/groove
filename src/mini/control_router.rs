// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    control::F32ControlValue,
    traits::{ControlIndex, ControlValue},
    Uid,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::entity_factory::ThingStore;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ControlRouter {
    uid_to_control: HashMap<Uid, Vec<(Uid, ControlIndex)>>,
}
#[allow(dead_code)]
impl ControlRouter {
    pub fn link_control(&mut self, source_uid: Uid, target_uid: Uid, control_index: ControlIndex) {
        self.uid_to_control
            .entry(source_uid)
            .or_default()
            .push((target_uid, control_index));
    }

    pub fn unlink_control(
        &mut self,
        source_uid: Uid,
        target_uid: Uid,
        control_index: ControlIndex,
    ) {
        self.uid_to_control
            .entry(source_uid)
            .or_default()
            .retain(|(uid, index)| !(*uid == target_uid && *index == control_index));
    }

    pub fn control_links(&self, source_uid: Uid) -> Option<&Vec<(Uid, ControlIndex)>> {
        self.uid_to_control.get(&source_uid)
    }

    pub fn route(&mut self, entity_store: &mut ThingStore, source_uid: Uid, value: ControlValue) {
        if let Some(control_links) = self.control_links(source_uid) {
            control_links.iter().for_each(|(target_uid, index)| {
                if let Some(e) = entity_store.get_mut(target_uid) {
                    // TODO: I got lazy here because I don't have an
                    // as_controllable_mut() yet. If/when we set up the macro to
                    // generate these easily, then extend to add that.
                    if let Some(e) = e.as_instrument_mut() {
                        e.control_set_param_by_index(index.0, F32ControlValue(value.0 as f32));
                    } else if let Some(e) = e.as_effect_mut() {
                        e.control_set_param_by_index(index.0, F32ControlValue(value.0 as f32));
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini::entity_factory::{Thing, ThingType};
    use groove_core::{
        traits::{
            gui::Shows, Configurable, Controllable, Generates, HandlesMidi, IsInstrument, Ticks,
        },
        StereoSample,
    };
    use groove_proc_macros::Uid;
    use std::sync::{Arc, RwLock};

    #[derive(Debug, Default, Uid, Serialize, Deserialize)]
    struct TestControllable {
        uid: Uid,

        #[serde(skip)]
        tracker: Arc<RwLock<Vec<(Uid, ControlIndex, ControlValue)>>>,
    }
    impl TestControllable {
        fn new_with(
            uid: Uid,
            tracker: Arc<RwLock<Vec<(Uid, ControlIndex, ControlValue)>>>,
        ) -> Self {
            Self { uid, tracker }
        }
    }
    impl IsInstrument for TestControllable {}
    impl Controllable for TestControllable {
        fn control_set_param_by_index(&mut self, index: usize, value: F32ControlValue) {
            if let Ok(mut tracker) = self.tracker.write() {
                tracker.push((self.uid, ControlIndex(index), ControlValue(value.0 as f64)));
            }
        }
    }
    impl HandlesMidi for TestControllable {}
    impl Generates<StereoSample> for TestControllable {
        fn value(&self) -> StereoSample {
            StereoSample::SILENCE
        }

        fn generate_batch_values(&mut self, _values: &mut [StereoSample]) {
            todo!()
        }
    }
    impl Ticks for TestControllable {
        fn tick(&mut self, _tick_count: usize) {
            todo!()
        }
    }
    impl Configurable for TestControllable {}
    impl Shows for TestControllable {}
    #[typetag::serde]
    impl Thing for TestControllable {
        fn thing_type(&self) -> ThingType {
            ThingType::Instrument
        }
        fn as_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
            Some(self)
        }
    }

    #[test]
    fn crud_works() {
        let mut cr = ControlRouter::default();
        assert!(
            cr.uid_to_control.is_empty(),
            "new ControlRouter should be empty"
        );

        let source_uid = Uid(1);
        let source_2_uid = Uid(2);
        let target_uid = Uid(3);
        let target_2_uid = Uid(4);

        cr.link_control(source_uid, target_uid, ControlIndex(0));
        assert_eq!(
            cr.uid_to_control.len(),
            1,
            "there should be one vec after inserting one link"
        );
        cr.link_control(source_uid, target_2_uid, ControlIndex(1));
        assert_eq!(
            cr.uid_to_control.len(),
            1,
            "there should still be one vec after inserting a second link for same source_uid"
        );
        cr.link_control(source_2_uid, target_uid, ControlIndex(0));
        assert_eq!(
            cr.uid_to_control.len(),
            2,
            "there should be two vecs after inserting one link for a second Uid"
        );

        assert_eq!(
            cr.control_links(source_uid).unwrap().len(),
            2,
            "the first source's vec should have two entries"
        );
        assert_eq!(
            cr.control_links(source_2_uid).unwrap().len(),
            1,
            "the second source's vec should have one entry"
        );

        let mut es = ThingStore::default();
        let tracker = Arc::new(RwLock::new(Vec::default()));
        let controllable = TestControllable::new_with(target_uid, Arc::clone(&tracker));
        es.add(Box::new(controllable));
        let controllable = TestControllable::new_with(target_2_uid, Arc::clone(&tracker));
        es.add(Box::new(controllable));

        cr.route(&mut es, source_uid, ControlValue(0.5));
        if let Ok(t) = tracker.read() {
            assert_eq!(
                t.len(),
                2,
                "there should be expected number of control events after the route {:#?}",
                t
            );
            assert_eq!(t[0], (target_uid, ControlIndex(0), ControlValue(0.5)));
            assert_eq!(t[1], (target_2_uid, ControlIndex(1), ControlValue(0.5)));
        };

        // Try removing links. Start with nonexistent link
        if let Ok(mut t) = tracker.write() {
            t.clear();
        }
        cr.unlink_control(source_uid, target_uid, ControlIndex(99));
        cr.route(&mut es, source_uid, ControlValue(0.5));
        if let Ok(t) = tracker.read() {
            assert_eq!(
                t.len(),
                2,
                "route results shouldn't change when removing nonexistent link {:#?}",
                t
            );
        };

        if let Ok(mut t) = tracker.write() {
            t.clear();
        }
        cr.unlink_control(source_uid, target_uid, ControlIndex(0));
        cr.route(&mut es, source_uid, ControlValue(0.5));
        if let Ok(t) = tracker.read() {
            assert_eq!(
                t.len(),
                1,
                "removing a link should continue routing to remaining ones {:#?}",
                t
            );
            assert_eq!(t[0], (target_2_uid, ControlIndex(1), ControlValue(0.5)));
        };
    }
}
