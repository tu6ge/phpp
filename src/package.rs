use std::{
    collections::{HashMap, HashSet},
    future::Future,
    io::Write,
    path::Path,
    pin::Pin,
    time::Duration,
};

use serde::Deserialize;
use tokio::time::sleep;

const PACKAGE_URL: &'static str = "https://repo.packagist.org/p2/";
const CACHE_DIR: &'static str = ".cache/composer2";

#[derive(Debug, Deserialize)]
pub struct P2 {
    pub(crate) packages: HashMap<String, Vec<Version>>,
    #[serde(skip)]
    names: HashSet<String>,
}

impl P2 {
    pub fn new(name: String) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            let exists = Self::file_exists(&name);
            if exists {
                return;
            }
            let json = Self::down(&name).await;
            if let Err(_) = json {
                return;
            }
            let json = json.unwrap();

            Self::save(&name, &json);
            println!("download {} success", name);

            sleep(Duration::from_millis(100)).await;

            let tree: P2 = serde_json::from_str(&json).unwrap();

            let verson_list = tree.packages.get(&name).unwrap();
            let info = &verson_list[0];
            let deps = &info.require;
            if let Some(Require::Map(deps)) = deps {
                for (name, version) in deps.iter() {
                    if name == "php" {
                        continue;
                    } else if matches!(name.find("ext-"), Some(0)) {
                        continue;
                    } else {
                        P2::new(name.to_owned()).await;
                    }
                }
            }
        })
    }

    pub async fn down(name: &str) -> Result<String, ()> {
        let mut url = String::from(PACKAGE_URL);
        url.push_str(name);
        url.push_str(".json");

        let response = reqwest::Client::new().get(url).send().await.unwrap();

        if !response.status().is_success() {
            return Err(());
        }

        let json = response.text().await.unwrap();

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

    pub fn save(name: &str, content: &str) {
        use dirs::home_dir;
        use std::fs::{create_dir_all, File};

        let cache_dir = home_dir().unwrap().join(CACHE_DIR);
        let repo_dir = cache_dir.join("repo");
        create_dir_all(&repo_dir).unwrap();

        let name_dir = name.replace("/", "-");
        let filename = format!("provider-{}.json", name_dir);
        let final_path = repo_dir.join(filename);

        let mut f = File::create(final_path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
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
}
