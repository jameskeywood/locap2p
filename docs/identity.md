# Identity and Identify

## Two different things

A node's **identity** is its long-lived cryptographic keypair. It determines the
node's peer ID.

libp2p **identify** is a protocol that runs after a connection is established.
It exchanges metadata such as addresses and supported protocols. It does not
create the identity.

## Creating an identity

The framework creates identities in `crates/locap2p-identity/src/lib.rs`.

When a node starts:

1. If the configured key file exists, it is loaded and decoded.
2. Otherwise a new Ed25519 keypair is generated.
3. The private key is encoded and written to the key file with owner-only
   permissions.
4. The peer ID is derived from the public key.

The same key file always produces the same peer ID. Deleting the file creates a
new identity and a new peer ID.

The identity path comes from configuration:

- node: `identity.path` in the application config,
- bootstrap: `identity_path` in the bootstrap service config,
- relay: `identity_path` in the relay service config.

Bootstrap and relay keys should be persistent. Their peer IDs are embedded in
the endpoints clients use, for example:

```text
/ip4/203.0.113.10/tcp/4002/p2p/12D3KooW...
```

## Starting libp2p with an identity

The network loads the configured identity and gives it to libp2p:

```rust
let identity = load_or_generate(&config.identity.path)?;
let swarm = SwarmBuilder::with_existing_identity(identity);
```

Because the same keypair is supplied every time, the node keeps the same peer
ID across restarts.

## What identify does

Identify runs over an authenticated connection. The peers have already proven
that they control their private keys, so identify is not an authentication
mechanism by itself.

Each peer sends information including:

- protocol version and agent version,
- public key,
- listen addresses,
- supported protocols,
- the address the remote peer observed for us.

This lets libp2p learn how to reach peers, which protocols they support, and
what public address our peer appears to have. The observed address is useful for
NAT traversal and DCUtR.

Identify is enabled for nodes, bootstrap and relay in their respective libp2p
behaviours.

## How identity, discovery and connectivity differ

| Concern | Purpose |
|---|---|
| Identity keypair | Cryptographic identity and stable peer ID |
| Identify | Exchange addresses and protocol metadata on a connection |
| Bootstrap service | Discover peers in the same application/session |
| Relay | Forward traffic when peers cannot connect directly |
| DCUtR | Attempt to upgrade a relayed connection to a direct connection |

## Practical notes

- Treat key files as secrets. Never commit them.
- Reusing a key preserves the peer ID and event author identity.
- Deleting the key makes the node appear as a new peer.
- Multiaddrs with `/p2p/<peer-id>` pin the expected service identity.
- Identify does not add peers to a session; the bootstrap service does that.
