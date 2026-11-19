set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"

export BOOTSTRAP_CONFIG="$ROOT_DIR/config/services/bootstrap.local.toml"
export RELAY_CONFIG="$ROOT_DIR/config/services/relay.local.toml"

docker compose \
    --project-directory "$DEPLOY_DIR" \
    -f "$DEPLOY_DIR/compose.yaml" \
    down
