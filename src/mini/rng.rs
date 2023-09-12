// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Rng(pub oorandom::Rand64);
impl Default for Rng {
    fn default() -> Self {
        // This is an awful source of entropy, but it's fine for this use case
        // where we just want a different fake struct each time.
        Self(oorandom::Rand64::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ))
    }
}
