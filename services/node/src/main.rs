use anyhow::Result;
use locap2p_core::Event;
use locap2p_gossip::{GossipEvent, GossipsubGossip, Topic};
use locap2p_network::{Network, NetworkConfig};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing_subscriber::EnvFilter;

mod node;

use node::Node;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let (handle, events) = Network::start(NetworkConfig::default()).await?;
    let node_id = handle.local_node_id();
    let gossip = GossipsubGossip::start(handle, events, Topic::default()).await?;
    let mut node = Node::new(node_id.clone(), gossip);

    println!("Node: {node_id}");
    println!("Type a message and press Enter. Press Ctrl-C to quit.");

    let mut stdin = BufReader::new(tokio::io::stdin()).lines();

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                match line? {
                    Some(line) if !line.trim().is_empty() => {
                        let event = node.create_event(line);
                        node.observe(event.clone());
                        print_event(&event);

                        if let Err(error) = node.publish(&event).await {
                            eprintln!("warning: failed to publish event: {error}");
                        }
                    }
                    Some(_) => {}
                    None => break,
                }
            }
            event = node.next_event() => {
                match event {
                    Some(GossipEvent::Event(event)) => {
                        if node.observe(event.clone()) {
                            print_event(&event);
                        }
                    }
                    Some(GossipEvent::PeerJoined(peer)) => println!("peer joined: {peer}"),
                    Some(GossipEvent::PeerLeft(peer)) => println!("peer left: {peer}"),
                    None => break,
                }
            }
            result = tokio::signal::ctrl_c() => {
                result?;
                break;
            }
        }
    }

    Ok(())
}

fn print_event(event: &Event) {
    println!("{}: {}", event.author(), event.payload());
}
