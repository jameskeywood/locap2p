#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
source "$SCRIPT_DIR/_common.sh"

for config in "$BOOTSTRAP_CONFIG" "$RELAY_CONFIG"; do
    if [[ ! -f "$config" ]]; then
        echo "missing $config" >&2
        exit 1
    fi
done

mkdir -p "$BOOTSTRAP_STATE" "$RELAY_STATE"

case "${1:-}" in
    --build)
        shift
        "${compose[@]}" up -d --build "$@"
        ;;
    --pull)
        shift
        "${compose[@]}" pull
        "${compose[@]}" up -d "$@"
        ;;
    *)
        "${compose[@]}" up -d "$@"
        ;;
esac

"${compose[@]}" ps
