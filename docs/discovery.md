# Peer Discovery

## Overview

Nodes find other members of their session through a bootstrap service. The
bootstrap is a lightweight control-plane rendezvous point. It does not carry
application traffic and is never treated as a gossip peer.

Relay and direct-connection behaviour is described in
[connectivity.md](connectivity.md). Node identity is described in
[identity.md](identity.md).

## Configuration

Discovery is configured by the application:

```json
{
  "discovery": {
    "bootstrap": [
      "/ip4/127.0.0.1/tcp/4002/p2p/12D3KooW..."
    ],
    "membership_ttl_ms": 10000,
    "expiry_grace_ms": 5000
  }
}
```

Every bootstrap endpoint must include `/p2p/<peer-id>` so the expected service
peer is pinned.

The bootstrap service prints its complete endpoint block on startup, or with
`--print-info`. Copy that endpoint into `discovery.bootstrap`.

## Sessions

A node belongs to an application and session:

```json
{
  "application": {
    "id": "locap2p-demo/1",
    "session": "ABC123"
  }
}
```

The bootstrap registry is keyed by the application ID and session ID. Nodes in
different applications or sessions do not see each other through the same
bootstrap service. The synchronization protocol is also scoped by the
application/session hash.

## Bootstrap flow

1. The node starts listening on its configured transport addresses.
2. It announces itself to the bootstrap service with its application, session,
   addresses and membership TTL.
3. The bootstrap returns the other members of the session.
4. The node registers returned peer addresses with libp2p and reports newly seen
   peers as `PeerDiscovered`.
5. The node repeats the announce at roughly one third of the membership TTL.
6. If a peer has not appeared in a successful bootstrap response for the TTL plus
   the configured expiry grace period, the node reports `PeerExpired`.

The bootstrap stores registrations in memory. A registration expires after its
TTL, so a crashed node disappears without an explicit leave message.

## Scope and limits

- The bootstrap service is a discovery dependency. If it is unavailable,
  existing connections continue, but the node cannot learn new peers until it
  reconnects.
- Discovery is private only to the extent that the session ID is hard to guess.
  Treat session IDs as capabilities if sessions should not be public.
- Bootstrap resource limits are configured statically by the service operator.
