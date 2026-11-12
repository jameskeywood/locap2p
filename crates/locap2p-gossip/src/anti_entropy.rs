use std::{
    collections::{BTreeSet, HashSet, VecDeque},
    time::Duration,
};

use locap2p_core::{Event, EventId, EventStore, NodeId};
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

/// Gossip strategy based on periodic pull anti-entropy and eager push.
pub struct AntiEntropyGossip {
    network: Network,
    store: EventStore,
    config: AntiEntropyConfig,
    peers: BTreeSet<NodeId>,
    pending: VecDeque<GossipEvent>,
    ticker: Interval,
    cursor: usize,
}

impl AntiEntropyGossip {
    /// Creates an anti-entropy gossip instance.
    pub fn start(
        network: Network,
        store: EventStore,
        config: AntiEntropyConfig,
    ) -> Result<Self, GossipError> {
        if config.sync_interval.is_zero() {
            return Err(GossipError::InvalidConfig("sync interval must be non-zero"));
        }

        let mut ticker = interval(config.sync_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Ok(Self {
            network,
            store,
            config,
            peers: BTreeSet::new(),
            pending: VecDeque::new(),
            ticker,
            cursor: 0,
        })
    }

    fn pull_from_peers(&mut self) {
        let message = SyncMessage::Pull {
            have: self.store.summary(),
        };
        let data = match codec::encode(&message) {
            Ok(data) => data,
            Err(error) => {
                warn!(%error, "failed to encode pull request");
                return;
            }
        };

        for peer in self.select_peers() {
            if let Err(error) = self.network.request(&peer, data.clone()) {
                warn!(%peer, %error, "failed to send pull request");
            }
        }
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
            Ok(SyncMessage::Pull { have }) => {
                let have: HashSet<EventId> = have.into_iter().collect();
                let events = self
                    .store
                    .missing_from(&have, self.config.max_events_per_response);
                SyncMessage::PullResponse { events }
            }
            Ok(SyncMessage::Push { events }) => {
                self.merge_events(events);
                SyncMessage::PushAck
            }
            Ok(SyncMessage::PullResponse { .. } | SyncMessage::PushAck) => {
                warn!(%peer, "ignoring unexpected sync request");
                SyncMessage::PushAck
            }
            Err(error) => {
                warn!(%peer, %error, "discarding invalid sync request");
                SyncMessage::PushAck
            }
        };

        let result = codec::encode(&response)
            .map_err(GossipError::from)
            .and_then(|data| {
                self.network
                    .respond(request_id, data)
                    .map_err(GossipError::from)
            });

        if let Err(error) = result {
            warn!(%peer, %error, "failed to send sync response");
        }
    }

    fn handle_response(&mut self, peer: &NodeId, data: &[u8]) {
        match codec::decode(data) {
            Ok(SyncMessage::PullResponse { events }) => {
                let inserted = self.merge_events(events);
                debug!(%peer, inserted, "applied pull response");
            }
            Ok(SyncMessage::PushAck) => debug!(%peer, "push acknowledged"),
            Ok(SyncMessage::Pull { .. } | SyncMessage::Push { .. }) => {
                warn!(%peer, "ignoring unexpected sync response");
            }
            Err(error) => warn!(%peer, %error, "discarding invalid sync response"),
        }
    }

    fn merge_events(&mut self, events: Vec<Event>) -> usize {
        let mut inserted = 0;

        for event in events {
            if self.store.insert(event.clone()) {
                inserted += 1;
                self.pending.push_back(GossipEvent::Event(event));
            }
        }

        inserted
    }

    fn select_peers(&mut self) -> Vec<NodeId> {
        if self.peers.is_empty() || self.config.fanout == 0 {
            return Vec::new();
        }

        let peers: Vec<NodeId> = self.peers.iter().cloned().collect();
        let start = self.cursor % peers.len();
        let count = self.config.fanout.min(peers.len());
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

impl Gossip for AntiEntropyGossip {
    async fn publish(&mut self, event: Event) -> Result<(), GossipError> {
        self.store.insert(event.clone());

        let message = SyncMessage::Push {
            events: vec![event],
        };
        let data = codec::encode(&message)?;

        for peer in self.select_peers() {
            if let Err(error) = self.network.request(&peer, data.clone()) {
                warn!(%peer, %error, "failed to push event");
            }
        }

        Ok(())
    }

    async fn next_event(&mut self) -> Option<GossipEvent> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some(event);
            }

            tokio::select! {
                _ = self.ticker.tick() => self.pull_from_peers(),
                event = self.network.next_event() => self.handle_network_event(event?),
            }
        }
    }
}
