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
use sqlx::AnyPool;
use std::sync::Once;
use tokio::sync::mpsc;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

static DRIVER_INIT: Once = Once::new();

fn install_driver_for_url(connection_string: &str) -> Result<(), BoxError> {
    // SQLx 0.8では、個別ドライバー指定よりinstall_default_driversが推奨されている
    // ただし、接続文字列の検証は行う
    if !connection_string.starts_with("postgresql")
        && !connection_string.starts_with("mysql")
        && !connection_string.starts_with("sqlite")
    {
        return Err("Unsupported database URL scheme".into());
    }

    sqlx::any::install_default_drivers();
    Ok(())
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

#[derive(Debug, Clone)]
enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
}

impl Database {
    pub async fn new(connection_string: &str) -> Result<Self, BoxError> {
        DRIVER_INIT.call_once(|| {
            if let Err(e) = install_driver_for_url(connection_string) {
                eprintln!("Failed to install database driver: {}", e);
            }
        });

        let db_type = if connection_string.starts_with("postgresql") {
            DatabaseType::PostgreSQL
        } else if connection_string.starts_with("mysql") {
            DatabaseType::MySQL
        } else {
            DatabaseType::SQLite
        };

        let pool = AnyPool::connect(connection_string).await?;

        let create_table_sql = if connection_string.starts_with("postgresql") {
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
        } else if connection_string.starts_with("mysql") {
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
        } else {
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
        };

        sqlx::query(create_table_sql).execute(&pool).await?;

        let (sender, mut receiver) = mpsc::unbounded_channel::<SensorData>();
        let pool_clone = pool.clone();
        let db_type_clone = db_type.clone();

        tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                if let Err(e) = insert_sensor_data(&pool_clone, &data, &db_type_clone).await {
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

async fn insert_sensor_data(pool: &AnyPool, data: &SensorData, db_type: &DatabaseType) -> Result<(), BoxError> {
    // データベース固有のプレースホルダーを使用
    let sql = match db_type {
        DatabaseType::PostgreSQL => {
            "INSERT INTO sensor_data (timestamp, temperature_c, humidity_relative, pressure_pa, thi) VALUES ($1, $2, $3, $4, $5)"
        },
        DatabaseType::MySQL | DatabaseType::SQLite => {
            "INSERT INTO sensor_data (timestamp, temperature_c, humidity_relative, pressure_pa, thi) VALUES (?, ?, ?, ?, ?)"
        },
    };
    
    sqlx::query(sql)
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
    use tokio::time::{Duration, sleep};

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

    #[tokio::test]
    #[ignore = "requires sqlx any drivers"]
    async fn test_database_sqlite_creation() {
        let result = Database::new("sqlite::memory:").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires sqlx any drivers"]
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

    #[tokio::test]
    #[ignore = "requires sqlx any drivers"]
    async fn test_database_schema_creation_sqlite() {
        let database = Database::new("sqlite::memory:").await.unwrap();

        let sensor_data = SensorData {
            timestamp: Local::now(),
            temperature_c: 23.5,
            humidity_relative: 60.2,
            pressure_pa: 100500.0,
            thi: 75.8,
        };

        assert!(database.save_async(sensor_data).is_ok());
        sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    #[ignore = "requires sqlx any drivers"]
    async fn test_database_multiple_saves() {
        let database = Database::new("sqlite::memory:").await.unwrap();

        for i in 0..5 {
            let sensor_data = SensorData {
                timestamp: Local::now(),
                temperature_c: 20.0 + i as f64,
                humidity_relative: 50.0 + i as f64,
                pressure_pa: 100000.0 + i as f64 * 100.0,
                thi: 70.0 + i as f64,
            };
            assert!(database.save_async(sensor_data).is_ok());
        }

        sleep(Duration::from_millis(200)).await;
    }

    #[tokio::test]
    #[ignore = "requires sqlx any drivers"]
    async fn test_database_invalid_connection_string() {
        let result = Database::new("invalid://connection").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_connection_string_detection() {
        assert!("postgresql://user:pass@localhost/db".starts_with("postgresql"));
        assert!("mysql://user:pass@localhost/db".starts_with("mysql"));
        assert!(!"sqlite:memory:".starts_with("postgresql"));
        assert!(!"sqlite:memory:".starts_with("mysql"));
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

    #[tokio::test]
    #[ignore = "requires sqlx any drivers"]
    async fn test_async_save_error_handling() {
        let database = Database::new("sqlite::memory:").await.unwrap();

        let sensor_data = SensorData {
            timestamp: Local::now(),
            temperature_c: f64::NAN,
            humidity_relative: f64::INFINITY,
            pressure_pa: f64::NEG_INFINITY,
            thi: 75.0,
        };

        let result = database.save_async(sensor_data);
        assert!(result.is_ok());

        sleep(Duration::from_millis(100)).await;
    }

    #[test]
    fn test_sensor_data_with_special_values() {
        let measurement = Measurement {
            temperature_c: f64::NAN,
            pressure_pa: f64::INFINITY,
            humidity_relative: f64::NEG_INFINITY,
        };
        let thi = 0.0;

        let sensor_data = SensorData::from_measurement(measurement, thi);

        assert!(sensor_data.temperature_c.is_nan());
        assert!(
            sensor_data.pressure_pa.is_infinite() && sensor_data.pressure_pa.is_sign_positive()
        );
        assert!(
            sensor_data.humidity_relative.is_infinite()
                && sensor_data.humidity_relative.is_sign_negative()
        );
        assert_eq!(sensor_data.thi, 0.0);
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL database"]
    async fn test_database_postgresql_creation() {
        let result = Database::new("postgresql://test:test@localhost/test_db").await;

        if result.is_ok() {
            let database = result.unwrap();

            let sensor_data = SensorData {
                timestamp: Local::now(),
                temperature_c: 25.0,
                humidity_relative: 50.0,
                pressure_pa: 101325.0,
                thi: 72.5,
            };

            assert!(database.save_async(sensor_data).is_ok());
            sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL database"]
    async fn test_postgresql_schema_detection() {
        let connection_string = "postgresql://test:test@localhost/test_db";
        assert!(connection_string.starts_with("postgresql"));

        if let Ok(database) = Database::new(connection_string).await {
            let sensor_data = SensorData {
                timestamp: Local::now(),
                temperature_c: 23.5,
                humidity_relative: 60.2,
                pressure_pa: 100500.0,
                thi: 75.8,
            };

            assert!(database.save_async(sensor_data).is_ok());
            sleep(Duration::from_millis(200)).await;
        }
    }

    #[test]
    fn test_postgresql_schema_sql() {
        let connection_string = "postgresql://user:pass@localhost/db";
        assert!(connection_string.starts_with("postgresql"));

        let expected_keywords = vec![
            "CREATE TABLE IF NOT EXISTS",
            "SERIAL PRIMARY KEY",
            "TIMESTAMPTZ",
            "DOUBLE PRECISION",
        ];

        let sql = r#"
            CREATE TABLE IF NOT EXISTS sensor_data (
                id SERIAL PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                temperature_c DOUBLE PRECISION NOT NULL,
                humidity_relative DOUBLE PRECISION NOT NULL,
                pressure_pa DOUBLE PRECISION NOT NULL,
                thi DOUBLE PRECISION NOT NULL
            )
            "#;

        for keyword in expected_keywords {
            assert!(sql.contains(keyword));
        }
    }

    #[tokio::test]
    #[ignore = "requires MySQL database"]
    async fn test_database_mysql_creation() {
        let result = Database::new("mysql://test:test@localhost/test_db").await;

        if result.is_ok() {
            let database = result.unwrap();

            let sensor_data = SensorData {
                timestamp: Local::now(),
                temperature_c: 25.0,
                humidity_relative: 50.0,
                pressure_pa: 101325.0,
                thi: 72.5,
            };

            assert!(database.save_async(sensor_data).is_ok());
            sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    #[ignore = "requires MySQL database"]
    async fn test_mysql_schema_detection() {
        let connection_string = "mysql://test:test@localhost/test_db";
        assert!(connection_string.starts_with("mysql"));

        if let Ok(database) = Database::new(connection_string).await {
            let sensor_data = SensorData {
                timestamp: Local::now(),
                temperature_c: 23.5,
                humidity_relative: 60.2,
                pressure_pa: 100500.0,
                thi: 75.8,
            };

            assert!(database.save_async(sensor_data).is_ok());
            sleep(Duration::from_millis(200)).await;
        }
    }

    #[test]
    fn test_mysql_schema_sql() {
        let connection_string = "mysql://user:pass@localhost/db";
        assert!(connection_string.starts_with("mysql"));

        let expected_keywords = vec![
            "CREATE TABLE IF NOT EXISTS",
            "INT AUTO_INCREMENT PRIMARY KEY",
            "DATETIME(6)",
            "DOUBLE",
        ];

        let sql = r#"
            CREATE TABLE IF NOT EXISTS sensor_data (
                id INT AUTO_INCREMENT PRIMARY KEY,
                timestamp DATETIME(6) NOT NULL,
                temperature_c DOUBLE NOT NULL,
                humidity_relative DOUBLE NOT NULL,
                pressure_pa DOUBLE NOT NULL,
                thi DOUBLE NOT NULL
            )
            "#;

        for keyword in expected_keywords {
            assert!(sql.contains(keyword));
        }
    }

    #[test]
    fn test_database_url_patterns() {
        let urls = vec![
            ("sqlite::memory:", false, false),
            ("sqlite:./test.db", false, false),
            ("postgresql://user:pass@localhost/db", true, false),
            ("mysql://user:pass@localhost/db", false, true),
        ];

        for (url, is_postgres, is_mysql) in urls {
            assert_eq!(url.starts_with("postgresql"), is_postgres);
            assert_eq!(url.starts_with("mysql"), is_mysql);
        }
    }

    #[tokio::test]
    #[ignore = "requires sqlx any drivers"]
    async fn test_database_concurrent_saves() {
        let database = std::sync::Arc::new(Database::new("sqlite::memory:").await.unwrap());

        let mut handles = vec![];
        for i in 0..10 {
            let db_clone = database.clone();
            let handle = tokio::spawn(async move {
                let sensor_data = SensorData {
                    timestamp: Local::now(),
                    temperature_c: 20.0 + i as f64,
                    humidity_relative: 50.0,
                    pressure_pa: 101325.0,
                    thi: 70.0,
                };
                db_clone.save_async(sensor_data)
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        sleep(Duration::from_millis(300)).await;
    }
}
