# Connectivity

## Overview

A relay service lets peers connect when direct connections are not possible.
Direct connections remain preferred. libp2p DCUtR may upgrade a relayed
connection to a direct connection.

Peer discovery is described in [discovery.md](discovery.md). Node identity is
described in [identity.md](identity.md).

## Configuration

Connectivity is configured by the application:

```json
{
  "connectivity": {
    "relay": [
      "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooW..."
    ],
    "reserve_relay": true,
    "prefer_direct": true,
    "enable_dcutr": true
  }
}
```

Every relay endpoint must include `/p2p/<peer-id>` so the expected relay peer is
pinned.

The relay service prints its complete endpoint block on startup, or with
`--print-info`. Copy that endpoint into `connectivity.relay`.

## Relay flow

If `reserve_relay` is enabled, the node requests a circuit reservation from each
configured relay. The relay client produces a relayed circuit address and
libp2p adds it as an external address. That address is included in bootstrap
announcements.

When peers connect, direct addresses are preferred when `prefer_direct` is true.
Otherwise the relayed circuit address is used. If `enable_dcutr` is true and both
peers support DCUtR, libp2p attempts to replace a relayed connection with a
direct connection.

## Listening addresses

The network listens on the addresses in `network.listen_addrs`. Port `0` asks the
operating system to assign an ephemeral port. These are bind addresses; they are
not necessarily dialable by remote peers. Relayed circuit addresses provide the
reliable WAN fallback.

## Scope and limits

- The relay is used only as a fallback. Direct connections avoid relay bandwidth.
- DCUtR is an optimisation and may fail depending on NAT behaviour.
- Relay resource limits are configured statically by the service operator.
