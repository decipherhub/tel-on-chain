use clap::Parser;
use tel_core::config;
use tel_indexer::run_indexer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "config/config.toml")]
    config: String,

    /// Optional DEX name to index
    #[arg(short, long)]
    dex: Option<String>,

    /// Optional pool address to index
    #[arg(short, long)]
    pair: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    // Parse command line arguments
    let args = Args::parse();

    // Load config
    let config = config::load_config(&args.config)?;

    // Run indexer
    run_indexer(config, args.dex, args.pair).await?;

    Ok(())
} 