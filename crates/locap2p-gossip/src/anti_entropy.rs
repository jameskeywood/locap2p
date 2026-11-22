use std::collections::{BTreeSet, HashSet, VecDeque};

use locap2p_config::GossipConfig;
use locap2p_core::{Event, EventId, EventSet, NodeId};
use locap2p_network::{Network, NetworkEvent, RequestId};
use tokio::time::{Interval, MissedTickBehavior, interval};
use tracing::{debug, warn};

use crate::{
    Gossip, GossipError, GossipEvent, codec,
    membership::{MembershipChange, MembershipView, PeerRecord, now_ms},
    message::SyncMessage,
};

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
    config: GossipConfig,
    membership: MembershipView,
    candidates: BTreeSet<NodeId>,
    pending: VecDeque<GossipEvent>,
    ticker: Interval,
    selector: PeerSelector,
}

impl AntiEntropyGossip {
    /// Creates an anti-entropy gossip instance.
    pub fn start(network: Network, config: GossipConfig) -> Result<Self, GossipError> {
        if config.sync_interval_ms == 0 {
            return Err(GossipError::InvalidConfig("sync interval must be non-zero"));
        }

        let membership = MembershipView::new(
            network.local_node_id(),
            network.local_addresses(),
            now_ms(),
            config.membership_ttl_ms,
            config.expiry_grace_ms,
        );
        let mut ticker = interval(config.sync_interval());
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Ok(Self {
            network,
            store: EventSet::default(),
            config,
            membership,
            candidates: BTreeSet::new(),
            pending: VecDeque::new(),
            ticker,
            selector: PeerSelector::default(),
        })
    }

    fn send_to_fanout(
        &mut self,
        message: &SyncMessage,
        include_candidates: bool,
    ) -> Result<(), GossipError> {
        let data = codec::encode(message)?;
        let mut targets = self.membership.active_peers();
        if include_candidates {
            targets.extend(self.candidates.iter().cloned());
        }
        let peers = self.selector.select(&targets, self.config.fanout);

        for peer in peers {
            if let Err(error) = self.network.request(&peer, data.clone()) {
                warn!(%peer, %error, "failed to send sync message");
            }
        }

        Ok(())
    }

    fn pull_from_peers(&mut self) -> Result<(), GossipError> {
        self.refresh_local_record();
        let message = SyncMessage::Pull {
            have: self.store.summary(),
            members: self.membership.records(),
        };

        self.send_to_fanout(&message, true)
    }

    fn push_to_peers(&mut self, events: Vec<Event>) -> Result<(), GossipError> {
        self.refresh_local_record();
        let message = SyncMessage::Push {
            events,
            members: self.membership.records(),
        };

        self.send_to_fanout(&message, false)
    }

    fn refresh_local_record(&mut self) {
        self.membership
            .refresh_local(self.network.local_addresses(), now_ms());
    }

    fn expire_members(&mut self) {
        let changes = self.membership.expire(now_ms());
        self.apply_membership_changes(changes);
    }

    fn handle_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::SeedPeers(peers) => {
                self.candidates.extend(peers);
            }
            NetworkEvent::Request {
                peer,
                request_id,
                data,
            } => self.handle_request(peer, request_id, &data),
            NetworkEvent::Response { peer, data } => self.handle_response(&peer, &data),
            NetworkEvent::RequestFailed { peer, error } => {
                self.candidates.remove(&peer);
                debug!(%peer, %error, "sync request failed");
            }
        }
    }

    fn handle_request(&mut self, peer: NodeId, request_id: RequestId, data: &[u8]) {
        let message = match codec::decode(data) {
            Ok(message) => message,
            Err(error) => {
                warn!(%peer, %error, "discarding invalid sync request");
                return;
            }
        };

        let response = match message {
            SyncMessage::Pull { have, members } => {
                self.merge_members(members);
                let include_events = self.membership.is_active(&peer);
                self.answer_pull(have, include_events)
            }
            SyncMessage::Push { events, members } => {
                self.merge_members(members);
                if self.membership.is_active(&peer) {
                    self.merge_events(events);
                } else {
                    debug!(%peer, "discarding events from non-member");
                }
                SyncMessage::PushAck
            }
            SyncMessage::PullResponse { .. } | SyncMessage::PushAck => {
                warn!(%peer, "ignoring unexpected sync request");
                return;
            }
        };

        if let Err(error) = self.send_response(request_id, &response) {
            warn!(%peer, %error, "failed to send sync response");
        }
    }

    fn answer_pull(&self, have: Vec<EventId>, include_events: bool) -> SyncMessage {
        let events = if include_events {
            let have = have.into_iter().collect::<HashSet<_>>();
            self.store
                .missing_from(&have, self.config.max_events_per_response)
        } else {
            Vec::new()
        };

        SyncMessage::PullResponse {
            events,
            members: self.membership.records(),
        }
    }

    fn merge_members(&mut self, members: Vec<PeerRecord>) {
        let changes = self.membership.merge(members, now_ms());
        self.apply_membership_changes(changes);
    }

    fn apply_membership_changes(&mut self, changes: Vec<MembershipChange>) {
        for change in changes {
            match change {
                MembershipChange::Joined { peer_id, addresses } => {
                    self.candidates.remove(&peer_id);
                    self.register_peer_addresses(&peer_id, &addresses);
                    self.pending.push_back(GossipEvent::PeerJoined(peer_id));
                }
                MembershipChange::Left { peer_id } => {
                    self.candidates.remove(&peer_id);
                    self.pending.push_back(GossipEvent::PeerLeft(peer_id));
                }
                MembershipChange::AddressesUpdated { peer_id, addresses } => {
                    self.candidates.remove(&peer_id);
                    self.register_peer_addresses(&peer_id, &addresses);
                }
            }
        }
    }

    fn register_peer_addresses(&mut self, peer_id: &NodeId, addresses: &[String]) {
        if let Err(error) = self.network.register_peer_addresses(peer_id, addresses) {
            warn!(%peer_id, %error, "failed to register member addresses");
        }
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
            Ok(SyncMessage::PullResponse { events, members }) => {
                self.merge_members(members);
                if self.membership.is_active(peer) {
                    self.apply_pull_response(peer, events);
                } else {
                    debug!(%peer, "discarding events from non-member");
                }
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
                    self.expire_members();
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
    use locap2p_config::{
        ApplicationConfig, ApplicationId, CONFIG_SCHEMA_VERSION, ConnectivityConfig,
        DiscoveryConfig, FrameworkConfig, GossipConfig, IdentityConfig, NetworkConfig, SessionId,
    };
    use serde_json::json;

    use super::*;

    fn gossip() -> AntiEntropyGossip {
        let directory = tempfile::tempdir().unwrap();
        let config = FrameworkConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            application: ApplicationConfig {
                id: ApplicationId::new("locap2p-test/1").unwrap(),
                session: SessionId::new("test").unwrap(),
            },
            identity: IdentityConfig {
                path: directory.path().join("node.key"),
            },
            network: NetworkConfig {
                listen_addrs: Vec::new(),
                request_timeout_ms: 1_000,
            },
            discovery: DiscoveryConfig {
                bootstrap: Vec::new(),
            },
            connectivity: ConnectivityConfig::default(),
            gossip: GossipConfig::default(),
        };
        let network = Network::start(&config).unwrap();
        AntiEntropyGossip::start(network, GossipConfig::default()).unwrap()
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
