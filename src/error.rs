use std::fmt::{self, Display};

use thiserror::Error;
use zip::result::ZipError;

#[derive(Debug, Error)]
pub enum ComposerError {
    Reqwest(#[from] reqwest::Error),

    Io(#[from] std::io::Error),

    Json(#[from] serde_json::Error),

    #[allow(dead_code)]
    NotFoundPackage(String),

    NotFoundHomeDir,

    Zip(#[from] ZipError),

    Semver(#[from] semver::Error),

    GetPhpVersionFailed,

    PhpVersion,
}

impl Display for ComposerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "composer2 error".fmt(f)
    }
}
