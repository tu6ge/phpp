//! disponse P2 and parse composer.lock file

use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir_all, read_to_string, File},
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use dirs::home_dir;
use reqwest::header::USER_AGENT;
use semver::{Comparator, Prerelease, VersionReq};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::{
    autoload::{FilesData, Psr4Data, StaticData},
    error::ComposerError,
};

const CACHE_DIR: &str = ".cache/phpp";
const MY_USER_AGENT: &str = "tu6ge/phpp";

#[derive(Debug, Deserialize, Clone)]
pub struct P2 {
    pub(crate) packages: HashMap<String, Vec<Version>>,
}

impl P2 {
    pub fn down_all(
        name: String,
        version: Option<String>,
        ctx: Arc<Mutex<Context>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ComposerError>> + Send>> {
        Box::pin(async move {
            {
                if ctx.lock().unwrap().hash_set.get(&name).is_some() {
                    return Ok(());
                }
            }

            let json = {
                let url = ctx.lock().unwrap().p2_url.clone();

                let exists = Self::file_exists(&name, &url)?;
                if exists {
                    Self::read_file(&name, &url)?
                } else {
                    sleep(Duration::from_millis(200)).await;

                    let json = match Self::down(&name, &url).await {
                        Ok(json) => json,
                        Err(ComposerError::NotFoundPackage(_)) => return Ok(()),
                        Err(e) => return Err(e),
                    };

                    Self::save(&name, &json, &url)?;
                    json
                }
            };

            #[allow(clippy::expect_fun_call)]
            let tree: P2 = serde_json::from_str(&json)
                .expect(&format!("parse json failed, package: {}", name));

            let version_list = tree.packages.get(&name).expect("no found package name");
            //.ok_or(ComposerError::NotFoundPackageName(name.to_owned()))?;

            let mut info = version_list[0].clone();
            if let Some(req) = version {
                for item in version_list.iter() {
                    if Self::semver_check(&name, &req, &item.version)? {
                        info = item.clone();
                        break;
                    }
                }
            } else {
                // find last stable version
                for item in version_list.iter() {
                    let version = item.semver()?;
                    if version.pre == Prerelease::EMPTY {
                        info = item.clone();
                        break;
                    }
                }
                ctx.lock().unwrap().first_package = Some(info.clone());
            }
            info.name = Some(name.to_owned());

            {
                ctx.lock().unwrap().versions.push(info.clone());
                ctx.lock().unwrap().hash_set.insert(name.to_owned());
            }

            println!("  - Locking {}({})", name, info.version);
            let deps = &info.require;
            if let Some(Require::Map(deps)) = deps {
                for (dep_name, version) in deps.iter() {
                    //println!("source: {}, deps: {}, version:{}", name, dep_name, version);
                    if dep_name == "php" {
                        let mut ctx = ctx.lock().unwrap();
                        let php_version = &ctx.php_version;

                        if !Self::semver_check(&name, &version, php_version)? {
                            ctx.php_version_error
                                .push((format!("{}({})", name, info.version), version.to_owned()));
                        }
                    } else if matches!(dep_name.find("ext-"), Some(0)) {
                        let ext = dep_name.replace("ext-", "");
                        let mut ctx = ctx.lock().unwrap();
                        let exists = ctx.exists_extension(&ext);
                        if !exists {
                            ctx.php_extensions_error
                                .push((format!("{}({})", name, info.version), ext.to_owned()));
                        }
                    } else {
                        P2::down_all(dep_name.to_owned(), Some(version.to_owned()), ctx.clone())
                            .await?;
                    }
                }
            }

            Ok(())
        })
    }

    pub async fn down(name: &str, p2_url: &str) -> Result<String, ComposerError> {
        let mut url = String::from(p2_url);
        url.push_str(name);
        url.push_str(".json");

        let response = reqwest::Client::new()
            .get(url)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ComposerError::NotFoundPackage(name.to_owned()));
        }

        let json = response.text().await?;

