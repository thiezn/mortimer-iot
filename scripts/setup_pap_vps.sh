#!/usr/bin/env bash

# One-time setup for Mortimer IoT on Ubuntu VPS.
set -euo pipefail

REPO_DIR="/root/pap.mortimer.nl"
REPO_URL="git@github.com:thiezn/mortimer-iot.git"
DEPLOY_ROOT="/var/www/pap.mortimer.nl"
FRONTEND_DIR="$DEPLOY_ROOT/current"
BIN_DIR="$DEPLOY_ROOT/bin"
RUNTIME_DIR="$DEPLOY_ROOT/runtime"
TMP_DIR="$DEPLOY_ROOT/tmp"
ENV_DIR="/etc/mortimeriot"
ENV_FILE="$ENV_DIR/mortimeriot.env"
SERVICE_FILE="/etc/systemd/system/mortimeriot.service"
APACHE_SITE_FILE="/etc/apache2/sites-available/pap.mortimer.nl.conf"
DOMAIN="pap.mortimer.nl"
SERVICE_NAME="mortimeriot"
DEFAULT_PORT="2111"
DEFAULT_LISTENER_IP="127.0.0.1"
UPDATE_SCRIPT_PATH="/root/update_pap.sh"
CRON_ENTRY="20 * * * * /root/update_pap.sh > /var/log/update_pap.log 2>&1"

log() {
    printf '[%s] %s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$*"
}

require_root() {
    if [ "${EUID:-$(id -u)}" -ne 0 ]; then
        log "ERROR: run this script as root"
        exit 1
    fi
}

require_ubuntu() {
    if [ "$(uname -s)" != "Linux" ]; then
        log "ERROR: this script is only supported on Ubuntu Linux VPS hosts"
        exit 1
    fi

    if [ ! -f /etc/os-release ]; then
        log "ERROR: cannot detect OS (/etc/os-release not found)"
        exit 1
    fi

    # shellcheck disable=SC1091
    source /etc/os-release
    if [ "${ID:-}" != "ubuntu" ]; then
        log "ERROR: unsupported distribution '${ID:-unknown}'. Expected ubuntu."
        exit 1
    fi
}

ensure_command() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        log "ERROR: required command '$cmd' is not installed"
        exit 1
    fi
}

install_packages() {
    log "Installing Ubuntu packages"
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y \
        apache2 \
        build-essential \
        ca-certificates \
        curl \
        git \
        libssl-dev \
        nodejs \
        npm \
        pkg-config \
        rsync \
        sqlite3
}

install_rust() {
    if command -v cargo >/dev/null 2>&1; then
        log "Rust toolchain already installed"
        return
    fi

    log "Installing Rust toolchain"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
}

setup_repo() {
    if [ -d "$REPO_DIR/.git" ]; then
        log "Updating existing repository checkout at $REPO_DIR"
        cd "$REPO_DIR"
        git fetch --prune origin
        git pull --ff-only origin main
    else
        log "Cloning repository to $REPO_DIR"
        git clone "$REPO_URL" "$REPO_DIR"
        cd "$REPO_DIR"
    fi
}

build_artifacts() {
    log "Building Rust backend"
    cd "$REPO_DIR"
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    fi
    cargo build --release -p mortimeriot

    log "Building Svelte frontend"
    cd "$REPO_DIR/frontend"
    npm ci
    npm run build
}

deploy_artifacts() {
    log "Creating deployment directories"
    mkdir -p "$DEPLOY_ROOT" "$FRONTEND_DIR" "$BIN_DIR" "$RUNTIME_DIR" "$TMP_DIR"

    log "Deploying frontend assets"
    rsync -a --delete "$REPO_DIR/frontend/dist/" "$FRONTEND_DIR/"

    log "Deploying backend binary"
    local tmp_bin="$TMP_DIR/mortimeriot.$$.new"
    install -m 0755 "$REPO_DIR/target/release/mortimeriot" "$tmp_bin"
    mv -f "$tmp_bin" "$BIN_DIR/mortimeriot"
}

