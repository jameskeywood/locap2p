use std::collections::{BTreeMap, HashSet};

use crate::{Event, EventId};

/// Grow-only set of observed events.
///
/// The merge operation is set union, which is commutative, associative and
/// idempotent. This makes the set a CRDT: nodes that observe the same events
/// converge to the same state regardless of delivery order.
#[derive(Debug, Clone, Default)]
pub struct EventSet {
    events: BTreeMap<EventId, Event>,
}

impl EventSet {
    /// Inserts an event and returns `true` if it was not already present.
    pub fn insert(&mut self, event: Event) -> bool {
        self.events.insert(event.id().clone(), event).is_none()
    }

    pub fn contains(&self, id: &EventId) -> bool {
        self.events.contains_key(id)
    }

    /// Inserts every event that is not already present and returns the newly
    /// inserted events.
    pub fn merge(&mut self, events: Vec<Event>) -> Vec<Event> {
        let mut inserted = Vec::new();

        for event in events {
            if self.insert(event.clone()) {
                inserted.push(event);
            }
        }

        inserted
    }

    /// Returns the IDs of every event in the set, in sorted order.
    pub fn summary(&self) -> Vec<EventId> {
        self.events.keys().cloned().collect()
    }

    /// Returns events not present in `have`, up to `limit` events.
    pub fn missing_from(&self, have: &HashSet<EventId>, limit: usize) -> Vec<Event> {
        self.events
            .values()
            .filter(|event| !have.contains(event.id()))
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::NodeId;

    #[test]
    fn duplicate_insert_is_ignored() {
        let mut set = EventSet::default();
        let event = Event::new(NodeId::from("node-a"), 1, json!("hello"), 1);

        assert!(set.insert(event.clone()));
        assert!(!set.insert(event));
        assert_eq!(set.summary().len(), 1);
    }

    #[test]
    fn merge_returns_only_new_events() {
        let mut set = EventSet::default();
        let known = Event::new(NodeId::from("node-a"), 1, json!("known"), 1);
        let new = Event::new(NodeId::from("node-b"), 1, json!("new"), 2);
        set.insert(known.clone());

        assert_eq!(set.merge(vec![known, new.clone()]), vec![new.clone()]);
        assert!(set.merge(vec![new]).is_empty());
        assert_eq!(set.summary().len(), 2);
    }

    #[test]
    fn summary_contains_every_event_id() {
        let mut set = EventSet::default();
        let first = Event::new(NodeId::from("node-a"), 1, json!("one"), 1);
        let second = Event::new(NodeId::from("node-b"), 1, json!("two"), 2);
        set.insert(first.clone());
        set.insert(second.clone());

        let mut summary = set.summary();
        summary.sort();

        let mut expected = vec![first.id().clone(), second.id().clone()];
        expected.sort();

        assert_eq!(summary, expected);
    }

    #[test]
    fn missing_from_returns_only_unknown_events() {
        let mut set = EventSet::default();
        let known = Event::new(NodeId::from("node-a"), 1, json!("known"), 1);
        let missing = Event::new(NodeId::from("node-b"), 1, json!("missing"), 2);
        set.insert(known.clone());
        set.insert(missing.clone());

        let have = HashSet::from([known.id().clone()]);
        let events = set.missing_from(&have, 10);

        assert_eq!(events, vec![missing]);
    }

    #[test]
    fn missing_from_respects_limit() {
        let mut set = EventSet::default();
        for seq in 1..=3 {
            set.insert(Event::new(NodeId::from("node-a"), seq, json!(seq), seq));
        }

        let events = set.missing_from(&HashSet::new(), 2);

        assert_eq!(events.len(), 2);
    }
}
