use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use locap2p_bootstrap::{BootstrapConfig, run};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "locap2p-bootstrap")]
struct Cli {
    /// Path to the static bootstrap service configuration.
    #[arg(long)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = BootstrapConfig::load(&cli.config)
        .with_context(|| format!("failed to load {}", cli.config.display()))?;

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
