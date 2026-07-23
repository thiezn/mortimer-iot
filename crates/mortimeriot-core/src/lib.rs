pub mod protocol;

pub const INGEST_API_KEY_HEADER: &str = "X-API-Key";

pub use protocol::{
    system::{ApiErrorResponse, HealthcheckResponse, VersionResponse},
    weather::{
        LatestWeatherResponse, WeatherHistoryQuery, WeatherHistoryResponse, WeatherMeasurement,
        WeatherReading,
    },
};
