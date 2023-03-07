// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::{
    env::{current_dir, current_exe},
    fs,
    path::PathBuf,
};

pub struct Paths {}
impl Paths {
    pub fn cwd() -> PathBuf {
        PathBuf::from(
            current_dir()
                .ok()
                .map(PathBuf::into_os_string)
                .and_then(|exe| exe.into_string().ok())
                .unwrap(),
        )
    }

    pub fn asset_path() -> PathBuf {
        const ASSETS: &str = "assets";
        let mut path_buf = Self::cwd();
        path_buf.push(ASSETS);
        path_buf
    }

    pub fn project_path() -> PathBuf {
        const PROJECTS: &str = "projects";
        let mut path_buf = Self::cwd();
        path_buf.push(PROJECTS);
        path_buf
    }

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

    //#[cfg(test)]
    pub fn test_data_path() -> PathBuf {
        const TEST_DATA: &str = "test-data";
        let mut path_buf = Self::cwd();
        path_buf.push(TEST_DATA);
        path_buf
    }

    //#[cfg(test)]
    pub fn out_path() -> PathBuf {
        const OUT_DATA: &str = "target";
        let mut path_buf = Self::cwd();
        path_buf.push(OUT_DATA);
        if let Ok(_) = fs::create_dir_all(&path_buf) {
            path_buf
        } else {
            panic!(
                "Could not create output directory {:?} for writing",
                &path_buf
            );
        }
    }
}
