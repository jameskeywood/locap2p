set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"

if [[ -f "$DEPLOY_DIR/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source "$DEPLOY_DIR/.env"
    set +a
fi

missing=0
for config in bootstrap.local.toml relay.local.toml; do
    if [[ ! -f "$ROOT_DIR/config/services/$config" ]]; then
        echo "missing config/services/$config; copy it from the matching .example.toml" >&2
        missing=1
    fi
done
if [[ "$missing" -ne 0 ]]; then
    exit 1
fi

mkdir -p "$ROOT_DIR/state/bootstrap" "$ROOT_DIR/state/relay"

export BOOTSTRAP_CONFIG="$ROOT_DIR/config/services/bootstrap.local.toml"
export RELAY_CONFIG="$ROOT_DIR/config/services/relay.local.toml"

docker compose \
    --project-directory "$DEPLOY_DIR" \
    -f "$DEPLOY_DIR/compose.yaml" \
    up -d --build

docker compose \
    --project-directory "$DEPLOY_DIR" \
    -f "$DEPLOY_DIR/compose.yaml" \
    ps
