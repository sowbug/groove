// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_orchestration::{helpers::IOHelper, Orchestrator, Performance};
use groove_settings::SongSettings;
use groove_utils::{PathType, Paths};
use native_dialog::FileDialog;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum OpenError {
    Unknown,
}

#[derive(Debug, Clone)]
pub enum LoadError {
    File,
    Format,
}

#[derive(Debug, Clone)]
pub enum SaveError {
    File,
    Write,
    Format,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub selected_midi_input: Option<String>,
    pub selected_midi_output: Option<String>,

    pub should_reload_last_project: bool,
    pub last_project_filename: Option<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            selected_midi_input: Default::default(),
            selected_midi_output: Default::default(),
            should_reload_last_project: true,
            last_project_filename: Some(String::from("default.yaml")),
        }
    }
}

impl Preferences {
    pub async fn load_prefs() -> anyhow::Result<Preferences, LoadError> {
        use async_std::prelude::*;
        let mut contents = String::new();
        let mut file = async_std::fs::File::open(Paths::prefs())
            .await
            .map_err(|_| LoadError::File)?;
        file.read_to_string(&mut contents)
            .await
            .map_err(|_| LoadError::File)?;
        serde_json::from_str(&contents).map_err(|_| LoadError::Format)
    }

    pub async fn save_prefs(self) -> Result<(), SaveError> {
        use async_std::prelude::*;

        let json = serde_json::to_string_pretty(&self).map_err(|_| SaveError::Format)?;
        let path = Paths::prefs();
        if let Some(dir) = path.parent() {
            async_std::fs::create_dir_all(dir)
                .await
                .map_err(|_| SaveError::File)?;
        }

        {
            let mut file = async_std::fs::File::create(path)
                .await
                .map_err(|_| SaveError::File)?;

            file.write_all(json.as_bytes())
                .await
                .map_err(|_| SaveError::Write)?;
        }

        // TODO: re-implement the is_dirty thing to save regularly. As-is, we're
        // saving only on a requested close, which means we'll lose work
        // whenever we crash. (Not such a big deal right now because we don't
        // serialize anything.)
        //
        // This is a simple way to save at most once every couple seconds
        // async_std::task::sleep(std::time::Duration::from_secs(2)).await;

        Ok(())
    }
}

pub async fn open_dialog() -> Result<Option<PathBuf>, OpenError> {
    match FileDialog::new()
        .add_filter("YAML", &["yml", "yaml"])
        .add_filter("Groove Projects", &["nsn"])
        .show_open_single_file()
    {
        Ok(path) => {
            if let Some(path) = path {
                // The user selected a file
                Ok(Some(path))
            } else {
                // The user canceled
                Ok(None)
            }
        }
        Err(e) => {
            // something went wrong
            eprintln!("open dialog error: {:?}", e);
            Err(OpenError::Unknown)
        }
    }
}

pub async fn export_to_wav(performance: Performance) -> Result<(), SaveError> {
    if let Ok(Some(path)) = FileDialog::new()
        .set_filename("output.wav")
        .show_save_single_file()
    {
        if IOHelper::send_performance_to_file(&performance, &path).is_ok() {
            return Ok(());
        }
    }
    Err(SaveError::Write)
}
pub async fn export_to_mp3(performance: Performance) -> Result<(), SaveError> {
    if let Ok(Some(path)) = FileDialog::new()
        .set_filename("output.mp3")
        .show_save_single_file()
    {
        // TODO: have to find a properly licensed MP3 encoding library
        if IOHelper::send_performance_to_file(&performance, &path).is_ok() {
            return Ok(());
        }
    }
    Err(SaveError::Write)
}

pub async fn load_project(filename: PathBuf) -> Result<(Orchestrator, String), LoadError> {
    use async_std::prelude::*;

    if let Some(filename) = filename.to_str() {
        let mut path = Paths::projects_path(&PathType::Global);
        path.push(filename);

        let mut contents = String::new();
        let mut file = async_std::fs::File::open(path)
            .await
            .map_err(|_| LoadError::File)?;
        file.read_to_string(&mut contents)
            .await
            .map_err(|_| LoadError::File)?;

        if let Ok(settings) = serde_yaml::from_str::<SongSettings>(contents.as_str()) {
            if let Ok(instance) =
                settings.instantiate(&Paths::assets_path(&PathType::Global), false)
            {
                return Ok((instance, filename.to_string()));
            }
        }
    }
    Err(LoadError::File)
}
