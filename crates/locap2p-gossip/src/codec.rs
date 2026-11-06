use locap2p_core::{Event, NodeId};

use crate::CodecError;

pub(crate) fn encode(event: &Event) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(event)
}

pub(crate) fn decode(source: &NodeId, data: &[u8]) -> Result<Event, CodecError> {
    let event: Event = serde_json::from_slice(data)?;
    event.verify_id().map_err(|_| CodecError::InvalidEventId)?;

    if event.author() != source {
        return Err(CodecError::AuthorMismatch);
    }

    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(author: &str, payload: &str, timestamp_nanos: u64) -> Event {
        Event::new(NodeId::from(author), payload, timestamp_nanos)
    }

    #[test]
    fn event_round_trips() {
        let original = event("node-a", "hello", 10);
        let encoded = encode(&original).unwrap();
        let decoded = decode(original.author(), &encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn author_mismatch_is_rejected() {
        let original = event("node-a", "hello", 10);
        let encoded = encode(&original).unwrap();

        assert!(matches!(
            decode(&NodeId::from("node-b"), &encoded),
            Err(CodecError::AuthorMismatch)
        ));
    }

    #[test]
    fn tampered_event_is_rejected() {
        let original = event("node-a", "hello", 10);
        let encoded = encode(&original).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        value["payload"] = serde_json::Value::String("tampered".to_owned());
        let tampered = serde_json::to_vec(&value).unwrap();

        assert!(matches!(
            decode(original.author(), &tampered),
            Err(CodecError::InvalidEventId)
        ));
    }
}
