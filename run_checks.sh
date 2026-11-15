set -euo pipefail

cargo check --workspace

cargo fmt --all -- --check

cargo clippy --workspace --all-targets -- -D warnings

cargo test --workspace

./apps/demo/gradlew --project-dir apps/demo :app:check
