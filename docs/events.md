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

## Ownership

`EventSet` is owned by the gossip layer. The host does not hold or mutate
replicated state. It creates locally authored events and hands them to gossip
through `Gossip::publish`.

All integration, deduplication, and answering pull requests happens inside the
gossip layer. Because the gossip event loop is the only reader and writer of the
event set, no shared ownership or locking is required.

## Event lifecycle

Local event:

1. `Host::create_event` creates the event without storing it.
2. `Gossip::publish` encodes the push and integrates the event into the
   gossip-owned `EventSet`.
3. If the event is new, gossip queues `GossipEvent::Event` and pushes it to
   selected peers.
4. The host forwards the queued event to the application through the normal
   gossip event stream.

Remote event:

1. The gossip layer receives the event in a `Push` or `PullResponse`.
2. The event ID is verified.
3. The event is integrated into the gossip-owned `EventSet`.
4. `GossipEvent::Event` is emitted only when insertion reports a new event.
5. The host forwards it to the application.

`GossipEvent::Event` means "newly integrated event", whether it was created
locally or received from a peer. This gives the application one event stream and
prevents duplicate IPC frames when an event arrives through both push and pull.

## Digest

The digest is the sorted list of all known event IDs:

```rust
EventSet::summary() -> Vec<EventId>
```

A peer can use it to determine precisely which events the sender lacks. The
digest is exact, and its size grows with event history.
