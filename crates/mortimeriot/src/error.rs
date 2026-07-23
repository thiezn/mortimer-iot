use thiserror::Error as ThisError;

/// Result alias for
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Errors produced by this crate
#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Axum(#[from] axum::Error),

    /// Error occured while parsing or serializing JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    MigrateFrom(#[from] sqlx::migrate::MigrateError),

    #[error("Missing required config file at {0}")]
    MissingConfig(String),

    #[error("Missing required sqlite database file at {0}")]
    MissingDatabase(String),

    #[error("Missing required environment variable MORTIMERIOT_INGEST_API_KEY")]
    MissingIngestApiKey,

    #[error("Invalid UNIX timestamp")]
    InvalidTimestamp,
}
