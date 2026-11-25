# Configuration

## Overview

The application owns the node configuration. It loads a JSON file and passes the
resulting configuration to the host in the initial IPC `configure` frame over
the Unix socket. The host does not read the configuration file itself.

A complete example is committed as `config/application.example.json`. Copy it to
`config/local.json` and fill in the endpoints printed by the bootstrap and relay
services. Local and remote runtime configurations are ignored by Git.

## Fields

| Field | Meaning |
|---|---|
| `schema_version` | Configuration contract version; currently `1` |
| `application.id` | Application/protocol namespace |
| `application.session` | Game, lobby or session code |
| `identity.path` | Persistent libp2p identity key file |
| `network.listen_addrs` | Local bind addresses for libp2p transports |
| `network.request_timeout_ms` | Request/response timeout |
| `discovery.bootstrap` | Bootstrap seed gateway multiaddrs, including `/p2p/<id>` |
| `connectivity.relay` | Relay service multiaddrs, including `/p2p/<id>` |
| `connectivity.reserve_relay` | Request a relay circuit reservation |
| `connectivity.prefer_direct` | Rank direct addresses ahead of relayed addresses when dialing |
| `connectivity.enable_dcutr` | Attempt direct connection upgrades through relay |
| `gossip.sync_interval_ms` | Anti-entropy pull interval |
| `gossip.fanout` | Peers contacted per pull or eager push |
| `gossip.max_events_per_response` | Maximum events in one pull response |
| `gossip.membership_ttl_ms` | Membership record lifetime |
| `gossip.expiry_grace_ms` | Local tolerance before reporting a member left |

Configuration is loaded as supplied.

## Identity

Identity creation, peer IDs and the identify protocol are described in
[identity.md](identity.md).

## Local versus remote

The configuration shape is identical for local and remote operation. Only the
bootstrap and relay endpoints differ. Start the services separately and copy the
endpoints they print into `discovery.bootstrap` and `connectivity.relay`. The same
schema is used when the endpoints point at public DigitalOcean addresses.

## Service configuration

Bootstrap and relay services have separate static operator-owned configuration.
Application configuration contains only their endpoints. Applications cannot
change service limits or reservation settings.

Bootstrap seed behaviour is configured with `seed_ttl_secs`,
`max_seeds_per_session` and `max_seeds_returned` in the bootstrap service TOML.

Start a service with:

```sh
locap2p-bootstrap --config config/services/bootstrap.local.toml
locap2p-relay --config config/services/relay.local.toml
```

When listening on `0.0.0.0`, configure `advertise_addrs` with the address
clients should use.
