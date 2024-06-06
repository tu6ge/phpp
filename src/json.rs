//! about composer.json

use std::{
    fs::{read_to_string, remove_dir_all, File},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    error::ComposerError,
    package::{ComposerLock, Context, P2},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Composer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) require: Option<IndexMap<String, String>>,
}

impl Composer {
    pub fn new() -> Result<Composer, ComposerError> {
        let path = Path::new("./composer.json");

        let cp: Self = serde_json::from_str(&read_to_string(path)?)?;

        Ok(cp)
    }

    pub async fn get_lock(&mut self) -> Result<ComposerLock, ComposerError> {
        let ctx = Arc::new(Mutex::new(Context::new()?));
        let list = self.require.take();
        if let Some(list) = list {
            for (name, version) in list.iter() {
                {
                    let mut c = ctx.lock().unwrap();
                    c.first_package = None;
                }

                let origin_version = version.clone();

                let version = if version == "*" {
                    None
                } else {
                    Some(version.to_owned())
                };

                P2::down_all(name.to_owned(), version, ctx.clone())
                    .await
                    .expect("download error");

                let c = ctx.lock().unwrap();
                if let Some(p) = &c.first_package {
                    let version = &p.version;
                    let mut this = Self::new()?;
                    this.set_version(name, version);
                    this.save()?;
                }

                Self::eprint_php_version(name, &origin_version, &c.php_version_error)?;
                Self::eprint_extensions(name, &origin_version, &c.php_extensions_error)?;
            }
        }

        Ok(ComposerLock::new(ctx))
    }

    /// php version is not satisfy, return failure
    fn eprint_php_version(
        name: &str,
        origin_version: &str,
        list: &Vec<(String, String)>,
    ) -> Result<(), ComposerError> {
        if list.len() > 0 {
            for (i, item) in list.iter().enumerate() {
                eprintln!(
                    "{name}({}) -> .. -> {} need PHP version is {}",
                    origin_version, item.0, item.1
                );
                if i > 2 {
                    break;
                }
            }

            // rollback
            let mut this = Self::new()?;
            this.only_remove(name);
            this.save()?;

            return Err(ComposerError::PhpVersion);
        }

        Ok(())
    }

    fn eprint_extensions(
        name: &str,
        origin_version: &str,
        list: &Vec<(String, String)>,
    ) -> Result<(), ComposerError> {
        if list.len() > 0 {
            for (i, item) in list.iter().enumerate() {
                eprintln!(
                    "{name}({}) -> .. -> {} need ext-{},it is missing from your system. Install or enable PHP's {} extension.",
                    origin_version, item.0, item.1,item.1
                );
                if i > 2 {
                    break;
                }
            }

            // rollback
            let mut this = Self::new()?;
            this.only_remove(name);
            this.save()?;

            return Err(ComposerError::PhpVersion);
        }

        Ok(())
    }

    pub async fn install(&mut self) -> Result<(), ComposerError> {
        let packages = self.get_lock().await?;

        packages.installing().await?;

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

    pub fn insert(&mut self, name: &str, version: &Option<String>) -> Result<(), ComposerError> {
        let star = String::from("*");
        let version = version.as_ref().unwrap_or(&star);

        self.require = match self.require.take() {
            Some(mut list) => {
                list.insert(name.to_owned(), version.to_owned());
                //list.sort_keys();

                Some(list)
            }
            None => {
                let mut map = IndexMap::new();
                map.insert(name.to_owned(), version.to_owned());
                //map.sort_keys();
                Some(map)
            }
        };

        Ok(())
    }

    fn only_remove(&mut self, name: &str) {
        let require = self.require.take();
        if let Some(mut list) = require {
            list.swap_remove(name);
            self.require = Some(list);
        }
    }
    pub async fn remove(&mut self, name: &str) -> Result<(), ComposerError> {
        let require = self.require.take();
        if let Some(mut list) = require {
            list.swap_remove(name);
            self.require = Some(list);
        }

        let new_lock = self.get_lock().await?;
        let old_lock = ComposerLock::from_file()?;
        let deleteing = old_lock.get_deleteing_packages(&new_lock)?;

        let vendor = Path::new("./vendor");
        for item in deleteing.iter() {
            remove_dir_all(vendor.join(item))?;
        }
        for item in deleteing.iter() {
            let path = vendor.join(item);
            if let Some(parent) = path.parent() {
                if let Ok(res) = has_files(parent) {
                    if !res {
                        remove_dir_all(parent)?;
                    }
                }
            }
        }

        fn has_files(path: &Path) -> Result<bool, std::io::Error> {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                if file_type.is_file() || file_type.is_dir() {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        new_lock.update_autoload_files()?;

        Ok(())
    }

    pub fn save(&self) -> Result<(), ComposerError> {
        let path = Path::new("./composer.json");
        let mut f = File::create(path)?;
        let content = serde_json::to_string_pretty(&self)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
}
