use app::App;
use clap::{Parser, Subcommand};
use package::P2;

mod app;
mod package;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let app = App {};

    match &cli.command {
        Commands::Required { name } => {
            P2::new(name.to_owned()).await;
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
}
