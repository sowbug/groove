// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::TrackUid;
use groove_core::Normal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct BusRoute {
    pub aux_track_uid: TrackUid,
    pub amount: Normal,
}

/// A [BusStation] manages how signals move between tracks and aux tracks. These
/// collections of signals are sometimes called buses.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct BusStation {
    send_routes: HashMap<TrackUid, Vec<BusRoute>>,
}

impl BusStation {
    pub(crate) fn add_send_route(
        &mut self,
        track_uid: TrackUid,
        route: BusRoute,
    ) -> anyhow::Result<()> {
        self.send_routes.entry(track_uid).or_default().push(route);
        Ok(())
    }

    pub(crate) fn send_routes(&self) -> impl Iterator<Item = (&TrackUid, &Vec<BusRoute>)> {
        self.send_routes.iter()
    }

    #[allow(dead_code)]
    pub(crate) fn remove_send_route(&mut self, track_uid: &TrackUid, aux_track_uid: &TrackUid) {
        if let Some(routes) = self.send_routes.get_mut(track_uid) {
            routes.retain(|route| route.aux_track_uid != *aux_track_uid);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn remove_track_sends(&mut self, track_uid: &TrackUid) {
        self.send_routes.retain(|uid, _| track_uid != uid);
        self.send_routes.entry(*track_uid).or_default();
    }

    // If we want this method to be immutable and cheap, then we can't guarantee
    // that it will return a Vec. Such is life.
    #[allow(dead_code)]
    pub(crate) fn sends_for(&self, track_uid: &TrackUid) -> Option<&Vec<BusRoute>> {
        self.send_routes.get(track_uid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let mut bs = BusStation::default();
        assert!(bs.send_routes.is_empty());

        assert!(bs
            .add_send_route(
                TrackUid(7),
                BusRoute {
                    aux_track_uid: TrackUid(13),
                    amount: Normal::from(0.8),
                },
            )
            .is_ok());
        assert_eq!(bs.send_routes.len(), 1);

        assert!(bs
            .add_send_route(
                TrackUid(7),
                BusRoute {
                    aux_track_uid: TrackUid(13),
                    amount: Normal::from(0.7),
                },
            )
            .is_ok());
        assert_eq!(
            bs.send_routes.len(),
            1,
            "Adding a new send route with a new amount should replace the prior one"
        );

        bs.remove_send_route(&TrackUid(7), &TrackUid(13));
        assert_eq!(
            bs.send_routes.len(),
            1,
            "Removing route should still leave a (possibly empty) Vec"
        );
        assert!(
            bs.sends_for(&TrackUid(7)).unwrap().is_empty(),
            "Removing route should work"
        );

        // Removing nonexistent route is a no-op
        bs.remove_send_route(&TrackUid(7), &TrackUid(13));

        assert!(bs
            .add_send_route(
                TrackUid(7),
                BusRoute {
                    aux_track_uid: TrackUid(13),
                    amount: Normal::from(0.8),
                },
            )
            .is_ok());
        assert!(bs
            .add_send_route(
                TrackUid(7),
                BusRoute {
                    aux_track_uid: TrackUid(14),
                    amount: Normal::from(0.8),
                },
            )
            .is_ok());
        assert_eq!(
            bs.send_routes.len(),
            1,
            "Adding two sends to a track should not create an extra Vec"
        );
        assert_eq!(
            bs.sends_for(&TrackUid(7)).unwrap().len(),
            2,
            "Adding two sends to a track should work"
        );
        bs.remove_track_sends(&TrackUid(7));
        assert!(
            bs.sends_for(&TrackUid(7)).unwrap().is_empty(),
            "Removing all a track's sends should work"
        );
    }
}
