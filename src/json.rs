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
    config::{GlobalConfig, Packagist, Repositories},
    error::ComposerError,
    io::ErrWriter,
    package::{ComposerLock, Context, P2},
};

#[cfg(test)]
mod tests;

const PACKAGE_URL: &str = "https://repo.packagist.org/";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Composer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) require: Option<IndexMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    repositories: Option<Repositories>,
}

impl Composer {
    pub fn new() -> Result<Composer, ComposerError> {
        let path = Path::new("./composer.json");

        let mut content = String::new();

        match read_to_string(path) {
            Ok(c) => content = c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut file = File::create(path)?;
                file.write_all(b"{\"require\":{}}")?;
                content = String::from("{\"require\":{}}");
            }
            Err(e) => return Err(ComposerError::Io(e)),
        }

        let cp: Self = serde_json::from_str(&content)?;

        Ok(cp)
    }

    pub async fn get_lock(
        &self,
        stderr: &mut dyn ErrWriter,
        ctx: Arc<Mutex<Context>>,
    ) -> Result<ComposerLock, ComposerError> {
        if let Some(ref list) = self.require {
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

                Self::eprint_php_version(name, &origin_version, &c.php_version_error, stderr)?;
                Self::eprint_extensions(name, &origin_version, &c.php_extensions_error, stderr)?;
            }
        }

        Ok(ComposerLock::new(ctx))
    }

    /// php version is not satisfy, return failure
    fn eprint_php_version(
        name: &str,
        origin_version: &str,
        list: &Vec<(String, String)>,
        stderr: &mut dyn ErrWriter,
    ) -> Result<(), ComposerError> {
        if list.len() > 0 {
            for (i, item) in list.iter().enumerate() {
                stderr.write(&format!(
                    "{name}({}) -> .. -> {} need PHP version is {}",
                    origin_version, item.0, item.1
                ));
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
        stderr: &mut dyn ErrWriter,
    ) -> Result<(), ComposerError> {
        if list.len() > 0 {
            for (i, item) in list.iter().enumerate() {
                stderr.write(&format!(
                    "{name}({}) -> .. -> {} need ext-{},it is missing from your system. Install or enable PHP's {} extension.",
                    origin_version, item.0, item.1,item.1
                ));
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

    pub async fn install(
        &mut self,
        name: &str,
        stderr: &mut dyn ErrWriter,
    ) -> Result<(), ComposerError> {
        let p2_url = self.get_package_url()?;
        let mut context = Context::new()?;

        context.p2_url = p2_url;

        let ctx = Arc::new(Mutex::new(context));
        let packages = self.get_lock(stderr, ctx).await?;

        packages.installing().await?;

        if !name.is_empty() {
            if let Some(version) = packages.find_version(name) {
                self.set_version(name, &version.version);
                self.save()?;
            }
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
    pub async fn remove(
        &mut self,
        name: &str,
        stderr: &mut dyn ErrWriter,
    ) -> Result<(), ComposerError> {
        let p2_url = self.get_package_url()?;
        let mut context = Context::new()?;

        context.p2_url = p2_url;

        let ctx = Arc::new(Mutex::new(context));

        let require = self.require.take();
        if let Some(mut list) = require {
            list.swap_remove(name);
            self.require = Some(list);
        }

        let new_lock = self.get_lock(stderr, ctx).await?;
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

    pub fn set(
        &mut self,
        unset: bool,
        key: &str,
        value1: &Option<String>,
        value2: &Option<String>,
    ) -> Result<(), ComposerError> {
        match key {
            "repo.packagist" => {
                if !unset {
                    if let (Some(value1), Some(value2)) = (value1, value2) {
                        self.set_repo(value1, value2)?;
                    }
                } else {
                    self.repositories = None;
                }
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

    pub fn get_package_url(&self) -> Result<String, ComposerError> {
        // PACKAGE_URL
        let mut url = String::from(PACKAGE_URL);
        if let Some(repositories) = &self.repositories {
            url = repositories.packagist.url.to_owned();
        } else {
            let config = GlobalConfig::new()?;
            if let Some(repositories) = config.repositories {
                url = repositories.packagist.url.to_owned();
            }
        }

        url.push_str("/p2/");

        Ok(url)
    }
}
