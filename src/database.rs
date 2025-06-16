// MIT License
// Copyright (c) 2025 Yukke.org
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use chrono::{DateTime, Local};
use peripheral::bme280::Measurement;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use tokio::sync::mpsc;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct SensorData {
    pub timestamp: DateTime<Local>,
    pub temperature_c: f64,
    pub humidity_relative: f64,
    pub pressure_pa: f64,
    pub thi: f64,
}

impl SensorData {
    pub fn from_measurement(measurement: Measurement, thi: f64) -> Self {
        Self {
            timestamp: Local::now(),
            temperature_c: measurement.temperature_c,
            humidity_relative: measurement.humidity_relative,
            pressure_pa: measurement.pressure_pa,
            thi,
        }
    }
}

pub struct Database {
    sender: mpsc::UnboundedSender<SensorData>,
}

impl Database {
    pub async fn new(connection_string: &str) -> Result<Self, BoxError> {
        let options = SqliteConnectOptions::from_str(connection_string)?
            .create_if_missing(true)
            .disable_statement_logging();

        let pool = SqlitePool::connect_with(options).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sensor_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                temperature_c REAL NOT NULL,
                humidity_relative REAL NOT NULL,
                pressure_pa REAL NOT NULL,
                thi REAL NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let (sender, mut receiver) = mpsc::unbounded_channel::<SensorData>();
        let pool_clone = pool.clone();

        tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                if let Err(e) = insert_sensor_data(&pool_clone, &data).await {
                    eprintln!("Failed to save sensor data: {}", e);
                }
            }
        });

        Ok(Database { sender })
    }

    pub fn save_async(&self, data: SensorData) -> Result<(), BoxError> {
        self.sender.send(data)?;
        Ok(())
    }
}

async fn insert_sensor_data(pool: &SqlitePool, data: &SensorData) -> Result<(), BoxError> {
    sqlx::query(
        "INSERT INTO sensor_data (timestamp, temperature_c, humidity_relative, pressure_pa, thi) 
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(data.timestamp.to_rfc3339())
    .bind(data.temperature_c)
    .bind(data.humidity_relative)
    .bind(data.pressure_pa)
    .bind(data.thi)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};
    use peripheral::bme280::Measurement;

    #[test]
    fn test_sensor_data_creation() {
        let measurement = Measurement {
            temperature_c: 25.0,
            pressure_pa: 101325.0,
            humidity_relative: 50.0,
        };
        let thi = 72.5;

        let sensor_data = SensorData::from_measurement(measurement, thi);

        assert_eq!(sensor_data.temperature_c, 25.0);
        assert_eq!(sensor_data.pressure_pa, 101325.0);
        assert_eq!(sensor_data.humidity_relative, 50.0);
        assert_eq!(sensor_data.thi, 72.5);
        assert!(sensor_data.timestamp <= Local::now());
    }

    #[test]
    fn test_sensor_data_debug_format() {
        let sensor_data = SensorData {
            timestamp: Local.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap(),
            temperature_c: 23.5,
            humidity_relative: 60.2,
            pressure_pa: 100500.0,
            thi: 75.8,
        };

        let debug_string = format!("{:?}", sensor_data);
        assert!(debug_string.contains("SensorData"));
        assert!(debug_string.contains("23.5"));
        assert!(debug_string.contains("60.2"));
        assert!(debug_string.contains("100500"));
        assert!(debug_string.contains("75.8"));
    }

    #[test]
    fn test_sensor_data_from_measurement_timestamp() {
        let measurement = Measurement {
            temperature_c: 20.0,
            pressure_pa: 100000.0,
            humidity_relative: 40.0,
        };

        let before = Local::now();
        let sensor_data = SensorData::from_measurement(measurement, 65.0);
        let after = Local::now();

        assert!(sensor_data.timestamp >= before);
        assert!(sensor_data.timestamp <= after);
    }

    #[test]
    fn test_sensor_data_values_precision() {
        let measurement = Measurement {
            temperature_c: 25.123456789,
            pressure_pa: 101325.987654321,
            humidity_relative: 50.555555555,
        };
        let thi = 72.123456789;

        let sensor_data = SensorData::from_measurement(measurement, thi);

        assert_eq!(sensor_data.temperature_c, 25.123456789);
        assert_eq!(sensor_data.pressure_pa, 101325.987654321);
        assert_eq!(sensor_data.humidity_relative, 50.555555555);
        assert_eq!(sensor_data.thi, 72.123456789);
    }

    #[test]
    fn test_sensor_data_extreme_values() {
        let measurement = Measurement {
            temperature_c: -40.0,
            pressure_pa: 30000.0,
            humidity_relative: 0.0,
        };
        let thi = 0.0;

        let sensor_data = SensorData::from_measurement(measurement, thi);

        assert_eq!(sensor_data.temperature_c, -40.0);
        assert_eq!(sensor_data.pressure_pa, 30000.0);
        assert_eq!(sensor_data.humidity_relative, 0.0);
        assert_eq!(sensor_data.thi, 0.0);
    }

    #[test]
    fn test_sensor_data_high_values() {
        let measurement = Measurement {
            temperature_c: 85.0,
            pressure_pa: 110000.0,
            humidity_relative: 100.0,
        };
        let thi = 120.0;

        let sensor_data = SensorData::from_measurement(measurement, thi);

        assert_eq!(sensor_data.temperature_c, 85.0);
        assert_eq!(sensor_data.pressure_pa, 110000.0);
        assert_eq!(sensor_data.humidity_relative, 100.0);
        assert_eq!(sensor_data.thi, 120.0);
    }

    #[tokio::test]
    async fn test_database_new_creates_table() {
        let result = Database::new("sqlite::memory:").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_save_async() {
        let database = Database::new("sqlite::memory:").await.unwrap();

        let sensor_data = SensorData {
            timestamp: Local::now(),
            temperature_c: 25.0,
            humidity_relative: 50.0,
            pressure_pa: 101325.0,
            thi: 72.5,
        };

        let result = database.save_async(sensor_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_box_error_type_alias() {
        let _error: BoxError = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test"));
    }

    #[test]
    fn test_rfc3339_timestamp_format() {
        let timestamp = Local.with_ymd_and_hms(2025, 6, 16, 14, 30, 45).unwrap();
        let rfc3339_string = timestamp.to_rfc3339();

        assert!(rfc3339_string.contains("2025"));
        assert!(rfc3339_string.contains("06"));
        assert!(rfc3339_string.contains("16"));
        assert!(rfc3339_string.contains("14"));
        assert!(rfc3339_string.contains("30"));
        assert!(rfc3339_string.contains("45"));
    }
}
