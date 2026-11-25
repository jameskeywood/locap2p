set -euo pipefail

cargo check --workspace

cargo fmt --all -- --check

cargo clippy --workspace --all-targets -- -D warnings

cargo test --workspace

./apps/demo/gradlew --project-dir apps/demo :app:check

echo

echo "Rust line count"
git ls-files | grep -e "\.rs" | xargs wc -l | grep "total"

echo "Java line count"
git ls-files | grep -e "\.java" | xargs wc -l | grep "total"

echo "Combined line count"
git ls-files | grep -e "\.rs\|\.java" | xargs wc -l | grep "total"
