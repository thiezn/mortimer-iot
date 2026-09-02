use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use mortimeriot_core::{WeatherMeasurement, WeatherReading};
use sqlx::{
    Pool, QueryBuilder, Row, Sqlite,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::{debug, info};

use crate::{Error, Result};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Database client that encapsulates all SQLite interactions.
#[derive(Clone)]
pub struct DbClient {
    pool: Pool<Sqlite>,
}

impl DbClient {
    /// Connects to a SQLite database file and creates it when missing.
    ///
    /// Arguments:
    /// - `path`: Filesystem path to the SQLite file.
    pub async fn connect_sqlite_file(path: &Path) -> Result<Self> {
        info!(path = %path.display(), "connecting to sqlite database");
        let sqlite_url = format!("sqlite://{}", path.display());
        let options = SqliteConnectOptions::from_str(&sqlite_url)?
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result {
        info!("running database migrations");
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    /// Stores one weather measurement with a server-side timestamp.
    pub async fn store_weather_data(&self, data: &WeatherMeasurement) -> Result<WeatherReading> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::InvalidTimestamp)?;
        let recorded_at_ms = i64::try_from(now.as_millis()).map_err(|_| Error::InvalidTimestamp)?;

        debug!(
            query = "INSERT INTO weather_measurements (temperature_c, humidity_percent, wind_speed, recorded_at_ms) VALUES (?, ?, ?)",
            temperature = data.temperature,
            humidity = data.humidity,
            wind_speed = data.wind_speed,
            recorded_at_ms,
            "storing weather measurement"
        );
        let row = sqlx::query(
            "INSERT INTO weather_measurements (temperature_c, humidity_percent, wind_speed, recorded_at_ms)
             VALUES (?, ?, ?, ?)
             RETURNING id, temperature_c, humidity_percent, wind_speed, recorded_at_ms",
        )
        .bind(data.temperature)
        .bind(data.humidity)
        .bind(data.wind_speed)
        .bind(recorded_at_ms)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_reading(&row)
    }

    pub async fn list_weather_data(
        &self,
        from_ms: Option<i64>,
        to_ms: Option<i64>,
        cursor: Option<i64>,
        limit: u32,
    ) -> Result<(Vec<WeatherReading>, Option<i64>)> {
        let fetch_limit = limit.saturating_add(1);
        let mut qb = QueryBuilder::<Sqlite>::new(
            "SELECT id, temperature_c, humidity_percent, wind_speed, recorded_at_ms FROM weather_measurements WHERE 1=1",
        );

        if let Some(from_ms) = from_ms {
            qb.push(" AND recorded_at_ms >= ").push_bind(from_ms);
        }
        if let Some(to_ms) = to_ms {
            qb.push(" AND recorded_at_ms < ").push_bind(to_ms);
        }
        if let Some(cursor) = cursor {
            qb.push(" AND id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY id DESC LIMIT ")
            .push_bind(i64::from(fetch_limit));

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut items = Vec::with_capacity(rows.len().min(limit as usize));
        for row in rows {
            items.push(self.row_to_reading(&row)?);
        }

        let next_cursor = if items.len() > limit as usize {
            let _ = items.pop();
            items.last().map(|item| item.id)
        } else {
            None
        };

        Ok((items, next_cursor))
    }

    pub async fn latest_weather_data(&self) -> Result<Option<WeatherReading>> {
        let row = sqlx::query(
            "SELECT id, temperature_c, humidity_percent, wind_speed, recorded_at_ms
             FROM weather_measurements
             ORDER BY id DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| self.row_to_reading(&row)).transpose()
    }

    pub async fn health_ping(&self) -> Result {
        let _: (i64,) = sqlx::query_as("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }

    fn row_to_reading(&self, row: &sqlx::sqlite::SqliteRow) -> Result<WeatherReading> {
        let recorded_at_ms: i64 = row.try_get("recorded_at_ms")?;
        let datetime =
            OffsetDateTime::from_unix_timestamp_nanos((recorded_at_ms as i128) * 1_000_000)
                .map_err(|_| Error::InvalidTimestamp)?;
        let recorded_at = datetime
            .format(&Rfc3339)
            .map_err(|_| Error::InvalidTimestamp)?;

        Ok(WeatherReading {
            id: row.try_get("id")?,
            temperature: row.try_get("temperature_c")?,
            humidity: row.try_get("humidity_percent")?,
            wind_speed: row.try_get("wind_speed")?,
            recorded_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use mortimeriot_core::WeatherMeasurement;

    use super::DbClient;

    fn temp_db_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("mortimeriot-{name}-{unique}.db"))
    }

    #[tokio::test]
    async fn migration_and_insert_round_trip() {
        let db_path = temp_db_path("insert-roundtrip");
        let db = DbClient::connect_sqlite_file(&db_path)
            .await
            .expect("database created");
        db.run_migrations().await.expect("migrations applied");

        let stored = db
            .store_weather_data(&WeatherMeasurement {
                temperature: 21.5,
                humidity: 47.25,
                wind_speed: 0.0,
            })
            .await
            .expect("measurement stored");

        assert!(stored.id > 0);
        assert_eq!(stored.temperature, 21.5);
        assert_eq!(stored.humidity, 47.25);

        let latest = db
            .latest_weather_data()
            .await
            .expect("latest query succeeds")
            .expect("latest reading exists");
        assert_eq!(latest.id, stored.id);

        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn list_weather_data_returns_descending_results() {
        let db_path = temp_db_path("history-list");
        let db = DbClient::connect_sqlite_file(&db_path)
            .await
            .expect("database created");
        db.run_migrations().await.expect("migrations applied");

        let first = db
            .store_weather_data(&WeatherMeasurement {
                temperature: 18.0,
                humidity: 40.0,
                wind_speed: 0.0,
            })
            .await
            .expect("first insert succeeds");
        let second = db
            .store_weather_data(&WeatherMeasurement {
                temperature: 19.0,
                humidity: 41.0,
                wind_speed: 0.0,
            })
            .await
            .expect("second insert succeeds");

        let (items, next_cursor) = db
            .list_weather_data(None, None, None, 10)
            .await
            .expect("history query succeeds");

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, second.id);
        assert_eq!(items[1].id, first.id);
        assert!(next_cursor.is_none());

        let _ = std::fs::remove_file(db_path);
    }
}
