// Copyright (c) 2023 Mike Tsao. All rights reserved.

use app_dirs2::{AppDataType, AppInfo};
use std::{
    env::{current_dir, current_exe},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use strum_macros::{Display, EnumIter, IntoStaticStr};

const APP_INFO: AppInfo = AppInfo {
    name: "Groove",
    author: "Mike Tsao <mike@sowbug.com>",
};

/// How to load resources.
#[derive(Debug, Display, EnumIter, IntoStaticStr)]
pub enum PathType {
    /// A directory specified at app installation.
    System,

    /// A directory within the OS-specific per-user directory.
    User,

    /// $cwd. Useful during development.
    Cwd,

    /// $cwd/assets.
    Local,

    /// $cwd/test-data. This one is loaded explicitly by tests.
    Test,
}

#[derive(Debug, EnumIter)]
pub enum FileType {
    Project,
    Patch,
    Sample,
}

/// Paths contains path-building utilities.
#[derive(Clone, Debug)]
pub struct Paths {
    hives: Vec<PathBuf>,
}
impl Default for Paths {
    fn default() -> Self {
        if cfg!(unix) {
            Self {
                hives: vec![
                    Self::hive(PathType::System),
                    // TODO: this is annoying. It appears that app_dirs2 grabs
                    // the first directory in XDG_DATA_DIRS, which on my machine
                    // is a weird app-specific directory. Since it seems like
                    // anyone can add anything to XDG_DATA_DIRS in any order
                    // they want, I don't know that app_dirs2 is being
                    // unreasonable. For now I'm hacking it to match where my
                    // .deb currently puts the assets.
                    //
                    // We put this directory just before the incorrect System
                    // hive (before = the search order), so that if it is
                    // invalid, we'll still have something at the system level.
                    PathBuf::from("/usr/share/groove"),
                    Self::hive(PathType::User),
                    Self::hive(PathType::Local),
                ],
            }
        } else {
            Self {
                hives: vec![
                    Self::hive(PathType::System),
                    Self::hive(PathType::User),
                    Self::hive(PathType::Local),
                ],
            }
        }
    }
}
impl Paths {
    /// WAV, AIFF, etc.
    const SAMPLES: &str = "samples";

    /// Project files (.yaml, .ens, etc.).
    const PROJECTS: &str = "projects";

    /// Instrument patch files.
    const PATCHES: &str = "patches";

    /// The directory containing assets like samples, patches, and demo projects.
    const ASSETS: &str = "assets";

    /// The directory containing data used by unit tests.
    const TEST_DATA: &str = "test-data";

    /// The name of the app's preferences file.
    const PREFERENCES: &str = "preferences.json";

    pub fn clear_hives(&mut self) {
        self.hives.clear();
    }

    /// Adds a hive to the end of the search list.
    pub fn push_hive(&mut self, path: &Path) {
        let p = path.to_path_buf();
        if !self.hives.contains(&p) {
            self.hives.push(p)
        }
    }

    /// Inserts a hive at the start of the search list.
    pub fn insert_hive(&mut self, path: &Path) {
        let p = path.to_path_buf();
        if !self.hives.contains(&p) {
            self.hives.push(p)
        }
    }

    // https://devblogs.microsoft.com/oldnewthing/20030808-00/?p=42943
    pub fn hive(path_type: PathType) -> PathBuf {
        let r = match path_type {
            PathType::System => {
                app_dirs2::get_app_root(AppDataType::SharedData, &APP_INFO).unwrap()
            }
            PathType::User => app_dirs2::get_app_root(AppDataType::UserData, &APP_INFO).unwrap(),
            PathType::Cwd => Self::cwd(),
            PathType::Local => Self::cwd().join(Self::assets_rel()),
            PathType::Test => Self::cwd().join(Self::test_data_rel()),
        };
        r
    }

    pub fn hives(&self) -> &[PathBuf] {
        self.hives.as_ref()
    }

    fn cwd() -> PathBuf {
        PathBuf::from(
            current_dir()
                .ok()
                .map(PathBuf::into_os_string)
                .and_then(|exe| exe.into_string().ok())
                .unwrap(),
        )
    }

    /// Returns the directory for storing the preferences file and anything else
    /// that's per-user and small.
    pub fn config() -> PathBuf {
        app_dirs2::get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap()
    }

    /// Returns the full path of the user's preferences file.
    pub fn prefs_file() -> PathBuf {
        let mut r = Self::config();
        r.push(Self::PREFERENCES);
        r
    }

    /// Returns the directory containing projects installed with the application.
    #[deprecated]
    pub fn projects(path_type: PathType) -> PathBuf {
        let mut path = Self::hive(path_type);
        path.push(Self::PROJECTS);
        path
    }

    /// Returns the directory containing patches installed with the application.
    #[deprecated]
    pub fn patches(path_type: PathType) -> PathBuf {
        let mut path = Self::hive(path_type);
        path.push(Self::PATCHES);
        path
    }

    /// Returns the directory containing samples installed with the application.
    #[deprecated]
    pub fn samples(path_type: PathType) -> PathBuf {
        let mut path = Self::hive(path_type);
        path.push(Self::SAMPLES);
        path
    }

    /// Returns the directory containing the current executable.
    #[deprecated]
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

    pub fn assets_rel() -> &'static Path {
        Path::new(Self::ASSETS)
    }

    pub fn test_data_rel() -> &'static Path {
        Path::new(Self::TEST_DATA)
    }

    pub fn projects_rel() -> &'static Path {
        Path::new(Self::PROJECTS)
    }

    pub fn patches_rel() -> &'static Path {
        Path::new(Self::PATCHES)
    }

    pub fn samples_rel() -> &'static Path {
        Path::new(Self::SAMPLES)
    }

    pub fn rel_for(file_type: FileType) -> &'static Path {
        match file_type {
            FileType::Project => Self::projects_rel(),
            FileType::Patch => Self::patches_rel(),
            FileType::Sample => Self::samples_rel(),
        }
    }

    /// Looks for the given filename in the hives, returning the first match as
    /// an open file. Note that this method searches in reverse order from the
    /// hive order; this is because we want the UI to show the most general
    /// hives first (system, then user), but we want to make it easy to override
    /// a given filename by duplicating it in a more specific hive.
    pub fn search_and_open(&self, filename: &Path) -> anyhow::Result<File, anyhow::Error> {
        for path in self.hives.iter().rev() {
            let mut full_path = path.to_path_buf();
            full_path.push(filename);
            eprintln!("looking in {:?}", full_path.as_path());
            if let Ok(f) = std::fs::File::open(full_path) {
                return Ok(f);
            }
        }
        Err(anyhow::Error::msg("Couldn't find file named {arg}"))
    }

    pub fn search_and_read_to_string(&self, path: &Path) -> anyhow::Result<String, anyhow::Error> {
        let mut file = self.search_and_open(path)?;
        let mut s = String::new();
        if let Ok(_bytes_read) = file.read_to_string(&mut s) {
            Ok(s)
        } else {
            Err(anyhow::format_err!(
                "Couldn't read file {:?} to string",
                path
            ))
        }
    }

    pub fn search_and_open_with_file_type(
        &self,
        file_type: FileType,
        path: &Path,
    ) -> anyhow::Result<File, anyhow::Error> {
        let mut rel_path = Self::rel_for(file_type).to_path_buf();
        rel_path.push(path);
        self.search_and_open(rel_path.as_path())
    }

    pub fn build_patch(&self, instrument: &str, filename: &Path) -> PathBuf {
        let mut path = Self::patches_rel().to_path_buf();
        path.push(instrument);
        path.push(filename);
        path
    }

    pub fn build_sample(&self, dirs: &Vec<&str>, filename: &Path) -> PathBuf {
        let mut path = Self::samples_rel().to_path_buf();
        for dir in dirs {
            path.push(dir);
        }
        path.push(filename);
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    impl Paths {
        fn test_default() -> Self {
            let mut r = Self::default();
            r.push_test_path();
            r
        }

        fn push_test_path(&mut self) {
            self.push_hive(&Self::hive(PathType::Test));
        }
    }

    #[test]
    fn mainline_ok() {
        assert_eq!(Paths::assets_rel(), Path::new("assets"));
        assert_eq!(Paths::samples_rel(), Path::new("samples"));
        assert_eq!(Paths::projects_rel(), Path::new("projects"));
        assert_eq!(Paths::patches_rel(), Path::new("patches"));

        let paths = Paths::default();

        // We don't guarantee this number, but it's good to keep an eye on it in case it changes.
        if cfg!(unix) {
            assert_eq!(paths.hives().len(), 4);
        } else {
            assert_eq!(paths.hives().len(), 3);
        }
    }

    #[test]
    fn file_loading() {
        let paths = Paths::default();
        assert!(paths
            .search_and_open_with_file_type(
                FileType::Sample,
                Path::new("this-file-exists-only-in-test-data-samples.txt"),
            )
            .is_err());
        let paths = Paths::test_default();
        assert!(paths
            .search_and_open_with_file_type(
                FileType::Sample,
                Path::new("this-file-exists-only-in-test-data-samples.txt",)
            )
            .is_ok());
    }

    #[test]
    fn precedence() {
        let filename = Path::new("precedence.txt");
        let mut paths = Paths::default();
        assert!(paths.search_and_read_to_string(filename).is_err());
        paths.clear_hives();
        assert!(paths.search_and_read_to_string(filename).is_err());

        paths.push_hive(Path::new("test-data/hive-general"));
        let f = paths.search_and_read_to_string(filename);
        assert!(f.is_ok());

        // We need to trim rather than comparing with 42\n because some OSes (WINDOWS) think a newline is \r\n, and I'd prefer cross-platform approaches.
        let s = f.unwrap().trim().to_string();
        assert_eq!(s, "42");

        paths.push_hive(Path::new("test-data/hive-specific"));

        let f = paths.search_and_read_to_string(filename);
        assert!(f.is_ok());
        let s = f.unwrap();
        assert_eq!(s, "specific\n");
    }
}
