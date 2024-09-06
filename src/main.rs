use clap::{Parser, Subcommand};
use config::GlobalConfig;
use error::ComposerError;
use io::StderrWriter;
use json::Composer;
use package::P2;

mod autoload;
mod config;
mod error;
mod io;
mod json;
mod package;

#[tokio::main]
async fn main() -> Result<(), ComposerError> {
    let cli = Cli::parse();

    let mut composer = Composer::new()?;
    let mut std_err = StderrWriter {};

    //println!("{:?}", composer);

    match &cli.command {
        Commands::Require { name, version } => {
            composer.insert(name, version)?;
            composer.save()?;

            composer.install(&name, &mut std_err).await?;
        }
        Commands::Install => {
            composer.install("", &mut std_err).await?;
        }
        Commands::Clear => {
            P2::clear().expect("clear dir failed");
        }
        Commands::Remove { name } => {
            composer.remove(name, &mut std_err).await?;
            composer.save()?;
        }
        Commands::DumpAutoload => {
            composer.dump_autoload()?;
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
                config.set(*unset, key, value1, value2)?;
                config.save()?;
            } else {
                composer.set(*unset, key, value1, value2)?;
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
    DumpAutoload,
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
