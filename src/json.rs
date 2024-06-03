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

    pub async fn install(&self) -> Result<(), ComposerError> {
        let ctx = Arc::new(Mutex::new(Context::default()));
        if let Some(list) = &self.require {
            for (name, _) in list.iter() {
                let _ = P2::new(name.to_owned(), None, ctx.clone())
                    .await
                    .expect("download error");
            }

            let packages = ComposerLock::new(ctx);
            packages.save_file();

            packages.down_package().await.expect("download dist failed");

            packages.install_package().expect("install package failed");
        }

        Ok(())
    }

    pub fn insert(&mut self, name: &str) -> Result<(), ComposerError> {
        self.require = match &self.require {
            Some(list) => {
                let mut list = list.clone();
                list.insert(name.to_owned(), "*".to_owned());

                Some(list)
            }
            None => {
                let mut map = HashMap::new();
                map.insert(name.to_owned(), "*".to_owned());
                Some(map)
            }
        };

        Ok(())
    }
    pub fn remove(&mut self, name: &str) -> Result<(), ComposerError> {
        if let Some(ref list) = &self.require {
            let mut list = list.clone();
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
