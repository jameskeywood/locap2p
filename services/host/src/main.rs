//! Framework host that bridges gossip to an application over stdio.

use anyhow::Result;
use locap2p_gossip::{GossipEvent, GossipsubGossip, Topic};
use locap2p_ipc::{ClientFrame, ServerFrame, read_frame, write_frame};
use locap2p_network::{Network, NetworkConfig};
use tokio::io::{AsyncBufReadExt, BufReader, stdout};
use tokio::sync::mpsc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod host;

use host::Host;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let network = Network::start(NetworkConfig::default())?;
    let node_id = network.local_node_id();
    let gossip = GossipsubGossip::start(network, Topic::default())?;
    let mut host = Host::new(node_id.clone(), gossip);

    let (frames_tx, frames_rx) = mpsc::channel(64);
    let writer = tokio::spawn(write_frames(frames_rx));

    frames_tx
        .send(ServerFrame::Ready {
            node_id: node_id.clone(),
        })
        .await?;
    info!(node_id = %node_id, "host ready");

    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    loop {
        tokio::select! {
            frame = read_frame::<ClientFrame, _>(&mut lines) => {
                match frame {
                    Ok(Some(ClientFrame::Publish { payload })) => {
                        match host.publish(payload).await {
                            Ok(event) => {
                                info!(
                                    event_id = %event.id(),
                                    author = %event.author(),
                                    "published event"
                                );
                            }
                            Err(error) => {
                                warn!(%error, "failed to publish event");
                                if frames_tx
                                    .send(ServerFrame::Error {
                                        message: error.to_string(),
                                    })
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        warn!(%error, "invalid ipc frame");
                        if frames_tx
                            .send(ServerFrame::Error {
                                message: error.to_string(),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            event = host.next_event() => {
                match event {
                    Some(GossipEvent::Event(event)) => {
                        if host.observe(event.clone())
                            && frames_tx.send(ServerFrame::Event { event }).await.is_err()
                        {
                            break;
                        }
                    }
                    Some(GossipEvent::PeerJoined(node_id)) => {
                        if frames_tx
                            .send(ServerFrame::PeerJoined { node_id })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Some(GossipEvent::PeerLeft(node_id)) => {
                        if frames_tx
                            .send(ServerFrame::PeerLeft { node_id })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    drop(frames_tx);
    writer.await??;

    Ok(())
}

async fn write_frames(mut frames: mpsc::Receiver<ServerFrame>) -> Result<()> {
    let mut stdout = stdout();

    while let Some(frame) = frames.recv().await {
        write_frame(&mut stdout, &frame).await?;
    }

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
}
