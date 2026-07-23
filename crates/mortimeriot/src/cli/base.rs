use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use tracing::{Level, info};

use crate::{Error, Result};

use super::{init, start};

/// Command-line interface for the service.
#[derive(Debug, Parser)]
#[command(name = "mortimeriot")]
#[command(about = "Mortimer IoT API service")]
pub struct Cli {
    /// Global log level for the process.
    #[arg(long, value_enum, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    #[command(subcommand)]
    pub command: Command,
}

/// Supported CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initializes database and config files.
    Init(init::InitArgs),
    /// Starts the HTTP server.
    Start(start::StartArgs),
}

/// Supported tracing log levels.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogLevel {
    /// Error only.
    Error,
    /// Warnings and errors.
    Warn,
    /// Informational logs, warnings, and errors.
    Info,
    /// Debug and higher.
    Debug,
    /// Trace and higher.
    Trace,
}

impl LogLevel {
    /// Converts CLI log level to tracing level.
    pub fn to_tracing_level(self) -> Level {
        match self {
            Self::Error => Level::ERROR,
            Self::Warn => Level::WARN,
            Self::Info => Level::INFO,
            Self::Debug => Level::DEBUG,
            Self::Trace => Level::TRACE,
        }
    }
}

/// Runtime settings loaded from `mortimeriot.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Listener IP address.
    pub listener_ip: String,
    /// Listener TCP port.
    pub port: u16,
    /// SQLite file path.
    pub sqlite_db_path: String,
}

impl Default for Settings {
    /// Returns default runtime settings.
    fn default() -> Self {
        Self {
            listener_ip: "0.0.0.0".to_owned(),
            port: 2111,
            sqlite_db_path: "./mortimeriot.db".to_owned(),
        }
    }
}

/// Parses CLI arguments, initializes tracing, and executes a subcommand.
pub async fn run() -> Result {
    let cli = Cli::parse();
    init_tracing(cli.log_level);
    info!(level = ?cli.log_level, "starting CLI command execution");

    match cli.command {
        Command::Init(args) => init::run(args).await,
        Command::Start(args) => start::run(args).await,
    }
}

/// Loads settings from a TOML file.
///
/// Arguments:
/// - `path`: Path to the config file.
pub fn read_settings(path: &Path) -> Result<Settings> {
    if !path.exists() {
        return Err(Error::MissingConfig(path.display().to_string()));
    }

    let content = fs::read_to_string(path)?;
    let settings = toml::from_str(&content)?;
    Ok(settings)
}

/// Writes settings to a TOML file.
///
/// Arguments:
/// - `path`: Path to write the config file.
/// - `settings`: Runtime settings to serialize.
pub fn write_settings(path: &Path, settings: &Settings) -> Result {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let toml = toml::to_string_pretty(settings)?;
    fs::write(path, toml)?;
    Ok(())
}

/// Resolves the config path, defaulting to `mortimeriot.toml`.
///
/// Arguments:
/// - `path`: Optional config path from CLI.
pub fn config_path_or_default(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| PathBuf::from("mortimeriot.toml"))
}

/// Initializes tracing with a max-level filter.
///
/// Arguments:
/// - `level`: Maximum tracing level to emit.
fn init_tracing(level: LogLevel) {
    let _ = tracing_subscriber::fmt()
        .with_max_level(level.to_tracing_level())
        .with_target(false)
        .compact()
        .try_init();
}
