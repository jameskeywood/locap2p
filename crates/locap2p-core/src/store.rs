use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crate::{Event, EventId, EventSet};

/// Thread-safe handle to the node's shared event set.
///
/// Clones share the same underlying [`EventSet`], so the host and the gossip
/// layer always observe one logical set of events.
#[derive(Clone, Default)]
pub struct EventStore {
    inner: Arc<RwLock<EventSet>>,
}

impl EventStore {
    /// Inserts an event and returns `true` if it was not already present.
    pub fn insert(&self, event: Event) -> bool {
        self.inner
            .write()
            .expect("event store lock poisoned")
            .insert(event)
    }

    pub fn contains(&self, id: &EventId) -> bool {
        self.inner
            .read()
            .expect("event store lock poisoned")
            .contains(id)
    }

    /// Returns the IDs of every known event, in sorted order.
    pub fn summary(&self) -> Vec<EventId> {
        self.inner
            .read()
            .expect("event store lock poisoned")
            .summary()
    }

    /// Returns events not present in `have`, up to `limit` events.
    pub fn missing_from(&self, have: &HashSet<EventId>, limit: usize) -> Vec<Event> {
        self.inner
            .read()
            .expect("event store lock poisoned")
            .missing_from(have, limit)
    }

    pub fn len(&self) -> usize {
        self.inner.read().expect("event store lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .read()
            .expect("event store lock poisoned")
            .is_empty()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::NodeId;

    fn event(author: &str, seq: u64) -> Event {
        Event::new(NodeId::from(author), seq, json!(seq), seq)
    }

    #[test]
    fn clones_share_the_same_set() {
        let store = EventStore::default();
        let clone = store.clone();

        assert!(clone.insert(event("node-a", 1)));
        assert!(store.contains(event("node-a", 1).id()));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn duplicate_insertion_is_rejected() {
        let store = EventStore::default();
        let event = event("node-a", 1);

        assert!(store.insert(event.clone()));
        assert!(!store.insert(event));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn summary_and_missing_events_are_consistent() {
        let store = EventStore::default();
        let first = event("node-a", 1);
        let second = event("node-b", 1);
        store.insert(first.clone());
        store.insert(second.clone());

        let mut summary = store.summary();
        summary.sort();

        let mut expected = vec![first.id().clone(), second.id().clone()];
        expected.sort();
        assert_eq!(summary, expected);

        let have = HashSet::from([first.id().clone()]);
        assert_eq!(store.missing_from(&have, 10), vec![second]);
    }
}
