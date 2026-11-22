//! Application-facing configuration for a locap2p node.
//!
//! The configuration is intentionally serializable so that an application can
//! define it once and pass it to the framework without the framework needing
//! to read application-specific files itself.

use std::{fmt, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

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
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
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
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
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
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
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

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> FrameworkConfig {
        FrameworkConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            application: ApplicationConfig {
                id: ApplicationId::new("locap2p-demo/1"),
                session: SessionId::new("demo"),
            },
            identity: IdentityConfig {
                path: PathBuf::from("state/demo/node.key"),
            },
            network: NetworkConfig::default(),
            discovery: DiscoveryConfig {
                bootstrap: vec![ServiceEndpoint::new(
                    "/ip4/127.0.0.1/tcp/4002/p2p/12D3KooWTest",
                )],
            },
            connectivity: ConnectivityConfig {
                relay: vec![ServiceEndpoint::new(
                    "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWRelay",
                )],
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
}
