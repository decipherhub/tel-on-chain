use anyhow::Result;
use clap::{Parser, Subcommand};
use tel_core::config;
use tel_api::{api, indexer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the API server
    Api {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Run the indexer to collect and process DEX data
    Index {
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,

        #[arg(short, long)]
        dex: Option<String>,

        #[arg(short, long)]
        pair: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting tel-api");

    let cli = Cli::parse();

    match cli.command {
        Commands::Api {
            config: config_path,
        } => {
            let config = config::load_config(&config_path)?;
            api::run_server(config).await?;
        }
        Commands::Index {
            config: config_path,
            dex,
            pair,
        } => {
            let config = config::load_config(&config_path)?;
            indexer::run_indexer(config, dex, pair).await?;
        }
    }

    Ok(())
} 