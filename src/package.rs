use std::{
    collections::{HashMap, HashSet},
    future::Future,
    io::Write,
    pin::Pin,
    time::Duration,
};

use semver::VersionReq;
use serde::Deserialize;
use tokio::time::sleep;

use crate::error::ComposerError;

const PACKAGE_URL: &'static str = "https://repo.packagist.org/p2/";
const CACHE_DIR: &'static str = ".cache/composer2";

#[derive(Debug, Deserialize)]
pub struct P2 {
    pub(crate) packages: HashMap<String, Vec<Version>>,
    #[serde(skip)]
    names: HashSet<String>,
}

impl P2 {
    pub fn new(
        name: String,
        version: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ComposerError>> + Send>> {
        Box::pin(async move {
            let exists = Self::file_exists(&name);
            if exists {
                return Ok(());
            }
            let json = Self::down(&name).await?;

            Self::save(&name, &json)?;

            let tree: P2 = serde_json::from_str(&json)?;

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
                        P2::new(dep_name.to_owned(), Some(version.to_owned())).await?;
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

#[derive(Debug, Deserialize)]
pub struct Version {
    //name: String,
    pub(crate) version: String,
    pub(crate) version_normalized: String,
    pub(crate) dist: Option<Dist>,
    // autoload
    pub(crate) require: Option<Require>,
    // require-dev
}

#[derive(Debug, Deserialize)]
pub struct Dist {
    pub(crate) url: String,
    #[serde(rename = "type")]
    pub(crate) _type: String,
    pub(crate) reference: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Require {
    Map(HashMap<String, String>),
    String(String),
}

#[cfg(test)]
mod tests {
    use semver::{BuildMetadata, Prerelease, VersionReq};

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
