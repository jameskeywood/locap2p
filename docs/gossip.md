# Gossip

## Overview

The gossip layer delivers every correct event to every correct node using two
mechanisms:

- **Eager push** for low publish latency.
- **Periodic pull** for convergence after loss, disconnection, or late joins.

## Transport and discovery

libp2p provides transports, connections, and mDNS discovery. Events are carried
over a generic libp2p request/response channel using the protocol:

```text
/locap2p/sync/1
```

Requests and responses are opaque byte strings framed with CBOR. The gossip
layer performs its own JSON encoding inside those bytes.

mDNS discovers peers. On discovery, the network layer registers the peer's
addresses with the swarm, allowing request/response to auto-dial when a message
is sent. Peer discovery and expiry are surfaced as `PeerJoined` and `PeerLeft`.

## Wire protocol

```text
Pull        { have: [EventId] }  ->  PullResponse { events: [Event] }
Push        { events: [Event] }  ->  PushAck
```

All events in `Push` and `PullResponse` are validated with `Event::verify_id`
before being integrated.

## Pull anti-entropy

Every sync interval, a node:

1. Selects up to `fanout` peers using a round-robin cursor.
2. Sends `Pull { have: store.summary() }`.
3. Receives events it lacks in a `PullResponse`.
4. Verifies and inserts them into the `EventStore`.
5. Emits only newly inserted events to the application.

The responder computes:

```rust
store.missing_from(&requester_ids, max_events_per_response)
```

Responses are capped to keep individual messages bounded. If more events are
missing, they are obtained on later pulls.

## Eager push

When an event is published:

1. It is inserted into the local `EventStore`.
2. A `Push` containing the event is sent to up to `fanout` peers.
3. Each receiver validates and integrates the event, then replies `PushAck`.

Push is best-effort and is not forwarded on receipt, so only the first hop is
immediate. Nodes further away obtain the event through their next pull.

If a push fails, the peer is offline, or the publisher has no peers, pull
repairs the missing state later. Duplicate events received by both push and pull
are ignored by `EventStore`.

## Event loop

`AntiEntropyGossip::next_event` drives both the timer and the network:

```rust
loop {
    if let Some(event) = self.pending.pop_front() {
        return Some(event);
    }

    tokio::select! {
        _ = self.ticker.tick() => self.pull_from_peers(),
        event = self.network.next_event() => self.handle_network_event(event?),
    }
}
```

Pending application events are drained before more network events are processed.
The swarm only makes progress while `next_event` is being polled.

## Failure handling

- A failed request is logged and ignored; the next pull retries.
- Malformed messages are discarded.
- Duplicate events are idempotently ignored.
- Offline nodes catch up when they reconnect and pull again.
- Late joiners begin with an empty summary and pull the full history in batches.
- Outbound pushes and pulls may trigger dials through the registered mDNS
  addresses.

## Configuration

| Setting | Default | Meaning |
|---|---|---|
| `sync_interval` | 2 seconds | Time between pull rounds |
| `fanout` | 3 | Peers contacted per pull or push |
| `max_events_per_response` | 256 | Batch size for pull responses |
