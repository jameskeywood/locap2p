set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"

if [[ -f "$DEPLOY_DIR/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source "$DEPLOY_DIR/.env"
    set +a
fi

: "${DO_SSH_KEY:?set DO_SSH_KEY to a DigitalOcean SSH key fingerprint or name}"

DROPLET_NAME="${DROPLET_NAME:-locap2p-demo}"
DO_REGION="${DO_REGION:-lon1}"
DO_SIZE="${DO_SIZE:-s-1vcpu-1gb}"
DO_IMAGE="${DO_IMAGE:-ubuntu-24-04-x64}"

if ! command -v doctl >/dev/null 2>&1; then
    echo "doctl is required" >&2
    exit 1
fi

if [[ -n "${DIGITALOCEAN_ACCESS_TOKEN:-}" ]]; then
    export DIGITALOCEAN_ACCESS_TOKEN
fi

if ! doctl compute droplet get "$DROPLET_NAME" >/dev/null 2>&1; then
    echo "Creating droplet '$DROPLET_NAME' in $DO_REGION..."
    doctl compute droplet create "$DROPLET_NAME" \
        --region "$DO_REGION" \
        --size "$DO_SIZE" \
        --image "$DO_IMAGE" \
        --ssh-keys "$DO_SSH_KEY" \
        --user-data-file "$DEPLOY_DIR/scripts/cloud-init.yaml" \
        --wait >/dev/null
fi

IP=""
for _ in $(seq 1 30); do
    IP="$(doctl compute droplet get "$DROPLET_NAME" --format PublicIPv4 --no-header 2>/dev/null || true)"
    [[ -n "$IP" ]] && break
    sleep 2
done

if [[ -z "$IP" ]]; then
    echo "failed to obtain the droplet public IP" >&2
    exit 1
fi

echo "Droplet: $DROPLET_NAME"
echo "Public IP: $IP"
