use std::{
    collections::{HashMap, HashSet},
    fs::File,
    future::Future,
    io::Write,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use semver::VersionReq;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::error::ComposerError;

const PACKAGE_URL: &'static str = "https://repo.packagist.org/p2/";
const CACHE_DIR: &'static str = ".cache/composer2";

#[derive(Debug, Deserialize, Clone)]
pub struct P2 {
    pub(crate) packages: HashMap<String, Vec<Version>>,
    #[serde(skip)]
    names: HashSet<String>,
}

impl P2 {
    pub fn new(
        name: String,
        version: Option<String>,
        list: Arc<Mutex<Vec<Version>>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ComposerError>> + Send>> {
        Box::pin(async move {
            let exists = Self::file_exists(&name);
            if exists {
                return Ok(());
            }
            let _ = sleep(Duration::from_millis(200));

            let json = Self::down(&name).await?;

            Self::save(&name, &json).unwrap();

            let tree: P2 = serde_json::from_str(&json)
                .expect(&format!("parse json failed, package: {}", name));

            let version_list = tree
                .packages
                .get(&name)
                .ok_or(ComposerError::NotFoundPackageName(name.to_owned()))?;

            let mut info = &version_list[0];
            if let Some(req) = version {
                for item in version_list.iter() {
                    if Self::semver_check(&name, &req, &item.version) {
                        info = item;
                        break;
                    }
                }
            }

            list.lock().unwrap().push(info.clone());

            println!("download {}({})", name, info.version);
            let deps = &info.require;
            if let Some(Require::Map(deps)) = deps {
                for (dep_name, version) in deps.iter() {
                    //println!("source: {}, deps: {}, version:{}", name, dep_name, version);
                    if dep_name == "php" {
                        continue;
                    } else if matches!(dep_name.find("ext-"), Some(0)) {
                        continue;
                    } else {
                        P2::new(dep_name.to_owned(), Some(version.to_owned()), list.clone())
                            .await?;
                    }
                }
            }

            Ok(())
        })
    }

    pub async fn down(name: &str) -> Result<String, ComposerError> {
        let mut url = String::from(PACKAGE_URL);
        url.push_str(name);
        url.push_str(".json");

        let response = reqwest::Client::new().get(url).send().await?;

        if !response.status().is_success() {
            return Err(ComposerError::NotFoundPackage(name.to_owned()));
        }

        let json = response.text().await?;

        Ok(json)
    }

    pub fn file_exists(name: &str) -> bool {
        use dirs::home_dir;
        let cache_dir = home_dir().unwrap().join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");

        let name_dir = name.replace("/", "-");
        let filename = format!("provider-{}.json", name_dir);
        let final_path = repo_dir.join(filename);

        final_path.exists()
    }

    pub fn save(name: &str, content: &str) -> Result<(), ComposerError> {
        use dirs::home_dir;
        use std::fs::{create_dir_all, File};

        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");
        create_dir_all(&repo_dir)?;

        let name_dir = name.replace("/", "-");
        let filename = format!("provider-{}.json", name_dir);
        let final_path = repo_dir.join(filename);

        let mut f = File::create(final_path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }

    pub fn clear() -> Result<(), ComposerError> {
        use dirs::home_dir;
        use std::fs::remove_dir_all;
        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");
        remove_dir_all(repo_dir)?;

        // TODO other dir

        Ok(())
    }

    pub fn semver_check(name: &str, req: &str, version: &str) -> bool {
        let mut chars = version.chars();
        let first_char = chars.next();
        let version = if let Some('v') = first_char {
            &version[1..]
        } else if let Some('V') = first_char {
            &version[1..]
        } else {
            &version[..]
        };
        let chars = version.chars();
        let dot_count = chars.filter(|&c| c == '.').count();
        let version = if dot_count == 1 {
            format!("{}.0", version)
        } else {
            format!("{}", version)
        };

        let version = semver::Version::parse(&version)
            .expect(&format!("{}[{}] is not a valid version", name, version));
        if let Some(_) = req.find("||") {
            let mut parts = Vec::new();
            for item in req.split("||") {
                parts.push(item);
            }
            for item in parts.into_iter().rev() {
                let req = item.trim();
                let req = VersionReq::parse(req).unwrap();

                if req.matches(&version) {
                    return true;
                }
            }

            false
        } else if let Some(_) = req.find("|") {
            let mut parts = Vec::new();
            for item in req.split("|") {
                parts.push(item);
            }
            for item in parts.into_iter().rev() {
                let req = item.trim();
                let req = VersionReq::parse(req).unwrap();

                if req.matches(&version) {
                    return true;
                }
            }

            false
        } else {
            let version_req = VersionReq::parse(req).unwrap();

            version_req.matches(&version)
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ComposerLock {
    packages: Vec<Version>,
}

impl ComposerLock {
    pub fn new(versions: Arc<Mutex<Vec<Version>>>) -> Self {
        let ls = versions.lock().unwrap();

        let mut packages = Vec::new();
        for item in ls.iter() {
            if let Some(_) = item.name {
                packages.push(item.clone());
            }
        }

        packages.sort_by(|a, b| a.name.cmp(&b.name));

        Self { packages }
    }

    pub fn json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }

    pub fn save_file(&self) {
        let path = Path::new("./composer.lock");
        let mut f = File::create(path).unwrap();
        f.write(self.json().as_bytes()).unwrap();
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Version {
    pub name: Option<String>,
    pub(crate) version: String,
    pub(crate) version_normalized: String,
    source: Option<Source>,
    pub(crate) dist: Option<Dist>,
    // autoload
    pub(crate) require: Option<Require>,

    #[serde(rename = "require-dev")]
    pub(crate) requireDev: Option<Require>,

    autoload: Option<AutoloadEnum>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
struct Source {
    #[serde(rename = "type")]
    _type: String,

    url: String,
    reference: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Dist {
    pub(crate) url: String,
    #[serde(rename = "type")]
    pub(crate) _type: String,
    pub(crate) reference: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
enum Require {
    Map(HashMap<String, String>),
    String(String),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
enum AutoloadEnum {
    Psr(Autoload),
    String(String),
    Null(),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
struct Autoload {
    #[serde(rename = "psr-4")]
    psr4: Option<HashMap<String, PsrValue>>,

    #[serde(rename = "psr-0")]
    psr0: Option<HashMap<String, PsrValue>>,

    #[serde(rename = "classmap")]
    classMap: Option<AutoLoadClassmap>,

    files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
enum PsrValue {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
enum AutoLoadClassmap {
    Array(Vec<String>),
    Array2(Vec<Vec<String>>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deser() {
        let mut url = String::from(PACKAGE_URL);
        let name = "guzzlehttp/guzzle";
        url.push_str(name);
        url.push_str(".json");

        let json = reqwest::Client::new()
            .get(url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let res: P2 = serde_json::from_str(&json).unwrap();

        println!("{res:?}");
    }

    #[test]
    fn test_semver() {
        assert!(P2::semver_check("name", "^7.0| ^8.0", "7.2.3"));
        assert!(P2::semver_check("name", "^7.0| ^8.0", "8.2.3"));
        assert!(!P2::semver_check("name", "^7.0| ^8.0", "9.2.3"));
        assert!(!P2::semver_check("name", "^7.0|| ^8.0", "9.2.3"));
        assert!(P2::semver_check("name", "^7.0| ^8.0", "8.0"));
        //assert!(P2::semver_check("5.1.0-RC1", "5.1.0-RC1"));

        let chars = "1.2.4".chars();
        let dot_count = chars.filter(|&c| c == '.').count();
        assert_eq!(dot_count, 2);
    }
}
