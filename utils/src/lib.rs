// Copyright (c) 2023 Mike Tsao. All rights reserved.

use app_dirs2::{AppDataType, AppInfo};
use std::{
    env::{current_dir, current_exe},
    path::PathBuf,
};

const APP_INFO: AppInfo = AppInfo {
    name: "Groove",
    author: "Mike Tsao <mike@sowbug.com>",
};

/// How to load resources
pub enum PathType {
    /// The current working directory is root.
    Dev,

    /// A subdirectory in the OS-specific per-user documents directory is root.
    User,

    /// A system-wide installation directory is root.
    Global,
}

/// Paths contains path-building utilities.
pub struct Paths {}
impl Paths {
    /// The name of the subdirectory in the assets directory holding samples.
    pub const SAMPLES: &str = "samples";
    /// The name of the subdirectory in the assets directory holding projects.
    pub const PROJECTS: &str = "projects";
    /// The name of the subdirectory in the assets directory holding patches.
    pub const PATCHES: &str = "patches";

    fn cwd() -> PathBuf {
        PathBuf::from(
            current_dir()
                .ok()
                .map(PathBuf::into_os_string)
                .and_then(|exe| exe.into_string().ok())
                .unwrap(),
        )
    }

    /// Returns the directory containing assets installed with the application.
    pub fn assets_path(path_type: PathType) -> PathBuf {
        match path_type {
            PathType::Dev => Self::cwd().join("assets"),
            PathType::User => app_dirs2::get_app_root(AppDataType::UserData, &APP_INFO).unwrap(),
            PathType::Global => {
                if cfg!(unix) {
                    PathBuf::from("/usr/share/groove")
                } else {
                    app_dirs2::get_app_root(AppDataType::SharedData, &APP_INFO).unwrap()
                }
            }
        }
    }

    /// Returns the directory containing projects installed with the application.
    pub fn projects_path(path_type: PathType) -> PathBuf {
        let mut path = Self::assets_path(path_type);
        path.push(Self::PROJECTS);
        path
    }

    /// Returns the directory containing patches installed with the application.
    pub fn patches_path(path_type: PathType) -> PathBuf {
        let mut path = Self::assets_path(path_type);
        path.push(Self::PATCHES);
        path
    }

    /// Returns the directory containing samples installed with the application.
    pub fn samples_path(path_type: PathType) -> PathBuf {
        let mut path = Self::assets_path(path_type);
        path.push(Self::SAMPLES);
        path
    }

    /// Returns the directory containing the current executable.
    #[allow(dead_code)]
    pub fn exe_path() -> PathBuf {
        PathBuf::from(
            current_exe()
                .ok()
                .map(PathBuf::into_os_string)
                .and_then(|exe| exe.into_string().ok())
                .unwrap(),
        )
    }

    /// Returns the path of the user's preferences file.
    pub fn prefs() -> PathBuf {
        // See https://docs.rs/app_dirs2/latest/app_dirs2/ for platform-specific
        // example paths
        let mut path = app_dirs2::get_app_root(AppDataType::UserConfig, &APP_INFO)
            .unwrap_or(std::env::current_dir().unwrap_or_default());

        path.push("preferences.json");

        path
    }
}
