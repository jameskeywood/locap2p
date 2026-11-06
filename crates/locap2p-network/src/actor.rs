use std::collections::HashSet;

use futures::StreamExt;
use libp2p::{
    PeerId, Swarm, gossipsub, mdns, noise,
    swarm::{SwarmEvent, dial_opts::DialOpts},
    tcp, yamux,
};
use locap2p_core::NodeId;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    config::NetworkConfig,
    error::NetworkError,
    event::NetworkEvent,
    handle::{NetworkCommand, NetworkEvents, NetworkHandle},
};

const COMMAND_CHANNEL_CAPACITY: usize = 64;
const EVENT_CHANNEL_CAPACITY: usize = 256;

pub(crate) async fn start(
    config: NetworkConfig,
) -> Result<(NetworkHandle, NetworkEvents), NetworkError> {
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .with_quic()
        .with_behaviour(|key| Behaviour::new(key, config.discovery))
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .build();

    for address in &config.listen_addrs {
        swarm
            .listen_on(address.clone())
            .map_err(|error| NetworkError::Listen(error.to_string()))?;
    }

    let node_id = NodeId::from(swarm.local_peer_id().to_string());
    let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
    let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);

    let handle = NetworkHandle::new(node_id, command_tx);
    let events = NetworkEvents::new(event_rx);

    let actor = NetworkActor {
        swarm,
        command_rx,
        event_tx,
        discovered_peers: HashSet::new(),
    };
    tokio::spawn(actor.run());

    Ok((handle, events))
}

struct NetworkActor {
    swarm: Swarm<Behaviour>,
    command_rx: mpsc::Receiver<NetworkCommand>,
    event_tx: mpsc::Sender<NetworkEvent>,
    discovered_peers: HashSet<PeerId>,
}

impl NetworkActor {
    async fn run(mut self) {
        loop {
            tokio::select! {
                command = self.command_rx.recv() => {
                    let Some(command) = command else {
                        break;
                    };

                    if self.handle_command(command) {
                        break;
                    }
                }
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }
            }
        }
    }

    fn handle_command(&mut self, command: NetworkCommand) -> bool {
        match command {
            NetworkCommand::Subscribe { topic, reply } => {
                let result = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .subscribe(&gossipsub::IdentTopic::new(topic))
                    .map(|_| ())
                    .map_err(|error| NetworkError::Subscribe(error.to_string()));

                let _ = reply.send(result);
                false
            }
            NetworkCommand::Publish { topic, data, reply } => {
                let result = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(gossipsub::IdentTopic::new(topic), data)
                    .map(|_| ())
                    .map_err(|error| NetworkError::Publish(error.to_string()));

                let _ = reply.send(result);
                false
            }
            NetworkCommand::Shutdown { reply } => {
                let _ = reply.send(());
                true
            }
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, address) in peers {
                    if !self.discovered_peers.insert(peer_id) {
                        continue;
                    }

                    info!(%peer_id, %address, "discovered peer");
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);

                    let dial = DialOpts::peer_id(peer_id).addresses(vec![address]).build();
                    if let Err(error) = self.swarm.dial(dial) {
                        debug!(%peer_id, %error, "failed to dial discovered peer");
                    }

                    emit(
                        &self.event_tx,
                        NetworkEvent::PeerDiscovered(node_id(peer_id)),
                    )
                    .await;
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _address) in peers {
                    if !self.discovered_peers.remove(&peer_id) {
                        continue;
                    }

                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                    emit(&self.event_tx, NetworkEvent::PeerExpired(node_id(peer_id))).await;
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            })) => {
                let source = message.source.unwrap_or(propagation_source);
                emit(
                    &self.event_tx,
                    NetworkEvent::Message {
                        source: node_id(source),
                        topic: message.topic.to_string(),
                        data: message.data,
                    },
                )
                .await;
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "listening");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                debug!(%peer_id, "connection established");
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!(%peer_id, "connection closed");
            }
            _ => {}
        }
    }
}

async fn emit(event_tx: &mpsc::Sender<NetworkEvent>, event: NetworkEvent) {
    if event_tx.send(event).await.is_err() {
        debug!("network event receiver dropped");
    }
}

fn node_id(peer_id: PeerId) -> NodeId {
    NodeId::from(peer_id.to_string())
}
