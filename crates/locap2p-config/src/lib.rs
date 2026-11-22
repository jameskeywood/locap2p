//! Application-facing configuration for a locap2p node.
//!
//! The configuration is intentionally serializable so that an application can
//! define it once and pass it to the framework without the framework needing
//! to read application-specific files itself.

use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Version of the application configuration contract.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// A complete configuration for a node in a locap2p application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkConfig {
    pub schema_version: u32,
    pub application: ApplicationConfig,
    pub identity: IdentityConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub connectivity: ConnectivityConfig,
    #[serde(default)]
    pub gossip: GossipConfig,
}

impl FrameworkConfig {
    /// Loads a JSON configuration file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let file = std::fs::File::open(path.as_ref()).map_err(ConfigError::Io)?;
        serde_json::from_reader(file).map_err(ConfigError::Json)
    }

    /// Serializes the configuration as pretty-printed JSON.
    pub fn to_json_pretty(&self) -> Result<String, ConfigError> {
        serde_json::to_string_pretty(self).map_err(ConfigError::Json)
    }
}

/// Application identity and the logical session being joined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub id: ApplicationId,
    pub session: SessionId,
}

/// A logical application namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApplicationId(String);

impl ApplicationId {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        validate_name("application id", &value, 64, true)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ApplicationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A game, lobby or other application-defined session.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        validate_name("session id", &value, 128, false)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Path to the persistent libp2p identity key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub path: PathBuf,
}

/// Local libp2p transport settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_addrs: Vec<String>,
    pub request_timeout_ms: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addrs: vec![
                "/ip4/0.0.0.0/tcp/0".to_owned(),
                "/ip4/0.0.0.0/udp/0/quic-v1".to_owned(),
            ],
            request_timeout_ms: 10_000,
        }
    }
}

/// Bootstrap gateway settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub bootstrap: Vec<ServiceEndpoint>,
}

/// Relay and direct-connection preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectivityConfig {
    #[serde(default)]
    pub relay: Vec<ServiceEndpoint>,
    #[serde(default)]
    pub reserve_relay: bool,
    #[serde(default = "default_true")]
    pub prefer_direct: bool,
    #[serde(default)]
    pub enable_dcutr: bool,
}

impl Default for ConnectivityConfig {
    fn default() -> Self {
        Self {
            relay: Vec::new(),
            reserve_relay: false,
            prefer_direct: true,
            enable_dcutr: false,
        }
    }
}

/// Anti-entropy gossip policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GossipConfig {
    pub sync_interval_ms: u64,
    pub fanout: usize,
    pub max_events_per_response: usize,
    #[serde(default = "default_membership_ttl_ms")]
    pub membership_ttl_ms: u64,
    #[serde(default = "default_expiry_grace_ms")]
    pub expiry_grace_ms: u64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            sync_interval_ms: 2_000,
            fanout: 3,
            max_events_per_response: 256,
            membership_ttl_ms: default_membership_ttl_ms(),
            expiry_grace_ms: default_expiry_grace_ms(),
        }
    }
}

impl GossipConfig {
    pub fn sync_interval(&self) -> Duration {
        Duration::from_millis(self.sync_interval_ms)
    }
}

fn default_membership_ttl_ms() -> u64 {
    10_000
}

fn default_expiry_grace_ms() -> u64 {
    5_000
}

/// A multiaddr for a bootstrap or relay service.
///
/// The address must include an explicit `/p2p/<peer-id>` component so that the
/// service identity is pinned by configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEndpoint(String);

impl ServiceEndpoint {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        validate_endpoint("service", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Errors returned while loading or validating configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read configuration: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse configuration: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

fn default_true() -> bool {
    true
}

fn validate_name(
    field: &str,
    value: &str,
    max_len: usize,
    allow_slash: bool,
) -> Result<(), ConfigError> {
    if value.is_empty() {
        return Err(ConfigError::Invalid(format!("{field} is empty")));
    }

    if value.len() > max_len {
        return Err(ConfigError::Invalid(format!(
            "{field} exceeds {max_len} characters"
        )));
    }

    let valid = value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || matches!(character, '-' | '_' | '.')
            || (allow_slash && character == '/')
    });

    if !valid {
        return Err(ConfigError::Invalid(format!(
            "{field} contains unsupported characters"
        )));
    }

    Ok(())
}

fn validate_endpoint(kind: &str, value: &str) -> Result<(), ConfigError> {
    if !value.starts_with('/') {
        return Err(ConfigError::Invalid(format!(
            "{kind} endpoint is not a multiaddr: {value}"
        )));
    }

    if !value.contains("/p2p/") || value.ends_with('/') {
        return Err(ConfigError::Invalid(format!(
            "{kind} endpoint must include a /p2p/<peer-id> component: {value}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> FrameworkConfig {
        FrameworkConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            application: ApplicationConfig {
                id: ApplicationId::new("locap2p-demo/1").unwrap(),
                session: SessionId::new("demo").unwrap(),
            },
            identity: IdentityConfig {
                path: PathBuf::from("state/demo/node.key"),
            },
            network: NetworkConfig::default(),
            discovery: DiscoveryConfig {
                bootstrap: vec![
                    ServiceEndpoint::new("/ip4/127.0.0.1/tcp/4002/p2p/12D3KooWTest").unwrap(),
                ],
            },
            connectivity: ConnectivityConfig {
                relay: vec![
                    ServiceEndpoint::new("/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWRelay").unwrap(),
                ],
                reserve_relay: true,
                prefer_direct: true,
                enable_dcutr: true,
            },
            gossip: GossipConfig {
                sync_interval_ms: 1_000,
                fanout: 3,
                max_events_per_response: 256,
                membership_ttl_ms: 10_000,
                expiry_grace_ms: 3_000,
            },
        }
    }

    #[test]
    fn config_round_trips_through_json() {
        let config = config();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: FrameworkConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, config);
    }

    #[test]
    fn endpoint_requires_peer_id() {
        assert!(ServiceEndpoint::new("/ip4/127.0.0.1/tcp/4002").is_err());
        assert!(ServiceEndpoint::new("/ip4/127.0.0.1/tcp/4002/p2p/12D3KooW").is_ok());
    }
}
