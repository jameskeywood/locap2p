use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Deserialize;

/// Static configuration for a bootstrap seed service.
#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    /// Addresses the service listens on.
    pub listen_addrs: Vec<String>,
    /// Addresses advertised to clients. If empty, listen_addrs are used.
    #[serde(default)]
    pub advertise_addrs: Vec<String>,
    /// Persistent identity key used to derive the advertised peer ID.
    pub identity_path: PathBuf,
    /// How long a seed remains available for introduction.
    #[serde(default = "default_seed_ttl_secs")]
    pub seed_ttl_secs: u64,
    /// Maximum seeds retained per session.
    #[serde(default = "default_max_seeds_per_session")]
    pub max_seeds_per_session: usize,
    /// Maximum seeds returned in one response.
    #[serde(default = "default_max_seeds_returned")]
    pub max_seeds_returned: usize,
}

impl BootstrapConfig {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn seed_ttl(&self) -> Duration {
        Duration::from_secs(self.seed_ttl_secs)
    }
}

fn default_seed_ttl_secs() -> u64 {
    90
}

fn default_max_seeds_per_session() -> usize {
    32
}

fn default_max_seeds_returned() -> usize {
    8
}
