use app::App;
use clap::{Parser, Subcommand};
use json::Composer;
use package::P2;

mod app;
mod error;
mod json;
mod package;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let _app = App {};

    match &cli.command {
        Commands::Require { name } => {
            let mut composer = Composer::new().unwrap();
            composer.insert(name).unwrap();
            composer.save();

            composer.install().await.unwrap();
        }
        Commands::Install => {
            let composer = Composer::new().unwrap();

            composer.install().await.unwrap();
        }
        Commands::Clear => {
            P2::clear().expect("clear dir failed");
        }
        Commands::Remove { name } => {
            let mut composer = Composer::new().unwrap();
            composer.remove(name).unwrap();
            composer.save();
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
    Require { name: String },
    Install,
    Clear,
    Remove { name: String },
}
