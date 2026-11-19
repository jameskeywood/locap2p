//! Wire types for the locap2p bootstrap protocol.

use locap2p_config::{ApplicationId, SessionId};
use serde::{Deserialize, Serialize};

/// Versioned protocol used to announce to and discover from a bootstrap node.
pub const BOOTSTRAP_PROTOCOL: &str = "/locap2p/bootstrap/1";

/// Registers a node in a session and requests the current session members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapRequest {
    pub application: ApplicationId,
    pub session: SessionId,
    pub addresses: Vec<String>,
    pub ttl_ms: u64,
}

/// Current members of a session, excluding the requester.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapResponse {
    pub peers: Vec<PeerInfo>,
}

/// A discovered peer and the addresses it advertised.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let request = BootstrapRequest {
            application: ApplicationId::new("locap2p-demo/1").unwrap(),
            session: SessionId::new("demo").unwrap(),
            addresses: vec!["/ip4/127.0.0.1/tcp/4000".to_owned()],
            ttl_ms: 10_000,
        };

        let encoded = serde_json::to_vec(&request).unwrap();
        let decoded: BootstrapRequest = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, request);
    }

    #[test]
    fn response_round_trips() {
        let response = BootstrapResponse {
            peers: vec![PeerInfo {
                peer_id: "12D3KooWTest".to_owned(),
                addresses: vec!["/ip4/127.0.0.1/tcp/4001".to_owned()],
            }],
        };

        let encoded = serde_json::to_vec(&response).unwrap();
        let decoded: BootstrapResponse = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, response);
    }
}
