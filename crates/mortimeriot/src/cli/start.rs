use std::path::PathBuf;

use clap::Args;
use tracing::info;

use crate::{Error, Result, db::DbClient, server};

use super::base::{Settings, config_path_or_default, read_settings};

/// Arguments for the `start` subcommand.
#[derive(Debug, Args)]
pub struct StartArgs {
    /// Path to the config file to read.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Listener IP override.
    #[arg(long)]
    pub listener_ip: Option<String>,

    /// Listener port override.
    #[arg(long)]
    pub port: Option<u16>,

    /// SQLite DB path override.
    #[arg(long)]
    pub sqlite_db_path: Option<PathBuf>,
}

/// Executes the `start` command.
///
/// Arguments:
/// - `args`: Parsed CLI arguments for startup.
pub async fn run(args: StartArgs) -> Result {
    let config_path = config_path_or_default(args.config.clone());
    info!(config = %config_path.display(), "loading configuration");
    let settings = read_settings(&config_path)?;
    let settings = apply_overrides(settings, args);

    let db_path = PathBuf::from(&settings.sqlite_db_path);
    info!(db_path = %db_path.display(), "validating sqlite database path");
    if !db_path.exists() {
        return Err(Error::MissingDatabase(db_path.display().to_string()));
    }

    let db = DbClient::connect_sqlite_file(&db_path).await?;
    db.run_migrations().await?;
    let ingest_api_key =
        std::env::var("MORTIMERIOT_INGEST_API_KEY").map_err(|_| Error::MissingIngestApiKey)?;

    info!(listener_ip = %settings.listener_ip, port = settings.port, "starting daemon");
    server::run(db, settings.listener_ip, settings.port, ingest_api_key).await
}

/// Applies command-line overrides on top of loaded settings.
///
/// Arguments:
/// - `settings`: Settings loaded from config.
/// - `args`: Parsed CLI override arguments.
fn apply_overrides(settings: Settings, args: StartArgs) -> Settings {
    Settings {
        listener_ip: args.listener_ip.unwrap_or(settings.listener_ip),
        port: args.port.unwrap_or(settings.port),
        sqlite_db_path: args
            .sqlite_db_path
            .map(|path| path.display().to_string())
            .unwrap_or(settings.sqlite_db_path),
    }
}
