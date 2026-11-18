use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use locap2p_relay::{RelayConfig, print_info, run};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "locap2p-relay")]
struct Cli {
    /// Path to the static relay service configuration.
    #[arg(long)]
    config: PathBuf,

    /// Print the service endpoints needed in the application config and exit.
    #[arg(long)]
    print_info: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = RelayConfig::load(&cli.config)
        .with_context(|| format!("failed to load {}", cli.config.display()))?;

    if cli.print_info {
        print_info(&config)?;
        return Ok(());
    }

    init_tracing();
    run(config).await
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
}
