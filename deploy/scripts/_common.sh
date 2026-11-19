#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"
PROFILE="${PROFILE:-local}"

if [[ ! -f "$DEPLOY_DIR/.env" ]]; then
    echo "missing $DEPLOY_DIR/.env; copy deploy/.env.example to deploy/.env" >&2
    exit 1
fi

export BOOTSTRAP_CONFIG="$ROOT_DIR/config/services/bootstrap.$PROFILE.toml"
export RELAY_CONFIG="$ROOT_DIR/config/services/relay.$PROFILE.toml"

compose=(
    docker compose
    --project-directory "$DEPLOY_DIR"
    --env-file "$DEPLOY_DIR/.env"
    -f "$DEPLOY_DIR/compose.yaml"
)
