use core::panic;
use std::{
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

    let mut list = Vec::new();
    let versions = Arc::new(Mutex::new(list));

    match &cli.command {
        Commands::Required { name } => {
            let res = P2::new(name.to_owned(), None, versions.clone()).await;

            match res {
                Ok(()) => {}
                Err(ComposerError::NotFoundPackageName(_)) => {}
                Err(ComposerError::NotFoundPackage(_)) => {}
                Err(e) => panic!("{:?}", e),
            }

            let packages = ComposerLock::new(versions);
            packages.save_file();
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
