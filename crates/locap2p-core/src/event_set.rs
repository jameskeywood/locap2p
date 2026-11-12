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

    /// Merges another set into this one and returns the number of new events.
    pub fn merge(&mut self, other: &Self) -> usize {
        let mut inserted = 0;

        for event in other.events.values() {
            if self.insert(event.clone()) {
                inserted += 1;
            }
        }

        inserted
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
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

    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.events.values()
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
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn merge_is_idempotent() {
        let mut left = EventSet::default();
        let mut right = EventSet::default();
        left.insert(Event::new(NodeId::from("node-a"), 1, json!("one"), 1));
        right.insert(Event::new(NodeId::from("node-b"), 1, json!("two"), 2));

        assert_eq!(left.merge(&right), 1);
        assert_eq!(left.merge(&right), 0);
        assert_eq!(left.len(), 2);
    }

    #[test]
    fn merge_is_commutative() {
        let mut left = EventSet::default();
        let mut right = EventSet::default();
        left.insert(Event::new(NodeId::from("node-a"), 1, json!("left"), 1));
        right.insert(Event::new(NodeId::from("node-b"), 1, json!("right"), 2));

        let mut left_then_right = left.clone();
        left_then_right.merge(&right);

        let mut right_then_left = right.clone();
        right_then_left.merge(&left);

        assert_eq!(sorted_ids(&left_then_right), sorted_ids(&right_then_left));
    }

    #[test]
    fn merge_is_associative() {
        let mut first = EventSet::default();
        let mut second = EventSet::default();
        let mut third = EventSet::default();
        first.insert(Event::new(NodeId::from("node-a"), 1, json!("one"), 1));
        second.insert(Event::new(NodeId::from("node-b"), 1, json!("two"), 2));
        third.insert(Event::new(NodeId::from("node-c"), 1, json!("three"), 3));

        let mut left = first.clone();
        let mut second_then_third = second.clone();
        second_then_third.merge(&third);
        left.merge(&second_then_third);

        let mut right = first.clone();
        right.merge(&second);
        right.merge(&third);

        assert_eq!(sorted_ids(&left), sorted_ids(&right));
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

    fn sorted_ids(set: &EventSet) -> Vec<String> {
        let mut ids: Vec<String> = set.iter().map(|event| event.id().to_string()).collect();
        ids.sort();
        ids
    }
}
