use std::{
    collections::{BTreeSet, HashSet, VecDeque},
    time::Duration,
};

use locap2p_core::{Event, EventId, EventSet, NodeId};
use locap2p_network::{Network, NetworkEvent, RequestId};
use tokio::time::{Interval, MissedTickBehavior, interval};
use tracing::{debug, warn};

use crate::{Gossip, GossipError, GossipEvent, codec, message::SyncMessage};

/// Configuration for the anti-entropy gossip layer.
#[derive(Debug, Clone)]
pub struct AntiEntropyConfig {
    /// How often a node pulls missing events from its peers.
    pub sync_interval: Duration,
    /// Number of peers contacted per pull or eager push.
    pub fanout: usize,
    /// Maximum number of events returned in a single pull response.
    pub max_events_per_response: usize,
}

impl Default for AntiEntropyConfig {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_secs(2),
            fanout: 3,
            max_events_per_response: 256,
        }
    }
}

#[derive(Default)]
struct PeerSelector {
    cursor: usize,
}

impl PeerSelector {
    fn select(&mut self, peers: &BTreeSet<NodeId>, fanout: usize) -> Vec<NodeId> {
        if peers.is_empty() || fanout == 0 {
            return Vec::new();
        }

        let peers: Vec<NodeId> = peers.iter().cloned().collect();
        let start = self.cursor % peers.len();
        let count = fanout.min(peers.len());
        let selected = peers
            .iter()
            .cycle()
            .skip(start)
            .take(count)
            .cloned()
            .collect();

        self.cursor = (start + count) % peers.len();
        selected
    }
}

/// Gossip strategy based on periodic pull anti-entropy and eager push.
pub struct AntiEntropyGossip {
    network: Network,
    store: EventSet,
    config: AntiEntropyConfig,
    peers: BTreeSet<NodeId>,
    pending: VecDeque<GossipEvent>,
    ticker: Interval,
    selector: PeerSelector,
}

impl AntiEntropyGossip {
    /// Creates an anti-entropy gossip instance.
    pub fn start(network: Network, config: AntiEntropyConfig) -> Result<Self, GossipError> {
        if config.sync_interval.is_zero() {
            return Err(GossipError::InvalidConfig("sync interval must be non-zero"));
        }

        let mut ticker = interval(config.sync_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Ok(Self {
            network,
            store: EventSet::default(),
            config,
            peers: BTreeSet::new(),
            pending: VecDeque::new(),
            ticker,
            selector: PeerSelector::default(),
        })
    }

    fn send_to_fanout(&mut self, message: &SyncMessage) -> Result<(), GossipError> {
        let data = codec::encode(message)?;
        let peers = self.selector.select(&self.peers, self.config.fanout);

        for peer in peers {
            if let Err(error) = self.network.request(&peer, data.clone()) {
                warn!(%peer, %error, "failed to send sync message");
            }
        }

        Ok(())
    }

    fn pull_from_peers(&mut self) -> Result<(), GossipError> {
        let message = SyncMessage::Pull {
            have: self.store.summary(),
        };

        self.send_to_fanout(&message)
    }

    fn push_to_peers(&mut self, events: Vec<Event>) -> Result<(), GossipError> {
        self.send_to_fanout(&SyncMessage::Push { events })
    }

    fn handle_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::PeerDiscovered(peer) => {
                if self.peers.insert(peer.clone()) {
                    self.pending.push_back(GossipEvent::PeerJoined(peer));
                }
            }
            NetworkEvent::PeerExpired(peer) => {
                if self.peers.remove(&peer) {
                    self.pending.push_back(GossipEvent::PeerLeft(peer));
                }
            }
            NetworkEvent::Request {
                peer,
                request_id,
                data,
            } => self.handle_request(peer, request_id, &data),
            NetworkEvent::Response { peer, data } => self.handle_response(&peer, &data),
            NetworkEvent::RequestFailed { peer, error } => {
                debug!(%peer, %error, "sync request failed");
            }
        }
    }

    fn handle_request(&mut self, peer: NodeId, request_id: RequestId, data: &[u8]) {
        let response = match codec::decode(data) {
            Ok(SyncMessage::Pull { have }) => self.answer_pull(have),
            Ok(SyncMessage::Push { events }) => self.accept_push(events),
            Ok(SyncMessage::PullResponse { .. } | SyncMessage::PushAck) => {
                warn!(%peer, "ignoring unexpected sync request");
                return;
            }
            Err(error) => {
                warn!(%peer, %error, "discarding invalid sync request");
                return;
            }
        };

        if let Err(error) = self.send_response(request_id, &response) {
            warn!(%peer, %error, "failed to send sync response");
        }
    }

    fn answer_pull(&self, have: Vec<EventId>) -> SyncMessage {
        let have: HashSet<EventId> = have.into_iter().collect();
        let events = self
            .store
            .missing_from(&have, self.config.max_events_per_response);

        SyncMessage::PullResponse { events }
    }

    fn accept_push(&mut self, events: Vec<Event>) -> SyncMessage {
        self.merge_events(events);
        SyncMessage::PushAck
    }

    fn send_response(
        &mut self,
        request_id: RequestId,
        response: &SyncMessage,
    ) -> Result<(), GossipError> {
        let data = codec::encode(response)?;
        self.network.respond(request_id, data)?;
        Ok(())
    }

    fn handle_response(&mut self, peer: &NodeId, data: &[u8]) {
        match codec::decode(data) {
            Ok(SyncMessage::PullResponse { events }) => {
                self.apply_pull_response(peer, events);
            }
            Ok(SyncMessage::PushAck) => self.handle_push_ack(peer),
            Ok(SyncMessage::Pull { .. } | SyncMessage::Push { .. }) => {
                warn!(%peer, "ignoring unexpected sync response");
            }
            Err(error) => warn!(%peer, %error, "discarding invalid sync response"),
        }
    }

    fn apply_pull_response(&mut self, peer: &NodeId, events: Vec<Event>) {
        let inserted = self.merge_events(events);
        debug!(%peer, inserted, "applied pull response");
    }

    fn handle_push_ack(&self, peer: &NodeId) {
        debug!(%peer, "push acknowledged");
    }

    fn integrate(&mut self, event: Event) -> bool {
        if self.store.insert(event.clone()) {
            self.pending.push_back(GossipEvent::Event(event));
            true
        } else {
            false
        }
    }

    fn merge_events(&mut self, events: Vec<Event>) -> usize {
        let mut inserted = 0;

        for event in events {
            if self.integrate(event) {
                inserted += 1;
            }
        }

        inserted
    }
}

