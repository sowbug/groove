// Copyright (c) 2023 Mike Tsao. All rights reserved.

#[cfg(test)]
pub(crate) mod tests {
    use std::{env::current_dir, fs, path::PathBuf};

    pub struct TestOnlyPaths {}
    impl TestOnlyPaths {
        pub fn cwd() -> PathBuf {
            PathBuf::from(
                current_dir()
                    .ok()
                    .map(PathBuf::into_os_string)
                    .and_then(|exe| exe.into_string().ok())
                    .unwrap(),
            )
        }

        pub fn test_data_path() -> PathBuf {
            const TEST_DATA: &str = "test-data";
            let mut path_buf = Self::cwd();
            path_buf.push(TEST_DATA);
            path_buf
        }

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
}
