//! about composer.json

use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    error::ComposerError,
    package::{ComposerLock, Context, P2},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Composer {
    pub(crate) require: Option<HashMap<String, String>>,
}

impl Composer {
    pub fn new() -> Result<Composer, ComposerError> {
        let path = Path::new("./composer.json");

        let cp: Self = serde_json::from_str(&read_to_string(path)?)?;

        Ok(cp)
    }

    pub async fn install(mut self) -> Result<(), ComposerError> {
        let ctx = Arc::new(Mutex::new(Context::default()));
        let list = self.require.take();
        if let Some(list) = list {
            for (name, version) in list.iter() {
                let mut c = ctx.lock().unwrap();
                c.first_package = None;
                drop(c);

                let version = if version == "*" {
                    None
                } else {
                    Some(version.to_owned())
                };

                let _ = P2::new(name.to_owned(), version, ctx.clone())
                    .await
                    .expect("download error");

                let c = ctx.lock().unwrap();
                if let Some(p) = &c.first_package {
                    let version = &p.version;
                    let mut this = Self::new()?;
                    this.set_version(name, version);
                    this.save();
                }
            }

            let packages = ComposerLock::new(ctx);

            packages.installing().await?;
        }

        Ok(())
    }

    fn set_version(&mut self, name: &str, version: &str) {
        if let Some(mut list) = self.require.take() {
            list.entry(name.to_string()).and_modify(|e| {
                if e == "*" {
                    *e = version.to_string();
                }
            });
            self.require = Some(list);
        }
    }

    pub fn insert(&mut self, name: &str, version: Option<String>) -> Result<(), ComposerError> {
        let version = version.unwrap_or("*".to_owned());

        self.require = match self.require.take() {
            Some(mut list) => {
                list.insert(name.to_owned(), version);

                Some(list)
            }
            None => {
                let mut map = HashMap::new();
                map.insert(name.to_owned(), version);
                Some(map)
            }
        };

        Ok(())
    }
    pub fn remove(&mut self, name: &str) -> Result<(), ComposerError> {
        let require = self.require.take();
        if let Some(mut list) = require {
            list.remove(name);
            self.require = Some(list);

            // TODO remove vendor content
        }
        Ok(())
    }

    pub fn save(&self) {
        let path = Path::new("./composer.json");
        let mut f = File::create(path).unwrap();
        let content = serde_json::to_string_pretty(&self).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }
}