impl Gossip for AntiEntropyGossip {
    async fn publish(&mut self, event: Event) -> Result<(), GossipError> {
        if self.integrate(event.clone()) {
            self.push_to_peers(vec![event])?;
        }

        Ok(())
    }

    async fn next_event(&mut self) -> Option<GossipEvent> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some(event);
            }

            tokio::select! {
                _ = self.ticker.tick() => {
                    if let Err(error) = self.pull_from_peers() {
                        warn!(%error, "failed to start pull round");
                    }
                }
                event = self.network.next_event() => self.handle_network_event(event?),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use locap2p_network::NetworkConfig;
    use serde_json::json;

    use super::*;

    fn gossip() -> AntiEntropyGossip {
        let network = Network::start(NetworkConfig {
            listen_addrs: Vec::new(),
        })
        .unwrap();
        AntiEntropyGossip::start(network, AntiEntropyConfig::default()).unwrap()
    }

    fn event(seq: u64) -> Event {
        Event::new(NodeId::from("node-a"), seq, json!({ "seq": seq }), seq)
    }

    #[tokio::test]
    async fn publish_integrates_and_emits_event_once() {
        let mut gossip = gossip();
        let event = event(1);

        gossip.publish(event.clone()).await.unwrap();

        assert!(gossip.store.contains(event.id()));
        assert_eq!(gossip.pending.len(), 1);

        match gossip.next_event().await {
            Some(GossipEvent::Event(emitted)) => assert_eq!(emitted, event),
            other => panic!("expected event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn duplicate_publish_is_ignored() {
        let mut gossip = gossip();
        let event = event(1);

        gossip.publish(event.clone()).await.unwrap();
        gossip.publish(event).await.unwrap();

        assert_eq!(gossip.pending.len(), 1);
    }

    #[test]
    fn peer_selection_rotates_round_robin() {
        let peers = BTreeSet::from([
            NodeId::from("node-a"),
            NodeId::from("node-b"),
            NodeId::from("node-c"),
        ]);
        let mut selector = PeerSelector::default();

        assert_eq!(
            selector.select(&peers, 2),
            vec![NodeId::from("node-a"), NodeId::from("node-b")]
        );
        assert_eq!(
            selector.select(&peers, 2),
            vec![NodeId::from("node-c"), NodeId::from("node-a")]
        );
        assert_eq!(
            selector.select(&peers, 2),
            vec![NodeId::from("node-b"), NodeId::from("node-c")]
        );
    }

    #[test]
    fn peer_selection_handles_empty_peers_and_zero_fanout() {
        let mut selector = PeerSelector::default();

        assert!(selector.select(&BTreeSet::new(), 2).is_empty());

        let peers = BTreeSet::from([NodeId::from("node-a")]);
        assert!(selector.select(&peers, 0).is_empty());
    }
}
