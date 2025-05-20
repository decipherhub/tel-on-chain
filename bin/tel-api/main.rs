use anyhow::Result;
use clap::Parser;
use tel_core::config;
use tel_api::api;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config/default.toml")]
    config: String,
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
    let config = config::load_config(&cli.config)?;
    api::run_server(config).await?;

    Ok(())
} 