use app_dirs2::{AppDataType, AppInfo};
use std::{
    env::{current_dir, current_exe},
    path::PathBuf,
};

const APP_INFO: AppInfo = AppInfo {
    name: "Groove",
    author: "Mike Tsao <mike@sowbug.com>",
};

/// Paths contains path-building utilities.
pub struct Paths {}
impl Paths {
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
    pub fn asset_path() -> PathBuf {
        const ASSETS: &str = "assets";
        let mut path_buf = Self::cwd();
        path_buf.push(ASSETS);
        path_buf
    }

    /// Returns the directory containing projects installed with the application.
    pub fn project_path() -> PathBuf {
        const PROJECTS: &str = "projects";
        let mut path_buf = Self::cwd();
        path_buf.push(PROJECTS);
        path_buf
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
    pub fn prefs() -> std::path::PathBuf {
        // See https://docs.rs/app_dirs2/latest/app_dirs2/ for platform-specific
        // example paths
        let mut path = app_dirs2::get_app_root(AppDataType::UserConfig, &APP_INFO)
            .unwrap_or(std::env::current_dir().unwrap_or_default());

        path.push("preferences.json");

        path
    }
}
