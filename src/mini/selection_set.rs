// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::collections::{hash_set::Iter, HashSet};

use groove_core::IsUid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct SelectionSet<T: IsUid> {
    selected_uids: HashSet<T>,
}

impl<T: IsUid> SelectionSet<T> {
    pub fn insert(&mut self, uid: T) {
        self.selected_uids.insert(uid);
    }

    pub fn remove(&mut self, uid: &T) {
        self.selected_uids.remove(uid);
    }

    pub fn set_selected(&mut self, uid: T, selected: bool) {
        if selected {
            self.insert(uid);
        } else {
            self.remove(&uid);
        }
    }

    pub fn contains(&self, uid: &T) -> bool {
        self.selected_uids.contains(uid)
    }

    pub fn clear(&mut self) {
        self.selected_uids.clear();
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.selected_uids.iter()
    }

    pub fn len(&self) -> usize {
        self.selected_uids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.selected_uids.is_empty()
    }

    pub fn click(&mut self, uid: T, is_control_pressed: bool) {
        let is_selected = self.contains(&uid);
        if !is_control_pressed {
            self.clear();
        }
        if is_selected {
            self.remove(&uid);
        } else {
            self.insert(uid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::Uid;

    #[test]
    fn select_mainline() {
        let mut st = SelectionSet::default();

        assert!(st.is_empty());
        assert_eq!(st.len(), 0);

        let uid2048 = Uid(2048);
        let uid2049 = Uid(2049);
        assert!(!st.contains(&uid2048));

        st.insert(uid2048);
        assert!(st.contains(&uid2048));

        st.clear();
        assert!(st.is_empty());

        st.click(uid2048, false);
        assert_eq!(st.len(), 1);
        assert!(st.contains(&uid2048));
        assert!(!st.contains(&uid2049));

        st.click(uid2049, true);
        assert_eq!(st.len(), 2);
        assert!(st.contains(&uid2048));
        assert!(st.contains(&uid2049));

        st.click(uid2049, true);
        assert!(st.contains(&uid2048));
        assert!(!st.contains(&uid2049));

        st.click(uid2048, true);
        assert!(st.is_empty());
    }
}
