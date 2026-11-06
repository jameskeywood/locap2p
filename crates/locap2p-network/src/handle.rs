use locap2p_core::NodeId;
use tokio::sync::{mpsc, oneshot};

use crate::{NetworkError, NetworkEvent};

pub(crate) enum NetworkCommand {
    Subscribe {
        topic: String,
        reply: oneshot::Sender<Result<(), NetworkError>>,
    },
    Publish {
        topic: String,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<(), NetworkError>>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

/// Cloneable handle used to send commands to the network actor.
#[derive(Clone)]
pub struct NetworkHandle {
    node_id: NodeId,
    command_tx: mpsc::Sender<NetworkCommand>,
}

impl NetworkHandle {
    pub(crate) fn new(node_id: NodeId, command_tx: mpsc::Sender<NetworkCommand>) -> Self {
        Self {
            node_id,
            command_tx,
        }
    }

    pub fn local_node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    pub async fn subscribe(&self, topic: &str) -> Result<(), NetworkError> {
        let (reply, response) = oneshot::channel();
        self.command_tx
            .send(NetworkCommand::Subscribe {
                topic: topic.to_owned(),
                reply,
            })
            .await
            .map_err(|_| NetworkError::Closed)?;

        response.await.map_err(|_| NetworkError::Closed)?
    }

    pub async fn publish(&self, topic: &str, data: Vec<u8>) -> Result<(), NetworkError> {
        let (reply, response) = oneshot::channel();
        self.command_tx
            .send(NetworkCommand::Publish {
                topic: topic.to_owned(),
                data,
                reply,
            })
            .await
            .map_err(|_| NetworkError::Closed)?;

        response.await.map_err(|_| NetworkError::Closed)?
    }

    pub async fn shutdown(&self) {
        let (reply, response) = oneshot::channel();
        if self
            .command_tx
            .send(NetworkCommand::Shutdown { reply })
            .await
            .is_ok()
        {
            let _ = response.await;
        }
    }
}

/// Receives events emitted by the network actor.
pub struct NetworkEvents {
    event_rx: mpsc::Receiver<NetworkEvent>,
}

impl NetworkEvents {
    pub(crate) fn new(event_rx: mpsc::Receiver<NetworkEvent>) -> Self {
        Self { event_rx }
    }

    pub async fn next(&mut self) -> Option<NetworkEvent> {
        self.event_rx.recv().await
    }
}
