set -euo pipefail

cargo check --workspace

cargo check -p locap2p-gossip --all-targets

cargo fmt --all -- --check

cargo clippy --workspace --all-targets -- -D warnings

cargo test --workspace

./apps/demo/gradlew --project-dir apps/demo :app:check
