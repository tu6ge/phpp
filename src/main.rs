use app::App;
use clap::{Parser, Subcommand};
use config::GlobalConfig;
use error::ComposerError;
use json::Composer;
use package::P2;

mod app;
mod config;
mod error;
mod json;
mod package;

#[tokio::main]
async fn main() -> Result<(), ComposerError> {
    let cli = Cli::parse();

    let _app = App {};
    let mut composer = Composer::new()?;

    //println!("{:?}", composer);

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
        Commands::Config {
            global,
            unset,
            key,
            value1,
            value2,
        } => {
            if *global {
                let mut config = GlobalConfig::new().unwrap();
                if !unset {
                    if let Some(value1) = value1 {
                        config.set(key, value1, value2)?;
                    } else {
                        panic!("setting value is not empty");
                    }
                } else {
                    config.unset(key)?;
                }

                config.save()?;
            } else {
                if !unset {
                    if let Some(value1) = value1 {
                        composer.set(key, value1, value2)?;
                    } else {
                        panic!("setting value is not empty");
                    }
                } else {
                    composer.unset(key)?;
                }
                composer.save()?;
            }
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
    Config {
        /// setting global
        #[arg(short, long)]
        global: bool,

        #[arg(long)]
        unset: bool,

        key: String,
        value1: Option<String>,
        value2: Option<String>,
    },
}
