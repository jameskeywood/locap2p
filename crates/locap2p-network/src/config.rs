use libp2p::Multiaddr;

const DEFAULT_LISTEN_ADDRS: [&str; 2] = ["/ip4/0.0.0.0/tcp/0", "/ip4/0.0.0.0/udp/0/quic-v1"];

/// Configuration for the network actor.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub listen_addrs: Vec<Multiaddr>,
    pub discovery: DiscoveryMode,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addrs: DEFAULT_LISTEN_ADDRS
                .iter()
                .filter_map(|address| address.parse().ok())
                .collect(),
            discovery: DiscoveryMode::default(),
        }
    }
}

/// Peer discovery strategy.
///
/// This is the seam for future bootstrap, static peer and relay support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscoveryMode {
    #[default]
    Mdns,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_listens_locally_with_mdns() {
        let config = NetworkConfig::default();

        assert_eq!(config.listen_addrs.len(), 2);
        assert_eq!(config.discovery, DiscoveryMode::Mdns);
    }
}
