# Simple API and dashboard for hobby IoT project.

A Rust workspace for KOReader sync, composed of:

- `mortimeriot` (server): Axum API + WebDAV daemon with SQLite storage.
- `mortimeriot-client` (client): CLI client for interacting with the mortimeriot server.
- `mortimeriot-core` (core): Shared protocol models and constants used by both crates.

## Workspace layout

- `crates/mortimeriot`: server crate (library-first, thin binary wrapper).
- `crates/mortimeriot-client`: client crate (CLI for interacting with the mortimeriot server).
- `crates/mortimeriot-core`: core crate (shared protocol models and constants used by both crates).
- `frontend`: web dashboard (static files served by the server using svelte and shadcn).
- `hardware`: Arduino sketches for the IoT modules. Currently only a weather station is implemented.
- `mortimeriot.toml`: server runtime config.
- `mortimeriot.db`: SQLite database.

## Build

Build all crates:

```bash
cargo build --workspace
```

Build a specific crate:

```bash
cargo build -p mortimeriot
```

## Server usage

Initialize local files (DB, schema, config, books folder):

```bash
cargo run -p mortimeriot -- init
```

Start the server:

```bash
cargo run -p mortimeriot -- start
```

Override runtime values:

```bash
cargo run -p mortimeriot -- start \
  --config ./mortimeriot.toml \
  --listener-ip 0.0.0.0 \
  --port 2111 \
  --sqlite-db-path ./mortimeriot.db
```

Set log level:

```bash
cargo run -p mortimeriot -- --log-level debug start
```

## Client usage

The client defaults to `--base-url http://0.0.0.0:2111` and outputs JSON by default.

Global options:

- `--base-url <URL>`
- `--auth-user <USER>`
- `--auth-key <KEY>`
- `--log-level <error|warn|info|debug|trace>`

### Health and version

```bash
cargo run -p mortimeriot-client -- healthcheck
cargo run -p mortimeriot-client -- version
```

## Ubuntu VPS deployment (Apache + systemd)

This repository includes scripts and config files for running:

- Rust backend (`mortimeriot`) as a `systemd` service.
- Svelte frontend as static files served by Apache.
- Apache reverse proxy for `/api` and `/iot` to `127.0.0.1:2111`.
- Hourly pull/build/deploy through cron.

### Runtime layout on VPS

The deployment uses the following paths:

- Build checkout: `/root/pap.mortimer.nl`
- Deploy root: `/var/www/pap.mortimer.nl`
- Frontend static files: `/var/www/pap.mortimer.nl/current`
- Rust binary: `/var/www/pap.mortimer.nl/bin/mortimeriot`
- Runtime data (config + sqlite): `/var/www/pap.mortimer.nl/runtime`
- Environment secrets: `/etc/mortimeriot/mortimeriot.env`

### First-time setup

Run the setup script on the Ubuntu VPS itself, not on a local development machine.

1. Copy this repo to the server, or clone it.
2. Run the setup script as root:

```bash
chmod +x ./scripts/setup_pap_vps.sh ./scripts/update_pap.sh
sudo ./scripts/setup_pap_vps.sh
```

The setup script will:

- Install required Ubuntu packages (`apache2`, `git`, `cargo` prerequisites, `nodejs`, `npm`, etc.).
- Install Rust toolchain if missing.
- Pull/build backend and frontend.
- Deploy build artifacts into `/var/www/pap.mortimer.nl`.
- Prompt for `MORTIMERIOT_INGEST_API_KEY` and write it to `/etc/mortimeriot/mortimeriot.env`.
- Initialize config and sqlite database in `/var/www/pap.mortimer.nl/runtime`.
- Install and enable:
  - `crates/mortimeriot/systemd/mortimeriot.service`
  - `crates/mortimeriot/apache2/pap.mortimer.nl.conf`
- Enable Apache modules for proxying and TLS.
- Start/restart Apache and `mortimeriot`.

### Hourly updates with cron

The setup script installs `/root/update_pap.sh` and configures this root cron entry automatically:

```cron
20 * * * * /root/update_pap.sh > /var/log/update_pap.log 2>&1
```

`update_pap.sh` performs:

- `git pull --ff-only` from `main` in `/root/pap.mortimer.nl`
- `cargo build --release -p mortimeriot`
- `npm ci && npm run build` in `frontend`
- Deploy frontend `dist` into `/var/www/pap.mortimer.nl/current`
- Atomically replace backend binary in `/var/www/pap.mortimer.nl/bin/mortimeriot`
- Preserve runtime state in `/var/www/pap.mortimer.nl/runtime`
- Enforce `www-data:www-data` ownership
- Restart `mortimeriot` and reload Apache

### Service management

```bash
sudo systemctl status mortimeriot
sudo journalctl -u mortimeriot -n 200 --no-pager
sudo systemctl restart mortimeriot
```

### Apache management

```bash
sudo apache2ctl configtest
sudo systemctl reload apache2
sudo systemctl status apache2
```

### Smoke checks

Backend direct check:

```bash
curl -fsS http://127.0.0.1:2111/api/v1/health
```

Public checks:

```bash
curl -I https://pap.mortimer.nl/
curl -fsS https://pap.mortimer.nl/api/v1/health
```

### Notes

- The `systemd` service runs as `www-data:www-data`.
- The backend binds to `127.0.0.1:2111` by default in deployed config.
- SSL certificate paths in Apache config expect Let's Encrypt files at:
  - `/etc/letsencrypt/live/mortimer.nl/fullchain.pem`
  - `/etc/letsencrypt/live/mortimer.nl/privkey.pem`
- If `git pull --ff-only` fails in `/root/pap.mortimer.nl`, resolve repository state and rerun the update script.
