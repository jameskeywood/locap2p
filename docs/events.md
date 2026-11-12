# Events and Event State

## Events

An event is an immutable application record:

| Field | Meaning |
|---|---|
| `id` | Content-addressed identifier |
| `author` | Node that created the event |
| `seq` | Per-author sequence number |
| `timestamp_nanos` | Creator's claimed creation time |
| `payload` | Application-defined JSON value |

The ID is a BLAKE3 hash over the author, sequence, timestamp, payload length, and
payload. `Event::verify_id` recomputes the hash and rejects mismatches. Two
events with the same ID are the same event.

## EventSet

`EventSet` is a grow-only set of events stored in a `BTreeMap<EventId, Event>`.

Insertion is set union. Union is commutative, associative, and idempotent, so
nodes that observe the same events converge to the same set regardless of
delivery order. This makes `EventSet` a CRDT.

| Operation | Purpose |
|---|---|
| `insert` | Adds an event; returns `false` if already present |
| `contains` | Checks whether an event ID is known |
| `merge` | Unions another set into this one |
| `summary` | Returns every known event ID, sorted |
| `missing_from` | Returns events whose IDs are absent from a given set |

## EventStore

`EventStore` is a cloneable handle to the node's shared event set:

```rust
#[derive(Clone, Default)]
pub struct EventStore {
    inner: Arc<RwLock<EventSet>>,
}
```

The host and the gossip layer share one store. This gives the node one logical
event set, and allows the gossip layer to answer peer requests while the host
creates and observes events.

The lock is `std::sync::RwLock`. Critical sections are short and synchronous;
the lock is never held across an `await`.

| Method | Purpose |
|---|---|
| `insert(event)` | Inserts an event; returns `true` only if new |
| `contains(id)` | Checks whether the event is known |
| `summary()` | Returns every known event ID, sorted |
| `missing_from(have, limit)` | Returns up to `limit` events not in `have` |
| `len()` | Number of stored events |

`EventStore` is a shared handle around `EventSet`, not a separate CRDT.

## Event lifecycle

Local event:

1. `Host::create_event` creates the event and inserts it into the `EventStore`.
2. `Gossip::publish` eagerly pushes it to selected peers.
3. Peers that miss the push obtain it through periodic pull.

Remote event:

1. The gossip layer receives the event in a `Push` or `PullResponse`.
2. The event ID is verified.
3. The event is inserted into the `EventStore`.
4. `GossipEvent::Event` is emitted only when insertion reports a new event.
5. The host observes it idempotently and forwards it to the application.

`GossipEvent::Event` means "newly merged event", not merely "an event arrived on
the wire". This prevents duplicate IPC frames when an event arrives through both
push and pull.

## Digest

The digest is the sorted list of all known event IDs:

```rust
store.summary() -> Vec<EventId>
```

A peer can use it to determine precisely which events the sender lacks. The
digest is exact, and its size grows with event history.
