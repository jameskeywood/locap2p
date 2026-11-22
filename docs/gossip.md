# Gossip

## Overview

The gossip layer delivers every correct event to every correct node using two
mechanisms:

- **Eager push** for low publish latency.
- **Periodic pull** for convergence after loss, disconnection, or late joins.

Gossip also owns session membership. Bootstrap only provides initial seed
candidates; members are established, refreshed and expired through gossip.

The application-facing side is described in [ipc.md](ipc.md), seed discovery in
[discovery.md](discovery.md), and relay/direct connectivity in
[connectivity.md](connectivity.md).

## Transport and membership

libp2p provides transports, connections, and connectivity. Events and membership
records are carried over a session-scoped synchronization protocol using
libp2p's request/response channel. Requests and responses are opaque byte strings
framed with CBOR; the gossip layer performs its own JSON encoding inside those
bytes.

Each node has a versioned membership record:

```text
PeerRecord {
    peer_id,
    addresses,
    version,
    expires_at_ms,
    state: active | left,
}
```

Records are merged by version. A higher version replaces a lower version. An
`active` record adds the peer to the member set; a `left` record removes it and
acts as a tombstone.

Membership records are included in every pull, pull response and push. This
full-state exchange keeps the implementation simple for small sessions. Peers
renew their own records periodically, extending `expires_at_ms`. A crashed peer
disappears when its last record expires, with a local grace period.

Bootstrap seed candidates are used for the initial membership exchange only.
They are not reported as members until an active membership record is observed.

## Message exchange

| Exchange | Request | Response |
|---|---|---|
| Pull | Event digest and membership records | Missing events and membership records |
| Push | New events and membership records | An acknowledgement |

Membership is merged before application events are accepted. Application events
are served only to active members.

Every event received in a push or pull response is validated before it is
integrated.

## Pull anti-entropy

Every sync interval, a node:

1. Refreshes its local membership record with its current reachable addresses.
2. Selects up to the configured fanout of peers, rotating through active
   members and any seed candidates.
3. Sends each selected peer its event digest and membership records.
4. Receives missing events and membership records in the responses.
5. Merges membership changes.
6. Verifies and integrates events from active members.
7. Notifies the application only about newly integrated events and membership
   transitions.

The responder compares the requester's digest with its own event set and returns
up to the configured maximum number of missing events. Responses are capped to
keep individual messages bounded. If more events are missing, they are obtained
on later pull rounds.

## Eager push

When an event is published locally:

1. The gossip layer integrates it into its local event set.
2. If the event is new, gossip notifies the application and sends it to up to
   the configured fanout of active members.
3. Each receiver validates and integrates the event, then acknowledges it.

Push is best-effort and is not forwarded on receipt, so only the first hop is
immediate. Nodes further away obtain the event through their next pull.

If a push fails, the peer is offline, or the publisher has no peers, pull
repairs the missing state later. Duplicate events received by both push and pull
are ignored because the event set only grows.

## Event loop

The gossip layer runs a single event loop that serves three sources of work:
queued application notifications, the periodic pull timer, and network events.

Queued notifications are delivered before more network events are processed.
Locally published events enter the queue when they are integrated; remote events
enter it only when they are new. The network only makes progress while this loop
is being polled.

Keeping all state access inside one event loop means the replicated state needs
no shared ownership or locking.

## Failure handling

- A failed request is logged and ignored; the next pull retries.
- Malformed or unexpected messages are discarded.
- Duplicate events are idempotently ignored.
- Offline nodes catch up when they reconnect and pull again.
- Late joiners begin with an empty digest and pull the full history in batches.
- Membership records expire locally after the configured TTL plus grace period.
- Seed candidates that fail are removed from the candidate set.

## Configuration

| Setting | Default | Meaning |
|---|---|---|
| Sync interval | 1 second (demo) | Time between pull rounds |
| Fanout | 3 | Peers contacted per pull or push |
| Response limit | 256 | Maximum events returned in one pull response |
| Membership TTL | 10 seconds | Lifetime of a membership record |
| Expiry grace | 5 seconds | Local tolerance before reporting a member left |

Fanout is applied independently to each pull round and each eager push. Peer
selection rotates through the member set so that repeated rounds cover different
peers.
