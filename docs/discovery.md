# Peer Discovery

## Overview

Bootstrap is a short-lived seed gateway. It helps a node make its first
connections into a session, but it does not decide membership.

Membership is maintained and disseminated by gossip. Bootstrap responses are
only candidate peers; a peer becomes a member when its membership record is
observed through gossip.

Relay and direct-connection behaviour is described in
[connectivity.md](connectivity.md). Membership dissemination is described in
[gossip.md](gossip.md).

## Configuration

```json
{
  "discovery": {
    "bootstrap": [
      "/ip4/127.0.0.1/tcp/4002/p2p/12D3KooW..."
    ]
  }
}
```

Every bootstrap endpoint must include `/p2p/<peer-id>` so the expected service
peer is pinned.

## Seed flow

1. The node starts listening and learns its reachable addresses.
2. It periodically sends its session, peer id and addresses to the configured
   bootstrap services.
3. Bootstrap stores the node in a short-lived seed pool and returns a small
   number of other recent seeds.
4. The node registers the returned addresses with libp2p and passes the seed
   ids to gossip as candidates.
5. Gossip contacts the seeds and exchanges membership records.
6. Once a membership record is accepted, the peer becomes a member and is
   reported as `PeerJoined`.

Bootstrap does not push updates. Its seed pool expires entries after the
configured TTL, so a node that stops refreshing eventually drops out of the seed
pool.

## Scope and limits

- Bootstrap is only an introduction service. It does not track membership,
  peer-leave events, or session state beyond the short-lived seed pool.
- Returning a seed does not make that peer a member. Membership is established
  by gossip.
- Seeds can be stale. Membership gossip corrects the actual view.
- Bootstrap resource limits are configured statically by the service operator.
