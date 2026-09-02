use serde::{Deserialize, Serialize};

pub const MIN_TEMPERATURE_C: f64 = -40.0;
pub const MAX_TEMPERATURE_C: f64 = 80.0;
pub const MIN_HUMIDITY_PERCENT: f64 = 0.0;
pub const MAX_HUMIDITY_PERCENT: f64 = 100.0;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeatherMeasurement {
    pub temperature: f64,
    pub humidity: f64,
    pub wind_speed: f64,
}

impl WeatherMeasurement {
    pub fn validate(&self) -> std::result::Result<(), &'static str> {
        if self.temperature < MIN_TEMPERATURE_C || self.temperature > MAX_TEMPERATURE_C {
            return Err("temperature is out of supported sensor range");
        }
        if self.humidity < MIN_HUMIDITY_PERCENT || self.humidity > MAX_HUMIDITY_PERCENT {
            return Err("humidity is out of supported sensor range");
        }
        if self.wind_speed < 0.0 {
            return Err("wind speed cannot be negative");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeatherReading {
    pub id: i64,
    pub temperature: f64,
    pub humidity: f64,
    pub wind_speed: f64,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeatherHistoryQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeatherHistoryResponse {
    pub items: Vec<WeatherReading>,
    pub next_cursor: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatestWeatherResponse {
    pub item: Option<WeatherReading>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_sensor_bounds() {
        let reading = WeatherMeasurement {
            temperature: MIN_TEMPERATURE_C,
            humidity: MAX_HUMIDITY_PERCENT,
            wind_speed: 0.0,
        };

        assert!(reading.validate().is_ok());
    }

    #[test]
    fn rejects_out_of_range_temperature() {
        let reading = WeatherMeasurement {
            temperature: MAX_TEMPERATURE_C + 0.1,
            humidity: 50.0,
            wind_speed: 0.0,
        };

        assert!(reading.validate().is_err());
    }

    #[test]
    fn rejects_out_of_range_humidity() {
        let reading = WeatherMeasurement {
            temperature: 20.0,
            humidity: MIN_HUMIDITY_PERCENT - 1.0,
            wind_speed: 0.0,
        };

        assert!(reading.validate().is_err());
    }

    #[test]
    fn deserializes_arduino_payload_shape() {
        let input = r#"{"temperature":21.25,"humidity":48.5,"wind_speed":5.0}"#;
        let measurement: WeatherMeasurement = serde_json::from_str(input).expect("valid payload");

        assert_eq!(measurement.temperature, 21.25);
        assert_eq!(measurement.humidity, 48.5);
        assert_eq!(measurement.wind_speed, 5.0);
    }

    #[test]
    fn serializes_history_with_cursor() {
        let response = WeatherHistoryResponse {
            items: vec![WeatherReading {
                id: 10,
                temperature: 20.1,
                humidity: 45.4,
                wind_speed: 12.3,
                recorded_at: "2026-07-21T12:00:00Z".to_owned(),
            }],
            next_cursor: Some(9),
        };

        let json = serde_json::to_value(response).expect("serializable response");
        assert_eq!(json["next_cursor"], 9);
        assert_eq!(json["items"][0]["id"], 10);
    }
}
