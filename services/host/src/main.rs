//! Framework host that bridges gossip to an application over a Unix domain socket.
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use locap2p_gossip::{GossipEvent, start as start_gossip};
use locap2p_ipc::{ApplicationFrame, FrameworkFrame, read_frame, write_frame};
use locap2p_network::Network;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::OwnedWriteHalf;
use tokio::sync::mpsc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod host;

use host::Host;

#[derive(Debug, Parser)]
#[command(name = "locap2p-host")]
struct Cli {
    /// Path to the Unix socket created by the application
    #[arg(long)]
    ipc_socket: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();

    // Connect to the application's Unix socket path
    let stream = UnixStream::connect(&cli.ipc_socket)
        .await
        .with_context(|| format!("failed to connect to {}", cli.ipc_socket.display()))?;

    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    let config = match read_frame::<ApplicationFrame, _>(&mut lines).await {
        Ok(Some(ApplicationFrame::Configure { config })) => config,
        Ok(Some(frame)) => {
            send_error(
                format!("expected a configure frame before any other frame, received {frame:?}"),
                &mut write_half,
            )
            .await?;
            return Ok(());
        }
        Ok(None) => return Ok(()),
        Err(error) => {
            send_error(format!("invalid IPC frame: {error}"), &mut write_half).await?;
            return Ok(());
        }
    };

    let network = match Network::start(&config) {
        Ok(network) => network,
        Err(error) => {
            send_error(format!("failed to start network: {error}"), &mut write_half).await?;
            return Ok(());
        }
    };
    let node_id = network.local_node_id();
    let gossip = match start_gossip(network, &config.gossip) {
        Ok(gossip) => gossip,
        Err(error) => {
            send_error(format!("failed to start gossip: {error}"), &mut write_half).await?;
            return Ok(());
        }
    };
    let mut host = Host::new(node_id.clone(), gossip);

    let (frames_tx, frames_rx) = mpsc::channel(64);
    let writer = tokio::spawn(write_frames(frames_rx, write_half));

    frames_tx
        .send(FrameworkFrame::Ready {
            node_id: node_id.clone(),
        })
        .await?;
    info!(
        node_id = %node_id,
        application = %config.application.id,
        session = %config.application.session,
        "host ready"
    );

    loop {
        tokio::select! {
            frame = read_frame::<ApplicationFrame, _>(&mut lines) => {
                match frame {
                    Ok(Some(ApplicationFrame::Publish { payload })) => {
                        let event = host.create_event(payload);
                        let event_id = event.id().clone();
                        let author = event.author().clone();

                        match host.publish(event).await {
                            Ok(()) => {
                                info!(
                                    event_id = %event_id,
                                    author = %author,
                                    "published event"
                                );
                            }
                            Err(error) => {
                                warn!(%error, "failed to publish event");
                                if frames_tx
                                    .send(FrameworkFrame::Error {
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
                    Ok(Some(ApplicationFrame::Configure { .. })) => {
                        if frames_tx
                            .send(FrameworkFrame::Error {
                                message: "host is already configured".to_owned(),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        warn!(%error, "invalid ipc frame");
                        if frames_tx
                            .send(FrameworkFrame::Error {
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
                        if frames_tx.send(FrameworkFrame::Event { event }).await.is_err() {
                            break;
                        }
                    }
                    Some(GossipEvent::PeerJoined(node_id)) => {
                        if frames_tx
                            .send(FrameworkFrame::PeerJoined { node_id })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Some(GossipEvent::PeerLeft(node_id)) => {
                        if frames_tx
                            .send(FrameworkFrame::PeerLeft { node_id })
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

async fn send_error(message: impl Into<String>, write_half: &mut OwnedWriteHalf) -> Result<()> {
    write_frame(
        write_half,
        &FrameworkFrame::Error {
            message: message.into(),
        },
    )
    .await?;
    Ok(())
}

async fn write_frames(
    mut frames: mpsc::Receiver<FrameworkFrame>,
    mut write_half: OwnedWriteHalf,
) -> Result<()> {
    while let Some(frame) = frames.recv().await {
        write_frame(&mut write_half, &frame).await?;
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
