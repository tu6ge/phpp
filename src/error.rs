use std::{
    fmt::{self, Display},
    num::ParseIntError,
};

use reqwest::header::{InvalidHeaderValue, ToStrError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComposerError {
    Reqwest(#[from] reqwest::Error),

    Io(#[from] std::io::Error),

    Json(#[from] serde_json::Error),

    NotFoundPackage(String),

    NotFoundHomeDir,

    NotFoundPackageName(String),
}

impl Display for ComposerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "composer2 error".fmt(f)
    }
}
