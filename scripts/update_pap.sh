#!/usr/bin/env bash

# Pull, build, and deploy the Mortimer IoT backend + frontend.
set -euo pipefail

export PATH="/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

if [ -f "/root/.cargo/env" ]; then
    # Ensure cron has the rustup-managed cargo/rustc in PATH.
    source "/root/.cargo/env"
fi

REPO_DIR="/root/pap.mortimer.nl"
REPO_URL="git@github.com:thiezn/mortimer-iot.git"
DEPLOY_ROOT="/var/www/pap.mortimer.nl"
FRONTEND_DIR="$DEPLOY_ROOT/current"
BIN_DIR="$DEPLOY_ROOT/bin"
RUNTIME_DIR="$DEPLOY_ROOT/runtime"
TMP_DIR="$DEPLOY_ROOT/tmp"
SERVICE_NAME="mortimeriot"

log() {
    printf '[%s] %s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$*"
}

require_command() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        log "ERROR: required command '$cmd' is not installed"
        exit 1
    fi
}

log "Checking required build tools"
require_command git
require_command cargo
require_command npm
require_command rsync

log "Preparing filesystem layout"
mkdir -p "$DEPLOY_ROOT" "$FRONTEND_DIR" "$BIN_DIR" "$RUNTIME_DIR" "$TMP_DIR"

if [ -d "$REPO_DIR/.git" ]; then
    log "Updating existing repository checkout at $REPO_DIR"
    cd "$REPO_DIR"
    git fetch --prune origin
    if ! git pull --ff-only origin main; then
        log "ERROR: git pull failed (non fast-forward or local changes). Resolve repo state at $REPO_DIR and retry."
        exit 1
    fi
else
    log "Cloning repository to $REPO_DIR"
    git clone "$REPO_URL" "$REPO_DIR"
    cd "$REPO_DIR"
fi

log "Building Rust backend"
cargo build --release -p mortimeriot

log "Building Svelte frontend"
cd "$REPO_DIR/frontend"

# 1. Clear out any poison state or locked files
rm -rf node_modules package-lock.json
    
# 2. Force the correct CLI endpoint inside the build shell context
npm config set registry https://registry.npmjs.org/
    
# 3. Explicitly target the API endpoint during installation
npm install --registry=https://registry.npmjs.org/
    
npm ci
npm run build

log "Deploying frontend assets"
rsync -a --delete "$REPO_DIR/frontend/dist/" "$FRONTEND_DIR/"

log "Deploying backend binary"
tmp_bin="$TMP_DIR/mortimeriot.$$.new"
install -m 0755 "$REPO_DIR/target/release/mortimeriot" "$tmp_bin"
mv -f "$tmp_bin" "$BIN_DIR/mortimeriot"

log "Applying runtime permissions"
chown -R www-data:www-data "$DEPLOY_ROOT"

if systemctl is-enabled "$SERVICE_NAME" >/dev/null 2>&1; then
    log "Restarting systemd service: $SERVICE_NAME"
    systemctl restart "$SERVICE_NAME"
else
    log "Service $SERVICE_NAME is not enabled yet; skipping restart"
fi

if systemctl is-enabled apache2 >/dev/null 2>&1; then
    log "Reloading Apache"
    systemctl reload apache2
fi

log "Deployment complete"

