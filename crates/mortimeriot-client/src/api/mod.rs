use reqwest::{Client, Method, RequestBuilder};

use mortimeriot_core::{
    HealthcheckResponse, INGEST_API_KEY_HEADER, LatestWeatherResponse, VersionResponse,
    WeatherHistoryQuery, WeatherHistoryResponse, WeatherMeasurement, WeatherReading,
};

use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: String,
    http: Client,
    auth_key: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: String, auth_key: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: Client::new(),
            auth_key,
        }
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self.http.request(method, url);

        if let Some(key) = &self.auth_key {
            builder = builder.header(INGEST_API_KEY_HEADER, key);
        }

        builder
    }

    pub async fn healthcheck(&self) -> Result<HealthcheckResponse> {
        self.get_json("/api/v1/health").await
    }

    pub async fn version(&self) -> Result<VersionResponse> {
        self.get_json("/api/v1/version").await
    }

    pub async fn post_weather(&self, payload: &WeatherMeasurement) -> Result<WeatherReading> {
        self.post_json("/api/v1/weather", payload).await
    }

    pub async fn list_weather_data(
        &self,
        query: &WeatherHistoryQuery,
    ) -> Result<WeatherHistoryResponse> {
        let response = self
            .request(Method::GET, "/api/v1/weather")
            .query(query)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(Error::InvalidInput(format!(
                "request failed with status {}",
                response.status()
            )));
        }
        Ok(response.json::<WeatherHistoryResponse>().await?)
    }

    pub async fn latest_weather_data(&self) -> Result<LatestWeatherResponse> {
        self.get_json("/api/v1/weather/latest").await
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self.request(Method::GET, path).send().await?;
        if !response.status().is_success() {
            return Err(Error::InvalidInput(format!(
                "request failed with status {}",
                response.status()
            )));
        }
        Ok(response.json::<T>().await?)
    }

    pub async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let response = self.request(Method::POST, path).json(body).send().await?;
        if !response.status().is_success() {
            return Err(Error::InvalidInput(format!(
                "request failed with status {}",
                response.status()
            )));
        }
        Ok(response.json::<T>().await?)
    }
}
