use crate::{CodecError, message::SyncMessage};

pub(crate) fn encode(message: &SyncMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(message)
}

pub(crate) fn decode(data: &[u8]) -> Result<SyncMessage, CodecError> {
    let message: SyncMessage = serde_json::from_slice(data)?;
    validate(&message)?;
    Ok(message)
}

fn validate(message: &SyncMessage) -> Result<(), CodecError> {
    let events = match message {
        SyncMessage::PullResponse { events } | SyncMessage::Push { events } => events,
        SyncMessage::Pull { .. } | SyncMessage::PushAck => return Ok(()),
    };

    for event in events {
        event.verify_id().map_err(|_| CodecError::InvalidEventId)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use locap2p_core::{Event, NodeId};
    use serde_json::{Value, json};

    use super::*;

    fn event(author: &str, payload: Value, timestamp_nanos: u64) -> Event {
        Event::new(NodeId::from(author), 1, payload, timestamp_nanos)
    }

    #[test]
    fn pull_round_trips() {
        let message = SyncMessage::Pull {
            have: vec![event("node-a", json!("hello"), 10).id().clone()],
        };

        assert_eq!(decode(&encode(&message).unwrap()).unwrap(), message);
    }

    #[test]
    fn push_round_trips() {
        let message = SyncMessage::Push {
            events: vec![event("node-a", json!("hello"), 10)],
        };

        assert_eq!(decode(&encode(&message).unwrap()).unwrap(), message);
    }

    #[test]
    fn pull_response_round_trips() {
        let message = SyncMessage::PullResponse {
            events: vec![event("node-a", json!("hello"), 10)],
        };

        assert_eq!(decode(&encode(&message).unwrap()).unwrap(), message);
    }

    #[test]
    fn push_ack_round_trips() {
        let message = SyncMessage::PushAck;

        assert_eq!(decode(&encode(&message).unwrap()).unwrap(), message);
    }

    #[test]
    fn tampered_event_is_rejected() {
        let message = SyncMessage::Push {
            events: vec![event("node-a", json!("hello"), 10)],
        };
        let encoded = encode(&message).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        value["events"][0]["payload"] = json!("tampered");
        let tampered = serde_json::to_vec(&value).unwrap();

        assert!(matches!(decode(&tampered), Err(CodecError::InvalidEventId)));
    }

    #[test]
    fn relayed_event_is_accepted() {
        let message = SyncMessage::PullResponse {
            events: vec![event("node-a", json!("hello"), 10)],
        };

        let decoded = decode(&encode(&message).unwrap()).unwrap();

        assert_eq!(decoded, message);
    }
}
