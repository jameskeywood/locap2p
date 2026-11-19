#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"

if [[ ! -f "$DEPLOY_DIR/.env" ]]; then
    echo "missing $DEPLOY_DIR/.env; copy deploy/.env.example to deploy/.env" >&2
    exit 1
fi

set -a
# shellcheck disable=SC1091
source "$DEPLOY_DIR/.env"
set +a

: "${DROPLET_NAME:=locap2p-demo}"
: "${LOCAP2P_IMAGE:?set LOCAP2P_IMAGE in deploy/.env}"
: "${LOCAP2P_UID:?set LOCAP2P_UID in deploy/.env}"
: "${LOCAP2P_GID:?set LOCAP2P_GID in deploy/.env}"

REMOTE_ROOT="/opt/locap2p"
PROFILE="remote"
BOOTSTRAP_CONFIG="$ROOT_DIR/config/services/bootstrap.$PROFILE.toml"
RELAY_CONFIG="$ROOT_DIR/config/services/relay.$PROFILE.toml"

for config in "$BOOTSTRAP_CONFIG" "$RELAY_CONFIG"; do
    if [[ ! -f "$config" ]]; then
        echo "missing $config" >&2
        exit 1
    fi
done

if ! command -v doctl >/dev/null 2>&1; then
    echo "doctl is required" >&2
    exit 1
fi

IP="$(doctl compute droplet get "$DROPLET_NAME" --format PublicIPv4 --no-header 2>/dev/null || true)"
if [[ -z "$IP" ]]; then
    echo "droplet '$DROPLET_NAME' not found; run vps-up.sh first" >&2
    exit 1
fi

SSH_ARGS=(
    -o StrictHostKeyChecking=accept-new
    -o "UserKnownHostsFile=$DEPLOY_DIR/known_hosts"
    -o ConnectTimeout=10
)

remote() {
    ssh "${SSH_ARGS[@]}" "root@$IP" "$@"
}

echo "Waiting for SSH..."
for _ in $(seq 1 60); do
    if remote true >/dev/null 2>&1; then
        break
    fi
    sleep 5
done

if ! remote true >/dev/null 2>&1; then
    echo "SSH did not become available on the droplet" >&2
    exit 1
fi

echo "Waiting for Docker..."
for _ in $(seq 1 60); do
    if remote "command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1" >/dev/null 2>&1; then
        break
    fi
    sleep 5
done

if ! remote "command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1" >/dev/null 2>&1; then
    echo "Docker did not become available on the droplet" >&2
    exit 1
fi

remote "install -d -m 750 '$REMOTE_ROOT/deploy' '$REMOTE_ROOT/config/services' '$REMOTE_ROOT/state/bootstrap' '$REMOTE_ROOT/state/relay'"

tar -C "$ROOT_DIR" -cf - \
    deploy/.env \
    deploy/compose.yaml \
    deploy/scripts/_common.sh \
    deploy/scripts/up.sh \
    deploy/scripts/down.sh \
    deploy/scripts/endpoints.sh \
    "config/services/bootstrap.$PROFILE.toml" \
    "config/services/relay.$PROFILE.toml" \
    | remote "tar -xf - -C '$REMOTE_ROOT'"

remote "chmod 600 '$REMOTE_ROOT/deploy/.env'"
remote "chown -R '$LOCAP2P_UID:$LOCAP2P_GID' '$REMOTE_ROOT/state'"

if [[ -n "${GHCR_TOKEN:-}" ]]; then
    : "${GHCR_USER:?set GHCR_USER when GHCR_TOKEN is set}"
    printf '%s' "$GHCR_TOKEN" | remote "docker login ghcr.io -u '$GHCR_USER' --password-stdin"
fi

echo "Starting services..."
remote "cd '$REMOTE_ROOT' && PROFILE='$PROFILE' ./deploy/scripts/up.sh --pull"

echo "Waiting for service endpoints..."
remote "cd '$REMOTE_ROOT' && PROFILE='$PROFILE' ./deploy/scripts/endpoints.sh"
