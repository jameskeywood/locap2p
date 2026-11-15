# Events and Event State

## Events

An event is an immutable application record. Every event carries:

| Field | Meaning |
|---|---|
| Identifier | Content-addressed identifier derived from the event |
| Author | Node that created the event |
| Sequence | Per-author sequence number |
| Timestamp | Creator's claimed creation time |
| Payload | Application-defined JSON value |

The identifier is a BLAKE3 hash over the author, sequence number, timestamp,
payload length, and payload. Because the identifier is derived from the content,
any change produces a different identifier, and two events with the same
identifier are the same event. A receiver recomputes the identifier before
accepting an event; if it does not match, the event is rejected.

Timestamps are supplied by the author and are not trusted as wall-clock truth.
They are part of the event content and therefore part of its identity.

## Replicated event state

Each node maintains a grow-only set of events. Events are only added; they are
never modified or removed. Adding an event is set union: if the event is already
known, the addition has no effect.

Set union is commutative, associative, and idempotent. Nodes that observe the
same events therefore converge to the same state regardless of the order or
number of times events are delivered. This makes the replicated state a CRDT.

## Ownership

The gossip layer owns the replicated state. The host creates locally authored
events but does not hold or mutate the replicated set itself.

Applications hand payloads to the host. The host adds the event metadata,
computes the identifier, and passes the completed event to gossip. Gossip then
integrates it, deduplicates it, and makes it available for synchronization with
peers.

All integration, deduplication, and answering of pull requests happens inside
the gossip layer. Because the gossip event loop is the only reader and writer of
the event set, no shared ownership or locking is required.

## Event lifecycle

Local events follow this path:

1. The application asks the host to publish a payload.
2. The host adds the author, sequence number, and timestamp, then computes the
   event identifier.
3. Gossip adds the event to its local set.
4. If the event was not already present, gossip notifies the application that a
   new event has been integrated and eagerly pushes it to selected peers.

Remote events follow a similar path:

1. The event arrives in a push or a pull response.
2. The receiver verifies the event identifier.
3. The event is added to the local set.
4. The application is notified only if the event was not already present.

Local and remote events use the same notification stream. This gives the
application one place to observe updates and prevents duplicate notifications
when an event arrives through both push and pull.

## Digest

A node's digest is the sorted list of every event identifier it knows. A peer
can compare its own state with a digest to determine precisely which events the
sender lacks.

The digest is exact and its size grows with the event history. Nodes exchange
digests during periodic pull rounds, which is how missing events are discovered
and repaired.
