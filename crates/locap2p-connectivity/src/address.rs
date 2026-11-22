use libp2p::{Multiaddr, PeerId, multiaddr::Protocol};

pub(crate) struct AddressBook {
    local_peer_id: PeerId,
    listen: Vec<Multiaddr>,
    external: Vec<Multiaddr>,
    observed: Vec<Multiaddr>,
}

impl AddressBook {
    pub(crate) fn new(local_peer_id: PeerId) -> Self {
        Self {
            local_peer_id,
            listen: Vec::new(),
            external: Vec::new(),
            observed: Vec::new(),
        }
    }

    pub(crate) fn add_listen(&mut self, address: Multiaddr) {
        let address = normalize(address, self.local_peer_id);
        push_unique(&mut self.listen, address);
    }

    pub(crate) fn remove_listen(&mut self, address: &Multiaddr) {
        self.listen.retain(|candidate| candidate != address);
    }

    pub(crate) fn add_external(&mut self, address: Multiaddr) {
        let address = normalize(address, self.local_peer_id);
        push_unique(&mut self.external, address);
    }

    pub(crate) fn remove_external(&mut self, address: &Multiaddr) {
        self.external.retain(|candidate| candidate != address);
    }

    pub(crate) fn add_observed(&mut self, address: Multiaddr) {
        let address = normalize(address, self.local_peer_id);
        push_unique(&mut self.observed, address);
    }

    pub(crate) fn announceable_addresses(&self) -> Vec<Multiaddr> {
        let mut addresses: Vec<Multiaddr> = self
            .listen
            .iter()
            .chain(&self.external)
            .chain(&self.observed)
            .filter(|address| is_usable_address(address))
            .cloned()
            .collect();

        addresses.sort();
        addresses.dedup();
        addresses
    }

    pub(crate) fn rank_peer_addresses(
        &self,
        addresses: impl IntoIterator<Item = Multiaddr>,
        prefer_direct: bool,
    ) -> Vec<Multiaddr> {
        let mut addresses: Vec<Multiaddr> =
            addresses.into_iter().filter(is_usable_address).collect();

        addresses.sort();
        addresses.dedup();
        addresses.sort_by_key(|address| {
            let relayed = is_relayed(address);
            if prefer_direct { relayed } else { !relayed }
        });
        addresses
    }
}

fn normalize(mut address: Multiaddr, local_peer_id: PeerId) -> Multiaddr {
    if matches!(address.iter().last(), Some(Protocol::P2p(peer_id)) if peer_id == local_peer_id) {
        address.pop();
    }
    address
}

fn push_unique(addresses: &mut Vec<Multiaddr>, address: Multiaddr) {
    if !addresses.contains(&address) {
        addresses.push(address);
    }
}

pub(crate) fn is_usable_address(address: &Multiaddr) -> bool {
    !address.iter().any(|protocol| match protocol {
        Protocol::Ip4(address) => address.is_unspecified(),
        Protocol::Ip6(address) => address.is_unspecified(),
        _ => false,
    })
}

fn is_relayed(address: &Multiaddr) -> bool {
    address
        .iter()
        .any(|protocol| matches!(protocol, Protocol::P2pCircuit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;

    fn peer_id() -> PeerId {
        Keypair::generate_ed25519().public().to_peer_id()
    }

    fn address(value: &str) -> Multiaddr {
        value.parse().unwrap()
    }

    #[test]
    fn announced_addresses_include_observed_and_skip_unspecified() {
        let local_peer_id = peer_id();
        let mut book = AddressBook::new(local_peer_id);

        book.add_listen(address("/ip4/0.0.0.0/tcp/4000"));
        book.add_listen(address("/ip4/127.0.0.1/tcp/4000"));
        book.add_observed(address(&format!(
            "/ip4/203.0.113.5/tcp/54321/p2p/{local_peer_id}"
        )));

        let addresses = book.announceable_addresses();

        assert_eq!(
            addresses,
            vec![
                address("/ip4/127.0.0.1/tcp/4000"),
                address("/ip4/203.0.113.5/tcp/54321"),
            ]
        );
    }

    #[test]
    fn announced_addresses_are_deduplicated() {
        let local_peer_id = peer_id();
        let mut book = AddressBook::new(local_peer_id);

        book.add_listen(address("/ip4/127.0.0.1/tcp/4000"));
        book.add_external(address("/ip4/127.0.0.1/tcp/4000"));
        book.add_observed(address("/ip4/127.0.0.1/tcp/4000"));

        assert_eq!(
            book.announceable_addresses(),
            vec![address("/ip4/127.0.0.1/tcp/4000")]
        );
    }

    #[test]
    fn peer_addresses_are_ranked_by_preference() {
        let book = AddressBook::new(peer_id());
        let direct = address("/ip4/127.0.0.1/tcp/4000");
        let relayed = address(
            "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/p2p-circuit",
        );

        assert_eq!(
            book.rank_peer_addresses(vec![relayed.clone(), direct.clone()], true),
            vec![direct.clone(), relayed.clone()]
        );
        assert_eq!(
            book.rank_peer_addresses(vec![direct.clone(), relayed.clone()], false),
            vec![relayed, direct]
        );
    }

    #[test]
    fn unusable_peer_addresses_are_filtered() {
        let book = AddressBook::new(peer_id());

        assert!(
            book.rank_peer_addresses(vec![address("/ip4/0.0.0.0/tcp/4000")], true)
                .is_empty()
        );
    }
}
