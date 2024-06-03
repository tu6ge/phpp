use std::sync::{Arc, Mutex};

use app::App;
use clap::{Parser, Subcommand};
use package::{ComposerLock, Context, P2};

mod app;
mod error;
mod package;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let _app = App {};

    let ctx = Arc::new(Mutex::new(Context::default()));

    match &cli.command {
        Commands::Require { name } => {
            let _ = P2::new(name.to_owned(), None, ctx.clone())
                .await
                .expect("download error");

            let packages = ComposerLock::new(ctx);
            packages.save_file();

            packages.down_package().await.expect("download dist failed");

            packages.install_package().expect("install package failed");
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
    Require { name: String },
    Clear,
}
