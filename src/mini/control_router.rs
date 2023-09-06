// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    control::{ControlIndex, ControlValue},
    Uid,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ControlRouter {
    uid_to_control: HashMap<Uid, Vec<(Uid, ControlIndex)>>,
}
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

    pub fn route(
        &mut self,
        entity_store_fn: &mut dyn FnMut(&Uid, ControlIndex, ControlValue),
        source_uid: Uid,
        value: ControlValue,
    ) -> anyhow::Result<()> {
        if let Some(control_links) = self.control_links(source_uid) {
            control_links.iter().for_each(|(target_uid, index)| {
                entity_store_fn(target_uid, *index, value);
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini::entity_factory::ThingStore;
    use groove_core::{
        traits::{
            gui::Displays, Configurable, Controllable, Generates, HandlesMidi, Serializable, Ticks,
        },
        StereoSample,
    };
    use groove_proc_macros::{IsInstrument, Uid};
    use std::sync::{Arc, RwLock};

    #[derive(Debug, Default, IsInstrument, Uid, Serialize, Deserialize)]
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
    impl Controllable for TestControllable {
        fn control_set_param_by_index(&mut self, index: ControlIndex, value: ControlValue) {
            if let Ok(mut tracker) = self.tracker.write() {
                tracker.push((self.uid, index, value));
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
    impl Serializable for TestControllable {}
    impl Configurable for TestControllable {}
    impl Displays for TestControllable {}

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
        let _ = es.add(Box::new(controllable));
        let controllable = TestControllable::new_with(target_2_uid, Arc::clone(&tracker));
        let _ = es.add(Box::new(controllable));

        let _ = cr.route(
            &mut |target_uid, index, value| {
                if let Some(e) = es.get_mut(target_uid) {
                    if let Some(e) = e.as_controllable_mut() {
                        e.control_set_param_by_index(index, value);
                    }
                }
            },
            source_uid,
            ControlValue(0.5),
        );
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
        let _ = cr.route(
            &mut |target_uid, index, value| {
                if let Some(e) = es.get_mut(target_uid) {
                    if let Some(e) = e.as_controllable_mut() {
                        e.control_set_param_by_index(index, value);
                    }
                }
            },
            source_uid,
            ControlValue(0.5),
        );
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
        let _ = cr.route(
            &mut |target_uid, index, value| {
                if let Some(e) = es.get_mut(target_uid) {
                    if let Some(e) = e.as_controllable_mut() {
                        e.control_set_param_by_index(index, value);
                    }
                }
            },
            source_uid,
            ControlValue(0.5),
        );
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
