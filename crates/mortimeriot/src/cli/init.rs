use std::{fs, path::PathBuf};

use clap::Args;
use tracing::info;

use crate::{Result, db::DbClient};

use super::base::{Settings, config_path_or_default, write_settings};

/// Arguments for the `init` subcommand.
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Path to the config file to generate.
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

/// Executes the `init` command.
///
/// Arguments:
/// - `args`: Parsed CLI arguments for initialization.
pub async fn run(args: InitArgs) -> Result {
    let config_path = config_path_or_default(args.config);
    info!(config = %config_path.display(), "initializing service files");

    let mut settings = Settings::default();
    if let Some(listener_ip) = args.listener_ip {
        settings.listener_ip = listener_ip;
    }
    if let Some(port) = args.port {
        settings.port = port;
    }
    if let Some(sqlite_db_path) = args.sqlite_db_path {
        settings.sqlite_db_path = sqlite_db_path.display().to_string();
    }

    let db_path = PathBuf::from(&settings.sqlite_db_path);
    info!(db_path = %db_path.display(), "preparing sqlite database");
    if let Some(parent) = db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let db = DbClient::connect_sqlite_file(&db_path).await?;
    db.run_migrations().await?;

    info!("writing config file");
    write_settings(&config_path, &settings)
}
