#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"

if [[ -f "$DEPLOY_DIR/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source "$DEPLOY_DIR/.env"
    set +a
fi

DROPLET_NAME="${DROPLET_NAME:-locap2p-demo}"
REMOTE_ROOT="/opt/locap2p"
PROFILE="remote"

if ! command -v doctl >/dev/null 2>&1; then
    echo "doctl is required" >&2
    exit 1
fi

IP="$(doctl compute droplet get "$DROPLET_NAME" --format PublicIPv4 --no-header 2>/dev/null || true)"
if [[ -z "$IP" ]]; then
    echo "droplet '$DROPLET_NAME' not found"
    exit 0
fi

SSH_ARGS=(
    -o StrictHostKeyChecking=accept-new
    -o "UserKnownHostsFile=$DEPLOY_DIR/known_hosts"
    -o ConnectTimeout=10
)

ssh "${SSH_ARGS[@]}" "root@$IP" \
    "cd '$REMOTE_ROOT' && PROFILE='$PROFILE' ./deploy/scripts/down.sh"
