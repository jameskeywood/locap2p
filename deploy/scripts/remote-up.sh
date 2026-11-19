set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"

if [[ -f "$DEPLOY_DIR/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source "$DEPLOY_DIR/.env"
    set +a
fi

: "${LOCAP2P_IMAGE:?set LOCAP2P_IMAGE to the image to deploy}"

DROPLET_NAME="${DROPLET_NAME:-locap2p-demo}"
REMOTE_UID="${LOCAP2P_UID:-1000}"
REMOTE_GID="${LOCAP2P_GID:-1000}"
REMOTE_ROOT="/opt/locap2p"
BOOTSTRAP_CONFIG="$ROOT_DIR/config/services/bootstrap.remote.toml"
RELAY_CONFIG="$ROOT_DIR/config/services/relay.remote.toml"

if [[ ! -f "$BOOTSTRAP_CONFIG" ]]; then
    echo "missing $BOOTSTRAP_CONFIG" >&2
    exit 1
fi
if [[ ! -f "$RELAY_CONFIG" ]]; then
    echo "missing $RELAY_CONFIG" >&2
    exit 1
fi

if ! command -v doctl >/dev/null 2>&1; then
    echo "doctl is required" >&2
    exit 1
fi

if [[ -n "${DIGITALOCEAN_ACCESS_TOKEN:-}" ]]; then
    export DIGITALOCEAN_ACCESS_TOKEN
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

copy_to_remote() {
    scp "${SSH_ARGS[@]}" "$@"
}

echo "Waiting for SSH..."
for _ in $(seq 1 60); do
    if remote true >/dev/null 2>&1; then
        break
    fi
    sleep 5
done

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

remote "ufw allow OpenSSH >/dev/null 2>&1 || true"
remote "ufw allow 4001/tcp >/dev/null 2>&1 || true"
remote "ufw allow 4002/tcp >/dev/null 2>&1 || true"
remote "ufw --force enable >/dev/null 2>&1 || true"

remote "install -d -m 750 '$REMOTE_ROOT/deploy' '$REMOTE_ROOT/config/services' '$REMOTE_ROOT/state/bootstrap' '$REMOTE_ROOT/state/relay'"

copy_to_remote "$DEPLOY_DIR/compose.yaml" "root@$IP:$REMOTE_ROOT/deploy/compose.yaml"
copy_to_remote "$BOOTSTRAP_CONFIG" "root@$IP:$REMOTE_ROOT/config/services/bootstrap.remote.toml"
copy_to_remote "$RELAY_CONFIG" "root@$IP:$REMOTE_ROOT/config/services/relay.remote.toml"
remote "chown -R '$REMOTE_UID:$REMOTE_GID' '$REMOTE_ROOT/state'"

if [[ -n "${GHCR_TOKEN:-}" ]]; then
    : "${GHCR_USER:?set GHCR_USER when GHCR_TOKEN is set}"
    printf '%s' "$GHCR_TOKEN" | remote "docker login ghcr.io -u '$GHCR_USER' --password-stdin"
fi

COMPOSE_ENV="export LOCAP2P_IMAGE='$LOCAP2P_IMAGE' BOOTSTRAP_CONFIG=../config/services/bootstrap.remote.toml RELAY_CONFIG=../config/services/relay.remote.toml LOCAP2P_UID='$REMOTE_UID' LOCAP2P_GID='$REMOTE_GID' RUST_LOG='${RUST_LOG:-info}'"

echo "Starting services..."
remote "cd '$REMOTE_ROOT/deploy' && $COMPOSE_ENV && docker compose pull && docker compose up -d"

echo "Waiting for service endpoints..."
BOOTSTRAP_ENDPOINT=""
RELAY_ENDPOINT=""
for _ in $(seq 1 30); do
    BOOTSTRAP_ENDPOINT="$(remote "cd '$REMOTE_ROOT/deploy' && docker compose logs --no-color bootstrap 2>&1" | grep -oE '/(ip4|dns|dns4|dns6)/[^"]+/p2p/[A-Za-z0-9]+' | head -n 1 || true)"
    RELAY_ENDPOINT="$(remote "cd '$REMOTE_ROOT/deploy' && docker compose logs --no-color relay 2>&1" | grep -oE '/(ip4|dns|dns4|dns6)/[^"]+/p2p/[A-Za-z0-9]+' | head -n 1 || true)"
    if [[ -n "$BOOTSTRAP_ENDPOINT" && -n "$RELAY_ENDPOINT" ]]; then
        break
    fi
    sleep 2
done

if [[ -z "$BOOTSTRAP_ENDPOINT" || -z "$RELAY_ENDPOINT" ]]; then
    echo "failed to read service endpoints from the remote logs" >&2
    remote "cd '$REMOTE_ROOT/deploy' && docker compose ps && docker compose logs --tail=100 --no-color" >&2 || true
    exit 1
fi

echo "Bootstrap: $BOOTSTRAP_ENDPOINT"
echo "Relay: $RELAY_ENDPOINT"
