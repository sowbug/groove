use serde::{Deserialize, Serialize};

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
    fn path_prefs() -> std::path::PathBuf {
        let mut path = if let Some(project_dirs) =
            directories_next::ProjectDirs::from("me", "ensnare", "Ensnare")
        {
            // Linux: /home/alice/.config/ensnare
            // Win: C:\Users\Alice\AppData\Roaming\ensnare\Ensnare\config
            // Mac: /Users/Alice/Library/Application Support/me.ensnare.Ensnare
            project_dirs.config_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        };

        path.push("preferences.json");

        path
    }

    pub async fn load_prefs() -> anyhow::Result<Preferences, LoadError> {
        use async_std::prelude::*;
        let mut contents = String::new();
        let mut file = async_std::fs::File::open(Self::path_prefs())
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
        let path = Self::path_prefs();
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
        // whenever we crash.
        //        
        // This is a simple way to save at most once every couple seconds
        // async_std::task::sleep(std::time::Duration::from_secs(2)).await;

        Ok(())
    }
}
