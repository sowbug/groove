// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub mod tests {
    use std::{env::current_dir, fs, path::PathBuf};

    /// Since this struct is in a crate that many other crates use, we can't
    /// protect it with a #[cfg(test)]. But we do put it in the tests module, so
    /// that it'll look strange if anyone tries using it in a non-test
    /// configuration.
    pub struct TestOnlyPaths;

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

        pub fn data_path() -> PathBuf {
            const TEST_DATA: &str = "test-data";
            let mut path_buf = Self::cwd();
            path_buf.push(TEST_DATA);
            path_buf
        }

        /// Returns a [PathBuf] representing the target/ build directory, creating
        /// it if necessary.
        pub fn writable_out_path() -> PathBuf {
            const OUT_DATA: &str = "target";
            let mut path_buf = Self::cwd();
            path_buf.push(OUT_DATA);
            if fs::create_dir_all(&path_buf).is_ok() {
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
