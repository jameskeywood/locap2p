#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
source "$SCRIPT_DIR/_common.sh"

bootstrap_endpoint=""
relay_endpoint=""

for _ in $(seq 1 30); do
    bootstrap_endpoint="$("${compose[@]}" logs --no-color bootstrap 2>&1 | grep -oE '/(ip4|dns|dns4|dns6)/[^"]+/p2p/[A-Za-z0-9]+' | head -n 1 || true)"
    relay_endpoint="$("${compose[@]}" logs --no-color relay 2>&1 | grep -oE '/(ip4|dns|dns4|dns6)/[^"]+/p2p/[A-Za-z0-9]+' | head -n 1 || true)"
    if [[ -n "$bootstrap_endpoint" && -n "$relay_endpoint" ]]; then
        break
    fi
    sleep 2
done

if [[ -z "$bootstrap_endpoint" || -z "$relay_endpoint" ]]; then
    echo "failed to read service endpoints from the logs" >&2
    "${compose[@]}" ps >&2 || true
    "${compose[@]}" logs --tail=100 --no-color >&2 || true
    exit 1
fi

echo "Bootstrap: $bootstrap_endpoint"
echo "Relay: $relay_endpoint"
