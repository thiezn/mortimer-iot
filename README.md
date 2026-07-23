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
