use std::{fs, path::PathBuf, time::Duration};

use serde::Deserialize;

/// Static configuration for a relay service.
#[derive(Debug, Clone, Deserialize)]
pub struct RelayConfig {
    /// Addresses the relay listens on.
    pub listen_addrs: Vec<String>,
    /// Persistent identity key used to derive the relay peer ID.
    pub identity_path: PathBuf,
    /// Addresses advertised to the swarm as external addresses.
    #[serde(default)]
    pub advertise_addrs: Vec<String>,
    /// Circuit relay resource limits.
    #[serde(default)]
    pub limits: RelayLimits,
}

impl RelayConfig {
    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.listen_addrs.is_empty() {
            anyhow::bail!("relay service requires at least one listen address");
        }

        if self
            .advertise_addrs
            .iter()
            .any(|address| !address.starts_with('/'))
        {
            anyhow::bail!("advertised addresses must be multiaddrs");
        }

        if self.limits.max_reservations == 0 || self.limits.max_circuits == 0 {
            anyhow::bail!("relay reservation and circuit limits must be non-zero");
        }

        if self.limits.max_circuit_bytes == 0 || self.limits.max_circuit_duration_ms == 0 {
            anyhow::bail!("relay circuit limits must be non-zero");
        }

        Ok(())
    }
}

/// Resource limits applied by the relay server.
#[derive(Debug, Clone, Deserialize)]
pub struct RelayLimits {
    #[serde(default = "default_max_reservations")]
    pub max_reservations: usize,
    #[serde(default = "default_max_reservations_per_peer")]
    pub max_reservations_per_peer: usize,
    #[serde(default = "default_max_circuits")]
    pub max_circuits: usize,
    #[serde(default = "default_max_circuits_per_peer")]
    pub max_circuits_per_peer: usize,
    #[serde(default = "default_max_circuit_duration_ms")]
    pub max_circuit_duration_ms: u64,
    #[serde(default = "default_max_circuit_bytes")]
    pub max_circuit_bytes: u64,
}

impl Default for RelayLimits {
    fn default() -> Self {
        Self {
            max_reservations: default_max_reservations(),
            max_reservations_per_peer: default_max_reservations_per_peer(),
            max_circuits: default_max_circuits(),
            max_circuits_per_peer: default_max_circuits_per_peer(),
            max_circuit_duration_ms: default_max_circuit_duration_ms(),
            max_circuit_bytes: default_max_circuit_bytes(),
        }
    }
}

impl RelayLimits {
    pub fn max_circuit_duration(&self) -> Duration {
        Duration::from_millis(self.max_circuit_duration_ms)
    }
}

fn default_max_reservations() -> usize {
    512
}

fn default_max_reservations_per_peer() -> usize {
    16
}

fn default_max_circuits() -> usize {
    256
}

fn default_max_circuits_per_peer() -> usize {
    16
}

fn default_max_circuit_duration_ms() -> u64 {
    10 * 60 * 1000
}

fn default_max_circuit_bytes() -> u64 {
    1024 * 1024
}
