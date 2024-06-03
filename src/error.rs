use std::fmt::{self, Display};

use thiserror::Error;
use zip::result::ZipError;

#[derive(Debug, Error)]
pub enum ComposerError {
    Reqwest(#[from] reqwest::Error),

    Io(#[from] std::io::Error),

    Json(#[from] serde_json::Error),

    NotFoundPackage(String),

    NotFoundHomeDir,

    Zip(#[from] ZipError),
}

impl Display for ComposerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "composer2 error".fmt(f)
    }
}
