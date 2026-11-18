# locap2p

Trust-based locality-aware framework for distributed applications.

## Local demo

Bootstrap and relay run as separate services. Start each from its own terminal:

```sh
nix develop

cp config/services/bootstrap.example.toml config/services/bootstrap.local.toml
cp config/services/relay.example.toml config/services/relay.local.toml

cargo run -p locap2p-bootstrap -- --config config/services/bootstrap.local.toml
cargo run -p locap2p-relay -- --config config/services/relay.local.toml
```

Each service prints the endpoint to put into the application configuration. Copy `config/application.example.json` to `config/application.local.json`, fill in those endpoints, then run:

```sh
./run_demo.sh
```

## Useful resources

### JIRA

https://jameskeywood.atlassian.net/jira/software/projects/LOC/summary

### NixOS
- Rust Rover: https://nixos.wiki/wiki/Rust_Rover
- Rust overlay: https://github.com/oxalica/rust-overlay

### Build tools
- Cargo: https://docs.rs/cargo/latest/cargo/
  - Workspaces: https://doc.rust-lang.org/cargo/reference/workspaces.html
- Gradle: https://docs.gradle.org/
- Checkstyle plugin: https://checkstyle.org/getting-started.html

### Programming
- Rust book: https://doc.rust-lang.org/book/

### Networking
- libp2p docs: https://libp2p.io/docs/
	- AutoNAT: https://libp2p.io/docs/autonat/
	- Circuit Relay: https://libp2p.io/docs/circuit-relay/
	- Hole Punching: https://libp2p.io/docs/hole-punching/
	- DCUtR: https://libp2p.io/docs/dcutr/
- Rust libp2p: https://github.com/libp2p/rust-libp2p

### IPC
- Overview: https://en.wikipedia.org/wiki/Inter-process_communication
- NDJSON: https://en.wikipedia.org/wiki/JSON_streaming
- Unix Domain Sockets: https://kernel-internals.org/ipc/unix-sockets/
