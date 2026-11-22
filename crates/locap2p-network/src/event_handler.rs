use std::collections::{HashMap, VecDeque};

use libp2p::{
    PeerId, identify,
    request_response::{self, Message, ResponseChannel},
    swarm::SwarmEvent,
};
use locap2p_core::NodeId;
use tracing::{debug, warn};

use crate::{
    behaviour::BehaviourEvent,
    event::{NetworkEvent, RequestId},
};

#[derive(Default)]
pub(crate) struct SwarmEventHandler {
    pending_requests: HashMap<RequestId, ResponseChannel<Vec<u8>>>,
    next_request_id: u64,
    pending_events: VecDeque<NetworkEvent>,
}

impl SwarmEventHandler {
    pub(crate) fn pop_event(&mut self) -> Option<NetworkEvent> {
        self.pending_events.pop_front()
    }

    pub(crate) fn push_event(&mut self, event: NetworkEvent) {
        self.pending_events.push_back(event);
    }

    pub(crate) fn take_request(
        &mut self,
        request_id: RequestId,
    ) -> Option<ResponseChannel<Vec<u8>>> {
        self.pending_requests.remove(&request_id)
    }

    pub(crate) fn handle(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Sync(request_response::Event::Message {
                peer,
                message,
                ..
            })) => {
                self.handle_sync_message(peer, message);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Sync(
                request_response::Event::OutboundFailure { peer, error, .. },
            )) => {
                warn!(%peer, %error, "sync request failed");
                self.pending_events.push_back(NetworkEvent::RequestFailed {
                    peer: node_id(peer),
                    error: error.to_string(),
                });
            }
            SwarmEvent::Behaviour(BehaviourEvent::Sync(
                request_response::Event::InboundFailure { peer, error, .. },
            )) => {
                warn!(%peer, %error, "sync response failed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Sync(
                request_response::Event::ResponseSent { peer, .. },
            )) => {
                debug!(%peer, "sync response sent");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Error {
                peer_id,
                error,
                ..
            })) => {
                debug!(%peer_id, %error, "identify failed");
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                debug!(%peer_id, ?endpoint, "connection established");
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!(%peer_id, "connection closed");
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, ?error, "outgoing connection failed");
            }
            SwarmEvent::IncomingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, ?error, "incoming connection failed");
            }
            _ => {}
        }
    }

    fn handle_sync_message(&mut self, peer: PeerId, message: Message<Vec<u8>, Vec<u8>>) {
        match message {
            Message::Request {
                request, channel, ..
            } => {
                let request_id = RequestId::new(self.next_request_id);
                self.next_request_id = self.next_request_id.wrapping_add(1);
                self.pending_requests.insert(request_id, channel);
                self.pending_events.push_back(NetworkEvent::Request {
                    peer: node_id(peer),
                    request_id,
                    data: request,
                });
            }
            Message::Response { response, .. } => {
                self.pending_events.push_back(NetworkEvent::Response {
                    peer: node_id(peer),
                    data: response,
                });
            }
        }
    }
}

fn node_id(peer_id: PeerId) -> NodeId {
    NodeId::from(peer_id.to_string())
}
