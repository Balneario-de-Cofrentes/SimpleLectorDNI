//! JSON settings in a directory (the Tauri app config directory in practice).

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn load<T: Default + DeserializeOwned>(directory: &Path, file: &str) -> Result<T, String> {
    let path = directory.join(file);
    if !path.exists() {
        return Ok(T::default());
    }
    let json = std::fs::read(&path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&json).map_err(|error| error.to_string())
}

pub fn save<T: Serialize>(directory: &Path, file: &str, value: &T) -> Result<PathBuf, String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let path = directory.join(file);
    let json = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    std::fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{load, save};

    #[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
    struct Example {
        reader: String,
        attempts: u8,
    }

    #[test]
    fn missing_file_yields_defaults_and_saved_values_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let nested = directory.path().join("app");

        assert_eq!(
            load::<Example>(&nested, "s.json").unwrap(),
            Example::default()
        );
        save(
            &nested,
            "s.json",
            &Example {
                reader: "EMV".to_owned(),
                attempts: 2,
            },
        )
        .unwrap();
        assert_eq!(
            load::<Example>(&nested, "s.json").unwrap(),
            Example {
                reader: "EMV".to_owned(),
                attempts: 2
            }
        );
    }
}
