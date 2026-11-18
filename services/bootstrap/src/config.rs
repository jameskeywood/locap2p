use std::{fs, path::PathBuf};

use serde::Deserialize;

/// Static configuration for a bootstrap service.
#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    /// Addresses the service listens on.
    pub listen_addrs: Vec<String>,
    /// Addresses advertised to clients. If empty, listen_addrs are used.
    #[serde(default)]
    pub advertise_addrs: Vec<String>,
    /// Persistent identity key used to derive the advertised peer ID.
    pub identity_path: PathBuf,
    /// Maximum number of session members returned/stored per session.
    #[serde(default = "default_max_peers_per_session")]
    pub max_peers_per_session: usize,
}

impl BootstrapConfig {
    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.listen_addrs.is_empty() {
            anyhow::bail!("bootstrap service requires at least one listen address");
        }

        if self
            .advertise_addrs
            .iter()
            .any(|address| !address.starts_with('/'))
        {
            anyhow::bail!("advertised addresses must be multiaddrs");
        }

        if self.max_peers_per_session == 0 {
            anyhow::bail!("max_peers_per_session must be non-zero");
        }

        Ok(())
    }
}

fn default_max_peers_per_session() -> usize {
    256
}
