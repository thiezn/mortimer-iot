CREATE TABLE IF NOT EXISTS weather_measurements (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	temperature_c REAL NOT NULL,
	humidity_percent REAL NOT NULL,
	recorded_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_weather_measurements_recorded_at
	ON weather_measurements (recorded_at_ms, id);