configure_secrets() {
    log "Configuring runtime environment"
    mkdir -p "$ENV_DIR"
    chmod 750 "$ENV_DIR"

    local api_key=""
    if [ -f "$ENV_FILE" ] && grep -q '^MORTIMERIOT_INGEST_API_KEY=' "$ENV_FILE"; then
        log "Existing API key found in $ENV_FILE; keeping current value"
        return
    fi

    while [ -z "$api_key" ]; do
        read -r -s -p "Enter MORTIMERIOT_INGEST_API_KEY: " api_key
        printf '\n'
        if [ -z "$api_key" ]; then
            log "API key cannot be empty"
        fi
    done

    umask 027
    cat > "$ENV_FILE" <<EOF
MORTIMERIOT_INGEST_API_KEY=$api_key
EOF
    chmod 640 "$ENV_FILE"
    chown root:www-data "$ENV_FILE"
}

initialize_runtime() {
    local config_file="$RUNTIME_DIR/mortimeriot.toml"
    local sqlite_file="$RUNTIME_DIR/mortimeriot.db"

    log "Initializing runtime config and sqlite database"
    "$BIN_DIR/mortimeriot" init \
        --config "$config_file" \
        --listener-ip "$DEFAULT_LISTENER_IP" \
        --port "$DEFAULT_PORT" \
        --sqlite-db-path "$sqlite_file"
}

install_service_and_apache_files() {
    log "Installing systemd unit"
    install -m 0644 "$REPO_DIR/crates/mortimeriot/systemd/mortimeriot.service" "$SERVICE_FILE"

    log "Installing Apache virtual host"
    install -m 0644 "$REPO_DIR/crates/mortimeriot/apache2/pap.mortimer.nl.conf" "$APACHE_SITE_FILE"

    log "Installing hourly update script"
    install -m 0755 "$REPO_DIR/scripts/update_pap.sh" "$UPDATE_SCRIPT_PATH"
}

configure_cron() {
    log "Configuring cron job for hourly updates"

    local tmp_cron
    tmp_cron="$(mktemp)"

    if crontab -l >/dev/null 2>&1; then
        crontab -l | grep -Fv "$CRON_ENTRY" > "$tmp_cron"
    fi

    printf '%s\n' "$CRON_ENTRY" >> "$tmp_cron"
    crontab "$tmp_cron"
    rm -f "$tmp_cron"
}

enable_services() {
    log "Enabling required Apache modules"
    a2enmod rewrite ssl proxy proxy_http headers

    log "Enabling Apache site"
    a2ensite pap.mortimer.nl.conf

    if [ -f "/etc/apache2/sites-enabled/000-default.conf" ]; then
        a2dissite 000-default.conf || true
    fi

    if [ ! -f "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" ]; then
        log "ERROR: SSL certificate for $DOMAIN not found at /etc/letsencrypt/live/$DOMAIN"
        log "Install/renew certificates before enabling this site, then rerun this script"
        exit 1
    fi

    log "Validating Apache config"
    apache2ctl configtest

    log "Reloading systemd and starting service"
    systemctl daemon-reload
    systemctl enable --now "$SERVICE_NAME"

    log "Reloading Apache"
    systemctl reload apache2
}

set_permissions() {
    log "Applying ownership for deployment directories"
    chown -R www-data:www-data "$DEPLOY_ROOT"
}

run_smoke_checks() {
    log "Running smoke checks"
    curl --fail --silent --show-error "http://127.0.0.1:2111/api/v1/health" >/dev/null
    systemctl --no-pager --full status "$SERVICE_NAME" | head -n 10
}

main() {
    require_root
    require_ubuntu
    install_packages
    install_rust
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    fi

    ensure_command cargo
    ensure_command npm
    ensure_command rsync
    ensure_command apache2ctl
    ensure_command systemctl
    ensure_command curl

    setup_repo
    build_artifacts
    deploy_artifacts
    configure_secrets
    initialize_runtime
    install_service_and_apache_files
    configure_cron
    set_permissions
    enable_services
    run_smoke_checks

    log "Bootstrap complete"
}

main "$@"
