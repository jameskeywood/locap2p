set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"

if [[ -f "$DEPLOY_DIR/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source "$DEPLOY_DIR/.env"
    set +a
fi

DROPLET_NAME="${DROPLET_NAME:-locap2p-demo}"

if ! command -v doctl >/dev/null 2>&1; then
    echo "doctl is required" >&2
    exit 1
fi

if [[ -n "${DIGITALOCEAN_ACCESS_TOKEN:-}" ]]; then
    export DIGITALOCEAN_ACCESS_TOKEN
fi

if ! doctl compute droplet get "$DROPLET_NAME" >/dev/null 2>&1; then
    echo "droplet '$DROPLET_NAME' does not exist"
    exit 0
fi

echo "Deleting droplet '$DROPLET_NAME'..."
doctl compute droplet delete "$DROPLET_NAME" --force
echo "Deleted."
