use app::App;
use clap::{Parser, Subcommand};
use error::ComposerError;
use json::Composer;
use package::P2;

mod app;
mod error;
mod json;
mod package;

#[tokio::main]
async fn main() -> Result<(), ComposerError> {
    let cli = Cli::parse();

    let _app = App {};
    let mut composer = Composer::new()?;

    match &cli.command {
        Commands::Require { name, version } => {
            composer.insert(name, version)?;
            composer.save()?;

            composer.install().await?;
        }
        Commands::Install => {
            composer.install().await?;
        }
        Commands::Clear => {
            P2::clear().expect("clear dir failed");
        }
        Commands::Remove { name } => {
            composer.remove(name).await?;
            composer.save()?;
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Require {
        name: String,
        version: Option<String>,
    },
    Install,
    Clear,
    Remove {
        name: String,
    },
}
