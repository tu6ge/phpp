use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir_all, read_to_string, File},
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use dirs::home_dir;
use reqwest::header::USER_AGENT;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::error::ComposerError;

const PACKAGE_URL: &'static str = "https://repo.packagist.org/p2/";
const CACHE_DIR: &'static str = ".cache/composer2";
const MY_USER_AGENT: &'static str = "tu6ge/phpp";

#[derive(Debug, Deserialize, Clone)]
pub struct P2 {
    pub(crate) packages: HashMap<String, Vec<Version>>,
}

impl P2 {
    pub fn new(
        name: String,
        version: Option<String>,
        ctx: Arc<Mutex<Context>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ComposerError>> + Send>> {
        Box::pin(async move {
            if let Some(_) = ctx.lock().unwrap().hash_set.get(&name) {
                return Ok(());
            }

            let exists = Self::file_exists(&name);
            let json = if exists {
                Self::read_file(&name)?
            } else {
                let _ = sleep(Duration::from_millis(200));

                let json = match Self::down(&name).await {
                    Ok(json) => json,
                    Err(ComposerError::NotFoundPackage(_)) => return Ok(()),
                    Err(e) => return Err(e),
                };

                Self::save(&name, &json).unwrap();
                json
            };

            let tree: P2 = serde_json::from_str(&json)
                .expect(&format!("parse json failed, package: {}", name));

            let version_list = tree.packages.get(&name).expect("abc");
            //.ok_or(ComposerError::NotFoundPackageName(name.to_owned()))?;

            let mut info = &version_list[0];
            if let Some(req) = version {
                for item in version_list.iter() {
                    if Self::semver_check(&name, &req, &item.version) {
                        info = item;
                        break;
                    }
                }
            }

            ctx.lock().unwrap().versions.push(info.clone());
            ctx.lock().unwrap().hash_set.insert(name.to_owned());

            println!("  - Locking {}({})", name, info.version);
            let deps = &info.require;
            if let Some(Require::Map(deps)) = deps {
                for (dep_name, version) in deps.iter() {
                    //println!("source: {}, deps: {}, version:{}", name, dep_name, version);
                    if dep_name == "php" {
                        // TODO
                        continue;
                    } else if matches!(dep_name.find("ext-"), Some(0)) {
                        // TODO
                        continue;
                    } else {
                        P2::new(dep_name.to_owned(), Some(version.to_owned()), ctx.clone()).await?;
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
        let cache_dir = home_dir().unwrap().join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");

        let name_dir = name.replace("/", "-");
        let filename = format!("provider-{}.json", name_dir);
        let final_path = repo_dir.join(filename);

        final_path.exists()
    }

    pub fn save(name: &str, content: &str) -> Result<(), ComposerError> {
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
    pub fn read_file(name: &str) -> Result<String, ComposerError> {
        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");

        let name_dir = name.replace("/", "-");
        let filename = format!("provider-{}.json", name_dir);
        let final_path = repo_dir.join(filename);

        let content = read_to_string(final_path)?;

        Ok(content)
    }

    pub fn clear() -> Result<(), ComposerError> {
        use std::fs::remove_dir_all;
        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        remove_dir_all(cache_dir.join("repo"))?;

        remove_dir_all(cache_dir.join("files"))?;

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
    pub fn new(versions: Arc<Mutex<Context>>) -> Self {
        let ls = &versions.lock().unwrap().versions;

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

    pub async fn down_package(&self) -> Result<(), ComposerError> {
        use sha1::{Digest, Sha1};

        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("files");
        create_dir_all(&repo_dir)?;

        for item in self.packages.iter() {
            let dist = &item.dist.as_ref().unwrap();

            let name = item.name.as_ref().expect(&format!("not found name"));

            let package_dir = repo_dir.join(name.clone());
            create_dir_all(&package_dir)?;

            let mut hasher = Sha1::new();
            hasher.update(item.version.as_bytes());
            let sha1 = hasher.finalize();

            let mut file_name = hex::encode(&sha1);
            file_name.push_str(".zip");

            let file_path = package_dir.join(file_name);

            if file_path.exists() {
                continue;
            }
            let content = reqwest::Client::new()
                .get(dist.url.clone())
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await?
                .bytes()
                .await?;

            let mut f = File::create(file_path)?;
            f.write_all(&content)?;

            //break;
            println!("  - Downloading {}({})", name, item.version);
        }

        Ok(())
    }

    pub fn install_package(&self) -> Result<(), ComposerError> {
        use sha1::{Digest, Sha1};

        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("files");

        let vendor_dir = Path::new("./vendor");
        create_dir_all(&vendor_dir)?;

        for item in self.packages.iter() {
            let name = item.name.as_ref().expect(&format!("not found name"));

            println!("  - Installing {}({})", name, item.version);

            let vendor_item = vendor_dir.join(name.clone());
            create_dir_all(&vendor_item)?;

            let package_dir = repo_dir.join(name.clone());

            let mut hasher = Sha1::new();
            hasher.update(item.version.as_bytes());
            let sha1 = hasher.finalize();

            let mut file_name = hex::encode(&sha1);
            file_name.push_str(".zip");

            let file_path = package_dir.join(file_name);

            let f = File::open(&file_path)?;

            let mut archive = zip::ZipArchive::new(f)?;

            for i in 1..archive.len() {
                let mut file = archive.by_index(i).unwrap();
                let outpath = match file.enclosed_name() {
                    Some(path) => path.to_owned(),
                    None => continue,
                };

                let outpath: PathBuf = outpath.iter().skip(1).collect();
                let final_path = vendor_item.join(outpath);

                if file.is_dir() {
                    create_dir_all(&final_path)?;
                } else {
                    if let Some(p) = final_path.parent() {
                        if !p.exists() {
                            create_dir_all(p).unwrap();
                        }
                    }

                    let mut outfile = File::create(&final_path).unwrap();
                    std::io::copy(&mut file, &mut outfile).unwrap();
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Version {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub(crate) version: String,
    pub(crate) version_normalized: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Source>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dist: Option<Dist>,

    // autoload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) require: Option<Require>,

    #[serde(rename = "require-dev")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) require_dev: Option<Require>,

    #[serde(skip_serializing_if = "Option::is_none")]
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
pub(crate) enum Require {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    psr4: Option<HashMap<String, PsrValue>>,

    #[serde(rename = "psr-0")]
    #[serde(skip_serializing_if = "Option::is_none")]
    psr0: Option<HashMap<String, PsrValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    classmap: Option<AutoLoadClassmap>,

    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Default)]
pub(crate) struct Context {
    versions: Vec<Version>,
    hash_set: HashSet<String>,
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
