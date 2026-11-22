use locap2p_config::{ApplicationId, SessionId};
use serde::{Deserialize, Serialize};

pub const SEED_PROTOCOL: &str = "/locap2p/seed/1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedRequest {
    pub application: ApplicationId,
    pub session: SessionId,
    pub addresses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedResponse {
    pub seeds: Vec<SeedPeer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedPeer {
    pub peer_id: String,
    pub addresses: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let request = SeedRequest {
            application: ApplicationId::new("locap2p-demo/1"),
            session: SessionId::new("demo"),
            addresses: vec!["/ip4/127.0.0.1/tcp/4000".to_owned()],
        };

        let encoded = serde_json::to_vec(&request).unwrap();
        let decoded: SeedRequest = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, request);
    }

    #[test]
    fn response_round_trips() {
        let response = SeedResponse {
            seeds: vec![SeedPeer {
                peer_id: "12D3KooWTest".to_owned(),
                addresses: vec!["/ip4/127.0.0.1/tcp/4001".to_owned()],
            }],
        };

        let encoded = serde_json::to_vec(&response).unwrap();
        let decoded: SeedResponse = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, response);
    }
}
