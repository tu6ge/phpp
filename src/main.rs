use core::panic;
use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

use app::App;
use clap::{Parser, Subcommand};
use error::ComposerError;
use package::{ComposerLock, Version, P2};

mod app;
mod error;
mod package;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let app = App {};

    let list = Vec::new();
    let versions = Arc::new(Mutex::new(list));
    let version_hash_set = HashSet::new();
    let version_hash = Arc::new(Mutex::new(version_hash_set));

    match &cli.command {
        Commands::Required { name } => {
            let res = P2::new(
                name.to_owned(),
                None,
                versions.clone(),
                version_hash.clone(),
            )
            .await
            .expect("download error");

            let packages = ComposerLock::new(versions);
            packages.save_file();

            packages.down_package().await.expect("download dist failed");
        }
        Commands::Clear => {
            P2::clear().expect("clear dir failed");
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Required { name: String },
    Clear,
}
