use groove::IOHelper;
use groove::Paths;
use groove::SongSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedState {
    pub project_name: String,
    pub song_settings: SongSettings,
}

#[derive(Debug, Clone)]
pub enum LoadError {
    FileError,
    FormatError,
}

#[derive(Debug, Clone)]
pub enum SaveError {
    File,
    Write,
    Format,
}

impl SavedState {
    pub async fn load() -> anyhow::Result<SavedState, LoadError> {
        //  use async_std::prelude::*;

        // let mut contents = String::new();

        // let mut file = async_std::fs::File::open(Self::path())
        //     .await
        //     .map_err(|_| LoadError::FileError)?;

        // file.read_to_string(&mut contents)
        //     .await
        //     .map_err(|_| LoadError::FileError)?;

        // serde_json::from_str(&contents).map_err(|_| LoadError::FormatError)

        let mut path = Paths::project_path();
        path.push("low-cpu.yaml");
        match IOHelper::song_settings_from_yaml_file(path.to_str().unwrap()) {
            Ok(song_settings) => Ok(SavedState {
                project_name: "Woop Woop Woop".to_string(),
                song_settings,
            }),
            Err(err) => {
                println!("Error: {}", err);
                Err(LoadError::FileError)
            }
        }
    }

    pub async fn save(self) -> Result<(), SaveError> {
        // use async_std::prelude::*;

        // let json = serde_json::to_string_pretty(&self).map_err(|_| SaveError::FormatError)?;

        // let path = Self::path();

        // if let Some(dir) = path.parent() {
        //     async_std::fs::create_dir_all(dir)
        //         .await
        //         .map_err(|_| SaveError::FileError)?;
        // }

        // {
        //     let mut file = async_std::fs::File::create(path)
        //         .await
        //         .map_err(|_| SaveError::FileError)?;

        //     file.write_all(json.as_bytes())
        //         .await
        //         .map_err(|_| SaveError::WriteError)?;
        // }

        // // This is a simple way to save at most once every couple seconds
        // async_std::task::sleep(std::time::Duration::from_secs(2)).await;

        Ok(())
    }
}
