use libp2p::{
    dcutr,
    identity::Keypair,
    relay,
    swarm::{NetworkBehaviour, behaviour::toggle::Toggle},
};

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub(crate) relay_client: relay::client::Behaviour,
    pub(crate) dcutr: Toggle<dcutr::Behaviour>,
}

impl Behaviour {
    pub fn new(key: &Keypair, relay_client: relay::client::Behaviour, enable_dcutr: bool) -> Self {
        let dcutr = enable_dcutr
            .then(|| dcutr::Behaviour::new(key.public().to_peer_id()))
            .into();

        Self {
            relay_client,
            dcutr,
        }
    }
}
