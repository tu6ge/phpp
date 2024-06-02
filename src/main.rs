use core::panic;

use app::App;
use clap::{Parser, Subcommand};
use error::ComposerError;
use package::P2;

mod app;
mod error;
mod package;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let app = App {};

    match &cli.command {
        Commands::Required { name } => {
            let res = P2::new(name.to_owned(), None).await;

            match res {
                Ok(()) => {}
                Err(ComposerError::NotFoundPackageName(_)) => {}
                Err(ComposerError::NotFoundPackage(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
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
