use libp2p::Multiaddr;

const DEFAULT_LISTEN_ADDRS: [&str; 2] = ["/ip4/0.0.0.0/tcp/0", "/ip4/0.0.0.0/udp/0/quic-v1"];

/// Configuration for the network layer.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Addresses the node listens on.
    pub listen_addrs: Vec<Multiaddr>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addrs: DEFAULT_LISTEN_ADDRS
                .iter()
                .filter_map(|address| address.parse().ok())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_listens_locally() {
        let config = NetworkConfig::default();

        assert_eq!(config.listen_addrs.len(), 2);
    }
}
