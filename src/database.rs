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

use crate::types::BoxError;
use chrono::{DateTime, Local};
use peripheral::bme280::Measurement;
use sqlx::{AnyPool, any::AnyPoolOptions};
use std::sync::Once;
use tokio::sync::{mpsc, oneshot};

static DRIVER_INIT: Once = Once::new();

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
}

fn database_type_from_url(connection_string: &str) -> Result<DatabaseType, BoxError> {
    if connection_string.starts_with("postgresql://") {
        Ok(DatabaseType::PostgreSQL)
    } else if connection_string.starts_with("mysql://") {
        Ok(DatabaseType::MySQL)
    } else if connection_string.starts_with("sqlite:") {
        Ok(DatabaseType::SQLite)
    } else {
        Err("Unsupported database URL scheme".into())
    }
}

fn install_default_drivers() {
    // SQLx drivers only need to be installed once per process.
    DRIVER_INIT.call_once(sqlx::any::install_default_drivers);
}

#[derive(Debug)]
pub struct SensorData {
    pub timestamp: DateTime<Local>,
    pub temperature_c: f64,
    pub humidity_relative: f64,
    pub pressure_pa: f64,
    pub thi: f64,
}

impl SensorData {
    pub fn from_measurement(measurement: &Measurement, thi: f64) -> Self {
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
    sender: mpsc::UnboundedSender<SaveRequest>,
    #[cfg(test)]
    pool: AnyPool,
}

#[derive(Debug)]
struct SaveRequest {
    data: SensorData,
    completion: Option<oneshot::Sender<Result<(), String>>>,
}

impl Database {
    pub async fn new(connection_string: &str) -> Result<Self, BoxError> {
        let db_type = database_type_from_url(connection_string)?;
        install_default_drivers();

        let pool = if db_type == DatabaseType::SQLite && connection_string == "sqlite::memory:" {
            AnyPoolOptions::new()
                .max_connections(1)
                .connect(connection_string)
                .await?
        } else {
            AnyPool::connect(connection_string).await?
        };

        sqlx::query(create_table_sql(db_type))
            .execute(&pool)
            .await?;

        let (sender, mut receiver) = mpsc::unbounded_channel::<SaveRequest>();
        let pool_clone = pool.clone();

        tokio::spawn(async move {
            while let Some(request) = receiver.recv().await {
                let result = insert_sensor_data(&pool_clone, &request.data, db_type)
                    .await
                    .map_err(|error| error.to_string());

                if let Some(completion) = request.completion {
                    let _ = completion.send(result);
                } else if let Err(error) = result {
                    eprintln!("Failed to save sensor data: {error}");
                }
            }
        });

        Ok(Database {
            sender,
            #[cfg(test)]
            pool,
        })
    }

    pub fn save_async(&self, data: SensorData) -> Result<(), BoxError> {
        self.sender.send(SaveRequest {
            data,
            completion: None,
        })?;
        Ok(())
    }

    #[cfg(test)]
    async fn save(&self, data: SensorData) -> Result<(), BoxError> {
        let (completion, receiver) = oneshot::channel();
        self.sender.send(SaveRequest {
            data,
            completion: Some(completion),
        })?;

        match receiver.await? {
            Ok(()) => Ok(()),
            Err(error) => Err(std::io::Error::other(error).into()),
        }
    }
}

fn create_table_sql(db_type: DatabaseType) -> &'static str {
    match db_type {
        DatabaseType::PostgreSQL => {
            r#"
            CREATE TABLE IF NOT EXISTS sensor_data (
                id SERIAL PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                temperature_c DOUBLE PRECISION NOT NULL,
                humidity_relative DOUBLE PRECISION NOT NULL,
                pressure_pa DOUBLE PRECISION NOT NULL,
                thi DOUBLE PRECISION NOT NULL
            )
            "#
        }
        DatabaseType::MySQL => {
            r#"
            CREATE TABLE IF NOT EXISTS sensor_data (
                id INT AUTO_INCREMENT PRIMARY KEY,
                timestamp DATETIME(6) NOT NULL,
                temperature_c DOUBLE NOT NULL,
                humidity_relative DOUBLE NOT NULL,
                pressure_pa DOUBLE NOT NULL,
                thi DOUBLE NOT NULL
            )
            "#
        }
        DatabaseType::SQLite => {
            r#"
            CREATE TABLE IF NOT EXISTS sensor_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                temperature_c REAL NOT NULL,
                humidity_relative REAL NOT NULL,
                pressure_pa REAL NOT NULL,
                thi REAL NOT NULL
            )
            "#
        }
    }
}

