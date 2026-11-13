# Peer Discovery

## Overview

Nodes find each other on the local network using libp2p mDNS. There are no
bootstrap peers, DHT lookups, or relays; discovery is limited to the LAN.

The exchange protocol is described in [gossip.md](gossip.md).

## Node identity

Each host creates a fresh libp2p identity when it starts. The resulting
`node_id` is the libp2p `PeerId`, is used as the event author, and is sent to
the application in the `ready` IPC frame. It changes when the host restarts.

## Listening

By default the host listens on:

| Transport | Address |
|---|---|
| TCP + Noise + Yamux | `/ip4/0.0.0.0/tcp/0` |
| QUIC | `/ip4/0.0.0.0/udp/0/quic-v1` |

Port `0` asks the operating system to assign an ephemeral port.

## Discovery flow

1. mDNS advertises the host and listens for other nodes.
2. When peers are discovered, the network layer groups all reported addresses
   by peer ID and registers them with the swarm.
3. The peer is reported once as `PeerDiscovered`, which gossip turns into
   `PeerJoined` for the application.
4. When mDNS expires a peer, the network layer removes it from the discovered
   set and reports `PeerExpired`. Gossip turns this into `PeerLeft`.
5. Discovery does not connect to peers by itself. The first pull or push to a
   peer triggers a libp2p dial using the registered addresses.

Connections are authenticated by libp2p. mDNS itself is unauthenticated local
network discovery.

## Scope and limits

- Discovery is LAN-wide. There is no application-level network ID or topic
  filter, so every discovered peer is treated as a gossip peer.
- A discovered peer that does not support `/locap2p/sync/1` cannot exchange
  events; its sync requests fail.
- mDNS expiry removes the peer from gossip selection but does not close an
  existing connection or remove its addresses from the swarm.
