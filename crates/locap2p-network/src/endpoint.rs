use std::str::FromStr;

use libp2p::{Multiaddr, PeerId, multiaddr::Protocol};
use locap2p_config::ServiceEndpoint;

use crate::error::NetworkError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedEndpoint {
    pub(crate) peer_id: PeerId,
    pub(crate) address: Multiaddr,
}

pub(crate) fn parse_endpoints(
    kind: &str,
    endpoints: &[ServiceEndpoint],
) -> Result<Vec<ParsedEndpoint>, NetworkError> {
    endpoints
        .iter()
        .map(|endpoint| parse_endpoint(kind, endpoint.as_str()))
        .collect()
}

fn parse_endpoint(kind: &str, value: &str) -> Result<ParsedEndpoint, NetworkError> {
    let mut address = Multiaddr::from_str(value).map_err(|error| {
        NetworkError::Config(format!("{kind} endpoint is not a multiaddr: {error}"))
    })?;

    let peer_id = match address.iter().last() {
        Some(Protocol::P2p(peer_id)) => peer_id,
        _ => {
            return Err(NetworkError::Config(format!(
                "{kind} endpoint must end with /p2p/<peer-id>: {value}"
            )));
        }
    };

    address.pop();

    Ok(ParsedEndpoint { peer_id, address })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_strips_peer_id() {
        let endpoint = parse_endpoint(
            "bootstrap",
            "/ip4/127.0.0.1/tcp/4002/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
        )
        .unwrap();

        assert_eq!(endpoint.address.to_string(), "/ip4/127.0.0.1/tcp/4002");
    }

    #[test]
    fn rejects_endpoint_without_peer_id() {
        assert!(parse_endpoint("relay", "/ip4/127.0.0.1/tcp/4001").is_err());
    }
}