fn insert_sql(db_type: DatabaseType) -> &'static str {
    match db_type {
        DatabaseType::PostgreSQL => {
            r#"
            INSERT INTO sensor_data (
                timestamp,
                temperature_c,
                humidity_relative,
                pressure_pa,
                thi
            ) VALUES (
                $1::timestamptz,
                $2,
                $3,
                $4,
                $5
            )"#
        }
        DatabaseType::MySQL | DatabaseType::SQLite => {
            r#"
            INSERT INTO sensor_data (
                timestamp,
                temperature_c,
                humidity_relative,
                pressure_pa,
                thi
            ) VALUES (?, ?, ?, ?, ?)
            "#
        }
    }
}

impl Database {
    #[cfg(test)]
    async fn sensor_count(&self) -> Result<i64, BoxError> {
        let count = sqlx::query_scalar("SELECT COUNT(*) FROM sensor_data")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    #[cfg(test)]
    async fn latest_sensor_values(&self) -> Result<(String, f64, f64, f64, f64), BoxError> {
        let values = sqlx::query_as(
            r#"
            SELECT timestamp, temperature_c, humidity_relative, pressure_pa, thi
            FROM sensor_data
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(values)
    }
}

async fn insert_sensor_data(
    pool: &AnyPool,
    data: &SensorData,
    db_type: DatabaseType,
) -> Result<(), BoxError> {
    sqlx::query(insert_sql(db_type))
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
    use peripheral::bme280::Measurement;
    use std::sync::Arc;
    use tokio::time::{Duration, timeout};

    fn sample_timestamp() -> DateTime<Local> {
        DateTime::parse_from_rfc3339("2025-06-16T14:30:45+09:00")
            .unwrap()
            .with_timezone(&Local)
    }

    fn sample_sensor_data(temperature_c: f64) -> SensorData {
        SensorData {
            timestamp: sample_timestamp(),
            temperature_c,
            humidity_relative: 60.2,
            pressure_pa: 100_500.0,
            thi: 75.8,
        }
    }

    async fn wait_for_sensor_count(database: &Database, expected: i64) {
        timeout(Duration::from_secs(1), async {
            loop {
                if database.sensor_count().await.unwrap() == expected {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("database worker did not finish in time");
    }

    #[test]
    fn test_sensor_data_from_measurement() {
        let measurement = Measurement {
            temperature_c: 25.0,
            pressure_pa: 101325.0,
            humidity_relative: 50.0,
        };

        let before = Local::now();
        let sensor_data = SensorData::from_measurement(&measurement, 72.5);
        let after = Local::now();

        assert_eq!(sensor_data.temperature_c, 25.0);
        assert_eq!(sensor_data.pressure_pa, 101325.0);
        assert_eq!(sensor_data.humidity_relative, 50.0);
        assert_eq!(sensor_data.thi, 72.5);
        assert!(sensor_data.timestamp >= before);
        assert!(sensor_data.timestamp <= after);
    }

    #[test]
    fn test_database_type_from_url() {
        assert_eq!(
            database_type_from_url("postgresql://user:pass@localhost/db").unwrap(),
            DatabaseType::PostgreSQL
        );
        assert_eq!(
            database_type_from_url("mysql://user:pass@localhost/db").unwrap(),
            DatabaseType::MySQL
        );
        assert_eq!(
            database_type_from_url("sqlite::memory:").unwrap(),
            DatabaseType::SQLite
        );

        for invalid in [
            "invalid://connection",
            "postgresql_invalid",
            "mysql_invalid",
            "sqlite_invalid",
        ] {
            let error = database_type_from_url(invalid).unwrap_err();
            assert_eq!(error.to_string(), "Unsupported database URL scheme");
        }
    }

    #[test]
    fn test_database_sql_for_each_backend() {
        let postgres_create = create_table_sql(DatabaseType::PostgreSQL);
        assert!(postgres_create.contains("SERIAL PRIMARY KEY"));
        assert!(postgres_create.contains("TIMESTAMPTZ"));
        assert!(postgres_create.contains("DOUBLE PRECISION"));
        assert!(insert_sql(DatabaseType::PostgreSQL).contains("$1::timestamptz"));

        let mysql_create = create_table_sql(DatabaseType::MySQL);
        assert!(mysql_create.contains("INT AUTO_INCREMENT PRIMARY KEY"));
        assert!(mysql_create.contains("DATETIME(6)"));
        assert!(insert_sql(DatabaseType::MySQL).contains("VALUES (?, ?, ?, ?, ?)"));

        let sqlite_create = create_table_sql(DatabaseType::SQLite);
        assert!(sqlite_create.contains("INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sqlite_create.contains("timestamp TEXT NOT NULL"));
        assert!(insert_sql(DatabaseType::SQLite).contains("VALUES (?, ?, ?, ?, ?)"));
    }

    #[tokio::test]
    async fn test_database_sqlite_schema_creation() {
        let database = Database::new("sqlite::memory:").await.unwrap();

        let table_name: String = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'sensor_data'",
        )
        .fetch_one(&database.pool)
        .await
        .unwrap();

        assert_eq!(table_name, "sensor_data");
    }

    #[tokio::test]
    async fn test_database_save_persists_values() {
        let database = Database::new("sqlite::memory:").await.unwrap();
        let data = sample_sensor_data(23.5);
        let expected_timestamp = data.timestamp.to_rfc3339();

        database.save(data).await.unwrap();

        assert_eq!(database.sensor_count().await.unwrap(), 1);
        assert_eq!(
            database.latest_sensor_values().await.unwrap(),
            (expected_timestamp, 23.5, 60.2, 100_500.0, 75.8)
        );
    }

    #[tokio::test]
    async fn test_database_save_async_persists_value() {
        let database = Database::new("sqlite::memory:").await.unwrap();

        database.save_async(sample_sensor_data(24.0)).unwrap();
        wait_for_sensor_count(&database, 1).await;

        assert_eq!(database.latest_sensor_values().await.unwrap().1, 24.0);
    }

    #[tokio::test]
    async fn test_database_save_reports_insert_error() {
        let database = Database::new("sqlite::memory:").await.unwrap();
        sqlx::query("DROP TABLE sensor_data")
            .execute(&database.pool)
            .await
            .unwrap();

        let error = database.save(sample_sensor_data(25.0)).await.unwrap_err();

        assert!(error.to_string().contains("no such table"));
    }

    #[tokio::test]
    async fn test_database_invalid_connection_string() {
        let result = Database::new("invalid://connection").await;
        let error = result.err().expect("an unsupported URL should fail");

        assert_eq!(error.to_string(), "Unsupported database URL scheme");
    }

    #[tokio::test]
    async fn test_database_concurrent_saves() {
        let database = Arc::new(Database::new("sqlite::memory:").await.unwrap());
        let mut handles = Vec::new();

        for index in 0..10 {
            let database = Arc::clone(&database);
            handles.push(tokio::spawn(async move {
                database.save(sample_sensor_data(20.0 + index as f64)).await
            }));
        }

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        assert_eq!(database.sensor_count().await.unwrap(), 10);
    }

    #[tokio::test]
    #[ignore = "set TEST_POSTGRES_URL to run this integration test"]
    async fn test_database_postgresql_integration() {
        let url = std::env::var("TEST_POSTGRES_URL").expect("TEST_POSTGRES_URL must be set");
        let database = Database::new(&url).await.unwrap();

        database.save(sample_sensor_data(25.0)).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "set TEST_MYSQL_URL to run this integration test"]
    async fn test_database_mysql_integration() {
        let url = std::env::var("TEST_MYSQL_URL").expect("TEST_MYSQL_URL must be set");
        let database = Database::new(&url).await.unwrap();

        database.save(sample_sensor_data(25.0)).await.unwrap();
    }
}
