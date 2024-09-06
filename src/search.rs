//! search

use reqwest::header::USER_AGENT;
use serde::Deserialize;

use crate::{error::ComposerError, package::MY_USER_AGENT};

pub struct Search {
    keyword: String,
}

impl Search {
    pub fn new(keyword: &str) -> Search {
        Search {
            keyword: keyword.to_string(),
        }
    }

    pub async fn search(&self) -> Result<(), ComposerError> {
        let response: SearchResult = reqwest::Client::new()
            .get(format!(
                "https://packagist.org/search.json?q={}&per_page=15",
                self.keyword
            ))
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await?
            .json()
            .await?;

        for item in response.results {
            println!(
                "\x1b]8;;{}\x07{:30}\x1b]8;;\x07 {}",
                item.url, item.name, item.description
            );
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    name: String,
    description: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    //total: u32,
    results: Vec<SearchItem>,
}
