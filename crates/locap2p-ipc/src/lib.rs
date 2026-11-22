//! Newline-delimited JSON protocol between applications and the framework host.

use locap2p_config::FrameworkConfig;
use locap2p_core::{Event, NodeId};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt, Lines};

/// Maximum size of a single IPC frame in bytes.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// Frames sent from an application to the framework host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientFrame {
    Configure { config: FrameworkConfig },
    Publish { payload: Value },
}

/// Frames sent from the framework host to an application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    Ready { node_id: NodeId },
    Event { event: Event },
    PeerJoined { node_id: NodeId },
    PeerLeft { node_id: NodeId },
    Error { message: String },
}

/// Errors returned while reading or writing IPC frames.
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("ipc io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to encode ipc frame: {0}")]
    Encode(serde_json::Error),
    #[error("failed to decode ipc frame: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("ipc frame exceeds the maximum size of {MAX_FRAME_BYTES} bytes")]
    FrameTooLarge,
}

/// Reads one newline-delimited JSON frame, returning `None` on EOF.
pub async fn read_frame<T, R>(lines: &mut Lines<R>) -> Result<Option<T>, IpcError>
where
    T: DeserializeOwned,
    R: AsyncBufRead + Unpin,
{
    loop {
        let Some(line) = lines.next_line().await? else {
            return Ok(None);
        };

        if line.len() > MAX_FRAME_BYTES {
            return Err(IpcError::FrameTooLarge);
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        return Ok(Some(serde_json::from_str(line)?));
    }
}

/// Writes one newline-delimited JSON frame and flushes it.
pub async fn write_frame<T, W>(writer: &mut W, frame: &T) -> Result<(), IpcError>
where
    T: Serialize,
    W: AsyncWrite + Unpin,
{
    let mut frame = serde_json::to_vec(frame).map_err(IpcError::Encode)?;

    if frame.len() > MAX_FRAME_BYTES {
        return Err(IpcError::FrameTooLarge);
    }

    frame.push(b'\n');
    writer.write_all(&frame).await?;
    writer.flush().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use locap2p_config::FrameworkConfig;
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex};

    use super::*;

    #[tokio::test]
    async fn client_frame_round_trips() {
        let frame = ClientFrame::Publish {
            payload: json!({ "x": 1, "y": 2 }),
        };
        let (mut writer, reader) = duplex(1024);

        write_frame(&mut writer, &frame).await.unwrap();
        let mut lines = BufReader::new(reader).lines();
        let decoded: Option<ClientFrame> = read_frame(&mut lines).await.unwrap();

        assert_eq!(decoded, Some(frame));
    }

    #[tokio::test]
    async fn configure_frame_round_trips() {
        let config: FrameworkConfig = serde_json::from_value(json!({
            "schema_version": 1,
            "application": { "id": "locap2p-test/1", "session": "demo" },
            "identity": { "path": "state/demo/node.key" },
            "network": {
                "listen_addrs": ["/ip4/127.0.0.1/tcp/0"],
                "request_timeout_ms": 1000
            },
            "discovery": {
                "bootstrap": ["/ip4/127.0.0.1/tcp/4002/p2p/12D3KooWTest"]
            },
            "connectivity": {
                "relay": [],
                "reserve_relay": false,
                "prefer_direct": true,
                "enable_dcutr": false
            },
            "gossip": {
                "sync_interval_ms": 1000,
                "fanout": 3,
                "max_events_per_response": 64,
                "membership_ttl_ms": 10000,
                "expiry_grace_ms": 1000
            }
        }))
        .unwrap();
        let frame = ClientFrame::Configure { config };
        let (mut writer, reader) = duplex(4096);

        write_frame(&mut writer, &frame).await.unwrap();
        let mut lines = BufReader::new(reader).lines();
        let decoded: Option<ClientFrame> = read_frame(&mut lines).await.unwrap();

        assert_eq!(decoded, Some(frame));
    }

    #[tokio::test]
    async fn malformed_frame_is_rejected() {
        let (mut writer, reader) = duplex(1024);
        writer.write_all(b"not json\n").await.unwrap();
        let mut lines = BufReader::new(reader).lines();

        let result: Result<Option<ClientFrame>, _> = read_frame(&mut lines).await;

        assert!(matches!(result, Err(IpcError::Decode(_))));
    }
}
