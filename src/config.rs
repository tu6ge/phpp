use std::{
    fs::{create_dir_all, read_to_string, File},
    io::Write,
};

use dirs::home_dir;
use serde::{Deserialize, Serialize};

use crate::error::ComposerError;

const CONFIG_DIR: &str = ".config/phpp";

#[derive(Debug, Deserialize, Clone, Serialize, Default)]
pub struct GlobalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    repositories: Option<Repositories>,
}

impl GlobalConfig {
    pub fn new() -> Result<GlobalConfig, ComposerError> {
        let config_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CONFIG_DIR);

        create_dir_all(&config_dir)?;
        let path = config_dir.join("config.json");

        if !path.exists() {
            return Ok(Self::default());
        }

        let cp: Self = serde_json::from_str(&read_to_string(path)?)?;

        Ok(cp)
    }

    pub fn set(
        &mut self,
        key: &str,
        value1: &str,
        value2: &Option<String>,
    ) -> Result<(), ComposerError> {
        match key {
            "repo.packagist" => {
                if let Some(value2) = value2 {
                    self.set_repo(value1, value2)?;
                }
            }
            _ => todo!(),
        }

        Ok(())
    }
    pub fn unset(&mut self, key: &str) -> Result<(), ComposerError> {
        match key {
            "repo.packagist" => {
                self.repositories = None;
            }
            _ => todo!(),
        }

        Ok(())
    }

    fn set_repo(&mut self, value1: &str, value2: &str) -> Result<(), ComposerError> {
        let repo = self.repositories.take();

        self.repositories = match repo {
            Some(mut repo) => {
                repo.packagist._type = value1.to_owned();
                repo.packagist.url = value2.to_owned();
                Some(repo)
            }
            None => {
                let repo = Repositories {
                    packagist: Packagist {
                        _type: value1.to_owned(),
                        url: value2.to_owned(),
                    },
                };
                Some(repo)
            }
        };

        Ok(())
    }

    pub fn save(&self) -> Result<(), ComposerError> {
        let config_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CONFIG_DIR);

        create_dir_all(&config_dir)?;
        let path = config_dir.join("config.json");
        let mut f = File::create(path)?;
        let content = serde_json::to_string_pretty(&self)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub(crate) struct Repositories {
    packagist: Packagist,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
struct Packagist {
    #[serde(rename = "type")]
    _type: String,
    url: String,
}
