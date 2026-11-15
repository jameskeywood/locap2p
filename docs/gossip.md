# Gossip

## Overview

The gossip layer delivers every correct event to every correct node using two
mechanisms:

- **Eager push** for low publish latency.
- **Periodic pull** for convergence after loss, disconnection, or late joins.

The application-facing side is described in [ipc.md](ipc.md), and peer discovery
is described in [discovery.md](discovery.md).

## Transport and discovery

libp2p provides transports, connections, and mDNS discovery. Events are carried
over a dedicated versioned synchronization protocol using libp2p's
request/response channel. Requests and responses are opaque byte strings framed
with CBOR; the gossip layer performs its own JSON encoding inside those bytes.

mDNS discovers peers. On discovery, the network layer registers the peer's
addresses with the swarm, allowing request/response traffic to dial the peer
automatically. Peer discovery and expiry are surfaced to the application as
peer-joined and peer-left notifications.

## Message exchange

The synchronization protocol has two request types and two response types:

| Exchange | Request | Response |
|---|---|---|
| Pull | The sender's digest of known event identifiers | Events the requester lacks |
| Push | One or more newly published events | An acknowledgement |

Every event received in a push or pull response is validated before it is
integrated.

## Pull anti-entropy

Every sync interval, a node:

1. Selects up to the configured fanout of peers, rotating through the peer set
   so that all peers are eventually contacted.
2. Sends each selected peer its digest.
3. Receives the events it lacks in the responses.
4. Verifies and integrates those events.
5. Notifies the application only about events that were newly integrated.

The responder compares the requester's digest with its own event set and returns
up to the configured maximum number of missing events. Responses are capped to
keep individual messages bounded. If more events are missing, they are obtained
on later pull rounds.

## Eager push

When an event is published locally:

1. The gossip layer integrates it into its local event set.
2. If the event is new, gossip notifies the application and sends it to up to
   the configured fanout of peers.
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
- Outbound pushes and pulls may trigger dials through the registered mDNS
  addresses.

## Configuration

| Setting | Default | Meaning |
|---|---|---|
| Sync interval | 2 seconds | Time between pull rounds |
| Fanout | 3 | Peers contacted per pull or push |
| Response limit | 256 | Maximum events returned in one pull response |

Fanout is applied independently to each pull round and each eager push. Peer
selection rotates through the peer set so that repeated rounds cover different
peers.
