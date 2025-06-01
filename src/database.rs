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
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::{ConnectOptions, Row};
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
    pool: SqlitePool,
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

        Ok(Database { pool, sender })
    }

    pub fn save_async(&self, data: SensorData) -> Result<(), BoxError> {
        self.sender.send(data)?;
        Ok(())
    }

    pub async fn get_recent_data(&self, limit: i64) -> Result<Vec<SensorData>, BoxError> {
        let rows = sqlx::query(
            "SELECT timestamp, temperature_c, humidity_relative, pressure_pa, thi 
             FROM sensor_data 
             ORDER BY timestamp DESC 
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Local);

            results.push(SensorData {
                timestamp,
                temperature_c: row.get("temperature_c"),
                humidity_relative: row.get("humidity_relative"),
                pressure_pa: row.get("pressure_pa"),
                thi: row.get("thi"),
            });
        }

        Ok(results)
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
