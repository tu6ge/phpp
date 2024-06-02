use app::App;
use clap::{Parser, Subcommand};
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
            let _ = P2::new(name.to_owned(), None).await;
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
