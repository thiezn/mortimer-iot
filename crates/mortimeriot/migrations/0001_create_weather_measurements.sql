CREATE TABLE IF NOT EXISTS weather_measurements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    temperature_c REAL NOT NULL CHECK (temperature_c >= -40.0 AND temperature_c <= 80.0),
    humidity_percent REAL NOT NULL CHECK (humidity_percent >= 0.0 AND humidity_percent <= 100.0),
    recorded_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_weather_measurements_recorded_at
    ON weather_measurements (recorded_at_ms, id);