        Ok(json)
    }

    pub fn file_exists(name: &str, p2_url: &str) -> Result<bool, ComposerError> {
        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");
        let p2_url = String::from(p2_url).replace(":", "-");
        let p2_url = p2_url.replace("/", "-");
        let repo_dir = repo_dir.join(p2_url);
        create_dir_all(&repo_dir)?;

        let name_dir = name.replace('/', "-");
        let filename = format!("provider-{}.json", name_dir);
        let final_path = repo_dir.join(filename);

        Ok(final_path.exists())
    }

    pub fn save(name: &str, content: &str, p2_url: &str) -> Result<(), ComposerError> {
        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");
        let p2_url = String::from(p2_url).replace(":", "-");
        let p2_url = p2_url.replace("/", "-");
        let repo_dir = repo_dir.join(p2_url);
        create_dir_all(&repo_dir)?;

        let name_dir = name.replace('/', "-");
        let filename = format!("provider-{}.json", name_dir);
        let final_path = repo_dir.join(filename);

        let mut f = File::create(final_path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
    pub fn read_file(name: &str, p2_url: &str) -> Result<String, ComposerError> {
        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");
        let p2_url = String::from(p2_url).replace(":", "-");
        let p2_url = p2_url.replace("/", "-");
        let repo_dir = repo_dir.join(p2_url);
        create_dir_all(&repo_dir)?;

        let name_dir = name.replace('/', "-");
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

    pub fn semver_check(_name: &str, req: &str, version: &str) -> Result<bool, ComposerError> {
        let mut chars = version.chars();
        let first_char = chars.next();
        let version = if let Some('v') = first_char {
            &version[1..]
        } else if let Some('V') = first_char {
            &version[1..]
        } else {
            version
        };

        let chars = version.chars();
        let dot_count = chars.filter(|&c| c == '.').count();
        let version = if dot_count == 1 {
            format!("{}.0", version)
        } else {
            version.to_string()
        };

        let mut req_chars = req.chars();
        let req_first_char = req_chars.next();
        let req = if let Some('v') = req_first_char {
            &req[1..]
        } else if let Some('V') = req_first_char {
            &req[1..]
        } else {
            req
        };
        let req = req.replace("\\u003E", ">");
        let req = req.replace("\\u003C", "<");

        //println!("now req: {req}");

        #[allow(clippy::expect_fun_call)]
        let version = semver::Version::parse(&version)?;
        if req.contains("||") {
            let mut parts = Vec::new();
            for item in req.split("||") {
                parts.push(item);
            }
            for item in parts.into_iter().rev() {
                let req = item.trim();
                let req = VersionReq::parse(req)?;

                if req.matches(&version) {
                    return Ok(true);
                }
            }

            Ok(false)
        } else if req.find('|').is_some() {
            let mut parts = Vec::new();
            for item in req.split('|') {
                parts.push(item);
            }
            for item in parts.into_iter().rev() {
                let req = item.trim();
                let req = VersionReq::parse(req)?;

                if req.matches(&version) {
                    return Ok(true);
                }
            }

            Ok(false)
        } else if req.contains('-') {
            let mut parts = Vec::new();
            for item in req.split('-') {
                parts.push(item);
            }
            debug_assert!(parts.len() == 2);
            let req = format!(">={}", parts[0].trim());
            let comp1 = Comparator::parse(&req)?;
            let req = format!("<={}", parts[1].trim());
            let comp2 = Comparator::parse(&req)?;
            let req = VersionReq {
                comparators: vec![comp1, comp2],
            };

            if req.matches(&version) {
                return Ok(true);
            }

            Ok(false)
        } else if req.contains('>') && req.contains('<') {
            let mut parts = Vec::new();
            for item in req.split(' ') {
                parts.push(item);
            }
            debug_assert!(parts.len() == 2);
            let req = parts[0].trim();
            let comp1 = Comparator::parse(&req)?;
            let req = parts[1].trim();
            let comp2 = Comparator::parse(&req)?;
            let req = VersionReq {
                comparators: vec![comp1, comp2],
            };

            if req.matches(&version) {
                return Ok(true);
            }

            Ok(false)
        } else {
            let version_req = VersionReq::parse(&req)?;

            Ok(version_req.matches(&version))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComposerLock {
    pub packages: Vec<Version>,
}

impl ComposerLock {
    pub fn new(versions: Arc<Mutex<Context>>) -> Self {
        let ls = &versions.lock().unwrap().versions;

        let mut packages = Vec::new();
        for item in ls.iter() {
            if item.name.is_some() {
                packages.push(item.clone());
            }
        }

        packages.sort_by(|a, b| a.name.cmp(&b.name));

        Self { packages }
    }

    pub fn from_file() -> Result<Self, ComposerError> {
        let path = Path::new("./composer.lock");
        let content = read_to_string(path)?;

        let this: Self = serde_json::from_str(&content)?;

        Ok(this)
    }

    pub fn get_deleteing_packages(
        &self,
        new_lock: &ComposerLock,
    ) -> Result<HashSet<String>, ComposerError> {
        let mut this_set = HashSet::new();

        for item in self.packages.iter() {
            this_set.insert(item.name.as_ref().unwrap().to_owned());
        }
        let mut new_set = HashSet::new();
        for item in new_lock.packages.iter() {
            new_set.insert(item.name.as_ref().unwrap().to_owned());
        }

        let difference: HashSet<_> = this_set.difference(&new_set).cloned().collect();

        Ok(difference)
    }

    pub fn json(&self) -> Result<String, ComposerError> {
        let res = serde_json::to_string_pretty(&self)?;

        Ok(res)
    }

    pub async fn installing(&self) -> Result<(), ComposerError> {
        self.save_file()?;

        self.down_package().await?;

        self.install_package()?;

        self.write_psr4()?;

        self.write_installed_versions()?;

        self.write_class_loader()?;
        self.write_autoload_real()?;
        self.write_autoload_static()?;
        self.write_platform_check()?;
        self.write_autoload_classmap()?;
        self.write_autoload()?;

        self.write_autoload_files()?;

        Ok(())
    }
    pub fn update_autoload_files(&self) -> Result<(), ComposerError> {
        self.save_file()?;
        self.write_psr4()?;

        self.write_installed_versions()?;

        self.write_class_loader()?;
        self.write_autoload_real()?;
        self.write_autoload_static()?;
        self.write_platform_check()?;
        self.write_autoload_classmap()?;
        self.write_autoload()?;

        self.write_autoload_files()?;

        Ok(())
    }

    pub fn save_file(&self) -> Result<(), ComposerError> {
        let path = Path::new("./composer.lock");
        let mut f = File::create(path)?;
        f.write_all(self.json()?.as_bytes())?;

        Ok(())
    }

    async fn down_package(&self) -> Result<(), ComposerError> {
        use sha1::{Digest, Sha1};

        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("files");
        create_dir_all(&repo_dir)?;

        for item in self.packages.iter() {
            let dist = &item.dist.as_ref().expect("not found dist field");

            let name = item.name.as_ref().expect("not found name");

            let package_dir = repo_dir.join(name.clone());
            create_dir_all(&package_dir)?;

            let mut hasher = Sha1::new();
            hasher.update(item.version.as_bytes());
            let sha1 = hasher.finalize();

            let mut file_name = hex::encode(sha1);
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

    fn install_package(&self) -> Result<(), ComposerError> {
        use sha1::{Digest, Sha1};

        let cache_dir = home_dir()
            .ok_or(ComposerError::NotFoundHomeDir)?
            .join(CACHE_DIR);
        let repo_dir = cache_dir.join("files");

        let vendor_dir = Path::new("./vendor");
        create_dir_all(vendor_dir)?;

        for item in self.packages.iter() {
            let name = item.name.as_ref().expect("not found name");

            println!("  - Installing {}({})", name, item.version);

            let vendor_item = vendor_dir.join(name.clone());
            create_dir_all(&vendor_item)?;

            let package_dir = repo_dir.join(name.clone());

            let mut hasher = Sha1::new();
            hasher.update(item.version.as_bytes());
            let sha1 = hasher.finalize();

            let mut file_name = hex::encode(sha1);
            file_name.push_str(".zip");

            let file_path = package_dir.join(file_name);

            let f = File::open(&file_path)?;

            let mut archive = zip::ZipArchive::new(f)?;

            for i in 1..archive.len() {
                let mut file = archive.by_index(i)?;
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
                            create_dir_all(p)?;
                        }
                    }

                    let mut outfile = File::create(&final_path)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }
        }
        Ok(())
    }

    fn write_psr4(&self) -> Result<(), ComposerError> {
        let mut data = Psr4Data::new()?;
        data.append_lock(&self);

        data.write()
    }

    fn write_installed_versions(&self) -> Result<(), ComposerError> {
        let content = include_str!("../asset/InstalledVersions.php");

        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("InstalledVersions.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
    fn write_class_loader(&self) -> Result<(), ComposerError> {
        let content = include_str!("../asset/ClassLoader.php");

        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("ClassLoader.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
    fn write_autoload_real(&self) -> Result<(), ComposerError> {
        let content = include_str!("../asset/autoload_real.php");

        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload_real.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
    fn write_platform_check(&self) -> Result<(), ComposerError> {
        let content = include_str!("../asset/platform_check.php");

        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("platform_check.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
    fn write_autoload(&self) -> Result<(), ComposerError> {
        let content = include_str!("../asset/autoload.php");

        let path = Path::new("./vendor/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }
    fn write_autoload_classmap(&self) -> Result<(), ComposerError> {
        let content = include_str!("../asset/autoload_classmap.php");

        let path = Path::new("./vendor/composer");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload_classmap.php");
        let mut f = File::create(path)?;
        f.write_all(content.as_bytes())?;

        Ok(())
    }

    fn write_autoload_files(&self) -> Result<(), ComposerError> {
        let mut files = FilesData::new()?;
        files.append_lock(&self);
        files.write()
    }

    fn write_autoload_static(&self) -> Result<(), ComposerError> {
        let mut files = FilesData::new()?;
        files.append_lock(&self);

        let mut psr4 = Psr4Data::new()?;
        psr4.append_lock(&self);

        let static_data = StaticData::from(&files, &psr4);

        static_data.write()
    }

    pub fn find_version(&self, name: &str) -> Option<&Version> {
        for item in self.packages.iter() {
            if let Some(ref n) = item.name {
                if n == name {
                    return Some(item);
                }
            }
        }
        None
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
    pub(crate) autoload: Option<AutoloadEnum>,
}

impl Version {
    pub fn semver(&self) -> Result<semver::Version, ComposerError> {
        let mut chars = self.version.chars();
        let first_char = chars.next();
        let version = if let Some('v') = first_char {
            &self.version[1..]
        } else if let Some('V') = first_char {
            &self.version[1..]
        } else {
            &self.version[..]
        };
        let version = semver::Version::parse(version)?;

        Ok(version)
    }
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
pub(crate) enum AutoloadEnum {
    Psr(Autoload),
    String(String),
    Null(),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub(crate) struct Autoload {
    #[serde(rename = "psr-4")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) psr4: Option<HashMap<String, PsrValue>>,

    #[serde(rename = "psr-0")]
    #[serde(skip_serializing_if = "Option::is_none")]
    psr0: Option<HashMap<String, PsrValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    classmap: Option<AutoLoadClassmap>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub(crate) enum PsrValue {
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
    pub(crate) first_package: Option<Version>,
    pub(crate) php_extensions: Vec<String>,
    pub(crate) php_version: String,
    pub(crate) php_version_error: Vec<(String, String)>,
    pub(crate) php_extensions_error: Vec<(String, String)>,
    pub p2_url: String,
}

impl Context {
    pub fn new() -> Result<Self, ComposerError> {
        Ok(Context {
            php_version: Self::php_version()?,
            php_extensions: Self::php_extensions(),
            ..Default::default()
        })
    }

    fn php_version() -> Result<String, ComposerError> {
        //return Ok("7.0".to_owned());
        // PHP 8.1.2-1ubuntu2.17 (cli) (built: May  1 2024 10:10:07) (NTS)
        // PHP 7.4.3 (cli) (built: Feb 18 2020 17:29:57) ( NTS Visual C++ 2017 x64 )
        let output = Command::new("php")
            .arg("-v")
            .output()
            .map_err(|_| ComposerError::GetPhpVersionFailed)?;

        if output.status.success() {
            let stdout = std::str::from_utf8(&output.stdout)
                .map_err(|_| ComposerError::GetPhpVersionFailed)?;

            let re = regex::Regex::new(r"PHP (\d+\.\d+\.\d+)")
                .map_err(|_| ComposerError::GetPhpVersionFailed)?;
            if let Some(caps) = re.captures(stdout) {
                if let Some(version) = caps.get(1) {
                    return Ok(version.as_str().to_owned());
                }
            }
        }

        Err(ComposerError::GetPhpVersionFailed)
    }

    fn php_extensions() -> Vec<String> {
        let output = Command::new("php").arg("-m").output().unwrap();

        // 将输出转换为字符串
        let stdout = std::str::from_utf8(&output.stdout).unwrap();

        // 将输出按行分割并存储到 Vec<String> 中
        let extensions: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
        extensions
    }

    fn exists_extension(&self, extension: &str) -> bool {
        for item in self.php_extensions.iter() {
            if item == extension {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[tokio::test]
    // async fn test_deser() {
    //     let mut url = String::from("");
    //     let name = "guzzlehttp/guzzle";
    //     url.push_str(name);
    //     url.push_str(".json");

    //     let json = reqwest::Client::new()
    //         .get(url)
    //         .send()
    //         .await
    //         .unwrap()
    //         .text()
    //         .await
    //         .unwrap();

    //     let res: P2 = serde_json::from_str(&json).unwrap();

    //     println!("{res:?}");
    // }

    #[test]
    fn test_semver() {
        assert!(P2::semver_check("name", "^7.0| ^8.0", "7.2.3").unwrap());
        assert!(P2::semver_check("name", "^7.0| ^8.0", "8.2.3").unwrap());
        assert!(!P2::semver_check("name", "^7.0| ^8.0", "9.2.3").unwrap());
        assert!(!P2::semver_check("name", "^7.0|| ^8.0", "9.2.3").unwrap());
        assert!(P2::semver_check("name", "^7.0| ^8.0", "8.0").unwrap());
        assert!(P2::semver_check("name", ">=7.4", "8.0").unwrap());
        assert!(!P2::semver_check("name", ">=8.1", "8.0").unwrap());
        //assert!(P2::semver_check("5.1.0-RC1", "5.1.0-RC1"));

        let chars = "1.2.4".chars();
        let dot_count = chars.filter(|&c| c == '.').count();
        assert_eq!(dot_count, 2);

        let version = semver::Version::parse("5.0.8").unwrap();
        assert!(version.pre == Prerelease::EMPTY);
    }

    // #[test]
    // fn test_php_version() {
    //     let v = Context::php_version().unwrap();

    //     assert_eq!(v, "8.1.2".to_owned());
    // }
}
