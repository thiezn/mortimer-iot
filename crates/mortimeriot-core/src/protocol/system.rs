use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthcheckResponse {
    pub state: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionResponse {
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
}
