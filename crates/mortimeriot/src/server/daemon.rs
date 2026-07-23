use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Query, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use mortimeriot_core::{
    ApiErrorResponse, HealthcheckResponse, INGEST_API_KEY_HEADER, LatestWeatherResponse,
    VersionResponse, WeatherHistoryQuery, WeatherHistoryResponse, WeatherMeasurement,
};
use subtle::ConstantTimeEq;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::signal;
use tracing::{debug, info, warn};

use crate::{Result, db::DbClient};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_HISTORY_LIMIT: u32 = 1000;
const MAX_HISTORY_LIMIT: u32 = 10_000;

/// Shared axum application state.
#[derive(Clone)]
pub struct AppState {
    pub db: DbClient,
    pub ingest_api_key: String,
}

/// Starts the HTTP daemon.
///
/// Arguments:
/// - `db`: Database client used by all handlers.
/// - `listener_ip`: IP address to bind to.
/// - `port`: TCP port to bind to.
/// - `ingest_api_key`: Shared key used to accept ingestion writes.
pub async fn run(db: DbClient, listener_ip: String, port: u16, ingest_api_key: String) -> Result {
    info!(listener_ip, port, "starting HTTP daemon");
    let state = AppState { db, ingest_api_key };

    let app = Router::new()
        .route("/", get(root))
        .route("/api/v1/version", get(version))
        .route("/api/v1/health", get(healthcheck))
        .route("/healthcheck", get(healthcheck))
        .route("/api/v1/weather", get(list_weather_data))
        .route("/api/v1/weather/latest", get(latest_weather_data))
        .route("/iot/weather", post(ingest_weather_data))
        .fallback(non_existing_route_handler)
        .layer(DefaultBodyLimit::max(1024))
        .layer(middleware::from_fn(log_incoming_request))
        .with_state(state);

    let bind_with = format!("{listener_ip}:{port}");
    info!(address = %bind_with, "binding TCP listener");
    let listener = tokio::net::TcpListener::bind(bind_with).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("HTTP daemon stopped");
    Ok(())
}

/// Returns root endpoint text.
async fn root() -> &'static str {
    "Mortimer IoT server"
}

/// Returns server version.
async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: VERSION.to_owned(),
    })
}

/// Returns service health state.
async fn healthcheck(
    State(state): State<AppState>,
) -> std::result::Result<Json<HealthcheckResponse>, ApiError> {
    state
        .db
        .health_ping()
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    Ok(Json(HealthcheckResponse {
        state: "OK".to_owned(),
    }))
}

async fn ingest_weather_data(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WeatherMeasurement>,
) -> std::result::Result<(StatusCode, Json<mortimeriot_core::WeatherReading>), ApiError> {
    let Some(api_key_header) = headers.get(INGEST_API_KEY_HEADER) else {
        return Err(ApiError::unauthorized("missing API key header"));
    };
    let api_key = api_key_header
        .to_str()
        .map_err(|_| ApiError::unauthorized("invalid API key header encoding"))?;

    if !valid_api_key(&state.ingest_api_key, api_key) {
        return Err(ApiError::unauthorized("invalid API key"));
    }

    payload.validate().map_err(ApiError::bad_request)?;

    let reading = state
        .db
        .store_weather_data(&payload)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok((StatusCode::CREATED, Json(reading)))
}

async fn list_weather_data(
    State(state): State<AppState>,
    Query(query): Query<WeatherHistoryQuery>,
) -> std::result::Result<Json<WeatherHistoryResponse>, ApiError> {
    let from_ms = parse_rfc3339_to_ms(query.from.as_deref())?;
    let to_ms = parse_rfc3339_to_ms(query.to.as_deref())?;
    let limit = query
        .limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .min(MAX_HISTORY_LIMIT);

    let (items, next_cursor) = state
        .db
        .list_weather_data(from_ms, to_ms, query.cursor, limit)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(WeatherHistoryResponse { items, next_cursor }))
}

async fn latest_weather_data(
    State(state): State<AppState>,
) -> std::result::Result<Json<LatestWeatherResponse>, ApiError> {
    let item = state
        .db
        .latest_weather_data()
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(LatestWeatherResponse { item }))
}

/// Logs every incoming HTTP request.
async fn log_incoming_request(req: Request, next: Next) -> impl IntoResponse {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let started = std::time::Instant::now();

    debug!(method = %method, path = %path, "incoming HTTP request");

    let response = next.run(req).await;
    debug!(
        method = %method,
        path = %path,
        status = %response.status(),
        elapsed_ms = started.elapsed().as_millis(),
        "completed HTTP request"
    );

    response
}

/// Handles requests that do not match any route.
async fn non_existing_route_handler(req: Request) -> ApiError {
    warn!(method = %req.method(), path = %req.uri().path(), "request hit non-existing route");
    ApiError::not_found()
}

/// Waits for process shutdown signals.
async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal_stream) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            signal_stream.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn valid_api_key(expected: &str, provided: &str) -> bool {
    expected.as_bytes().ct_eq(provided.as_bytes()).into()
}

fn parse_rfc3339_to_ms(value: Option<&str>) -> std::result::Result<Option<i64>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let datetime = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| ApiError::bad_request("invalid RFC3339 timestamp"))?;
    let millis = i64::try_from(datetime.unix_timestamp_nanos() / 1_000_000)
        .map_err(|_| ApiError::bad_request("timestamp is out of range"))?;
    Ok(Some(millis))
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound,
    Internal,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    fn not_found() -> Self {
        Self::NotFound
    }

    fn internal(message: String) -> Self {
        warn!(error = %message, "internal server error");
        Self::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "route does not exist".to_owned(),
            ),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "unexpected server error".to_owned(),
            ),
        };

        (
            status,
            Json(ApiErrorResponse {
                code: code.to_owned(),
                message,
            }),
        )
            .into_response()
    }
}
