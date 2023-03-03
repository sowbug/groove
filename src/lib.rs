#![allow(clippy::box_default)]

// TODO: regularly scrutinize the re-exports and make sure they really should be
// top-level
pub use clock::TimeSignature;
pub use controllers::orchestrator::Orchestrator;
pub use entities::Entity;

pub mod common;
pub mod controllers;
pub mod effects;
pub mod engine;
pub mod helpers;
pub mod instruments;
pub mod messages;
pub mod midi;

pub(crate) mod clock;
pub(crate) mod entities;
pub(crate) mod metrics;
pub(crate) mod settings;
pub(crate) mod traits;
pub(crate) mod utils;

// https://stackoverflow.com/a/65972328/344467
pub fn app_version() -> &'static str {
    option_env!("GIT_DESCRIBE")
        .unwrap_or(option_env!("GIT_REV_PARSE").unwrap_or(env!("CARGO_PKG_VERSION")))
}

#[cfg(feature = "scripting")]
pub(crate) mod scripting;
#[cfg(feature = "scripting")]
pub use crate::scripting::ScriptEngine;
