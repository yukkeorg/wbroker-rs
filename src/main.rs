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

use std::cmp;

use chrono::prelude::*;
use clap::Parser;
use peripheral::bme280::Measurement;
use tokio::time::{Duration, Instant, sleep};

use peripheral::bme280;
use peripheral::so1602a;
use peripheral::{DEFAULT_I2C_BUS, I2cdev};

mod config;
mod database;
mod types;

use crate::types::BoxError;

use config::Config;
use database::{Database, SensorData};

const INTERVAL: u64 = 500;
#[cfg(test)]
const DISPLAY_WIDTH: usize = 16;

// Custom characters data
const CUSTOM_CHAR_DATA: [(u8, [u8; 8]); 2] = [
    (
        // Backslash dot data
        0x01,
        [
            0b00000,
            0b10000,
            0b01000,
            0b00100,
            0b00010,
            0b00001,
            0b00000,
            0b00000,
        ],
    ),
    (
        // Celcius sign
        0x02,
        [
            0b01000,
            0b10100,
            0b01110,
            0b01001,
            0b01000,
            0b01001,
            0b00110,
            0b00000,
        ],
    ),
];

const BACKSLASH: &str = "\x01";
const INDICATOR: [&str; 4] = [BACKSLASH, "|", "/", "-"];

#[derive(Parser)]
#[command(name = "wbroker-rs")]
#[command(about = "Temperature and humidity monitoring system for Raspberry Pi")]
struct Args {
    #[arg(short, long, env = "WBROKER_CONFIG", default_value = "config.toml")]
    #[arg(help = "Path to configuration file")]
    config_filepath: String,
}

/// Entry point of the program.
/// This program reads temperature and humidity data from a BME280 sensor
/// and displays it on a SO1602A LCD. It also shows a custom character
/// (backslash dot) on the LCD.
/// The program runs indefinitely, updating the display every 200 milliseconds.
/// # Returns
/// * `Ok(())` if the program runs successfully.
/// * `Err(e)` if there is an error during execution.
#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let args = Args::parse();
    let (config, config_loaded) = Config::load_or_default_with_status(&args.config_filepath);

    // One handle per device: `I2cdev` holds the slave address on the open file,
    // so a shared handle would reopen the device file on every address switch.
    let mut so1602a = so1602a::SO1602A::new(I2cdev::new(DEFAULT_I2C_BUS)?, so1602a::SO1602A_ADDR);
    let mut bme280 = bme280::Bme280::new(I2cdev::new(DEFAULT_I2C_BUS)?, bme280::BME280_ADDR)?;

    let database = init_database(&config, config_loaded).await?;
    let mut indicator_iter = INDICATOR.iter().cycle();

    init_display(&mut so1602a).await?;

    loop {
        // 更新間隔を調整するために１ループの処理にかかる時間を計測する。
        let processing_interval = Instant::now();

        // 収集するデータを取得
        let now = Local::now();
        let measurement = bme280.make_measurement().await?;
        let thi = calc_thi(measurement.temperature_c, measurement.humidity_relative);

        update_display(
            &mut so1602a,
            &now,
            measurement.temperature_c,
            measurement.humidity_relative,
            thi,
            indicator_iter.next().unwrap(),
        )?;

        update_database(&database, &measurement, thi).await?;

        // 処理時間をチェックして、規定の時間以内で処理していたら、規定時間まで待つ
        let delta = processing_interval.elapsed();
        let adjustment_wait = cmp::max(Duration::from_millis(INTERVAL) - delta, Duration::ZERO);
        if adjustment_wait > Duration::ZERO {
            sleep(adjustment_wait).await;
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

/// Calculate the temperature-humidity index.
/// # Arguments
/// * `temperature` - Temperature in Celsius.
/// * `humidity` - Relative humidity in %.
/// # Returns
/// * Temperature-humidity index.
fn calc_thi(temperature: f64, humidity: f64) -> f64 {
    0.81 * temperature + 0.01 * humidity * (0.99 * temperature - 14.3) + 46.3
}

/// Initialize display
/// # Arguments:
/// * `so1602a` - SO1602A module object
/// # Returns:
/// * None
async fn init_display(so1602a: &mut so1602a::SO1602A<I2cdev>) -> Result<(), BoxError> {
    so1602a.setup().await?;
    for (index, data) in CUSTOM_CHAR_DATA {
        so1602a.register_char(index, data)?;
    }
    Ok(())
}

/// Initialize database access.
/// # Arguments:
/// * `confit` - Config object
/// * `config_loaded` - a flag for config file is loaded
/// # Returns
/// Database object.
async fn init_database(config: &Config, config_loaded: bool) -> Result<Option<Database>, BoxError> {
    if config_loaded {
        let db = Database::new(&config.database.url)
            .await
            .map_err(|e| format!("Failed to initialize database: {}", e))?;
        Ok(Some(db))
    } else {
        println!("No config file found. Running without database logging.");
        Ok(None)
    }
}

/// Build the two lines shown on the display without performing I/O.
fn format_display_lines(
    now: &NaiveDateTime,
    temperature: f64,
    humidity: f64,
    thi: f64,
    indicator: &str,
) -> [String; 2] {
    let humidity_with_decimal = format!("{humidity:.1}");
    let humidity = if humidity_with_decimal.len() <= 4 {
        format!("{humidity_with_decimal:>4}")
    } else {
        format!("{humidity:>4.0}")
    };

    [
        now.format("%Y/%m/%d %H:%M").to_string(),
        format!("{temperature:>+5.1}\x02 {humidity}%{thi:>3.0}{indicator}"),
    ]
}

// Update display
// # Args:
// * `so1602a` -  SO1602A Object
// * `now` - Datetime for displaying
// * `temperature` - value of temperature
// * `humidity` - value of humidity
// * `thi` - value of thi
// * `indicator` - Indicator charactor
// # Returns:
//  None
fn update_display(
    so1602a: &mut so1602a::SO1602A<I2cdev>,
    now: &DateTime<Local>,
    temperature: f64,
    humidity: f64,
    thi: f64,
    indicator: &str,
) -> Result<(), BoxError> {
    let [line1, line2] =
        format_display_lines(&now.naive_local(), temperature, humidity, thi, indicator);

    so1602a.put_str(so1602a::SO1602A_1ST_LINE, &line1)?;
    so1602a.put_str(so1602a::SO1602A_2ND_LINE, &line2)?;

    Ok(())
}

// Update databaase
// # Args
// - `database` (Optional<&Database>) - database object
// - `measurement` (&Mesurement) - mesurement object
// - `thi` (f64) - value of thi
async fn update_database(
    database: &Option<Database>,
    measurement: &Measurement,
    thi: f64,
) -> Result<(), BoxError> {
    if let Some(database) = database {
        let sensor_data = SensorData::from_measurement(measurement, thi);
        if let Err(e) = database.save_async(sensor_data) {
            eprintln!("Failed to queue sensor data for saving: {}", e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_thi_reference_values() {
        let cases = [
            (25.0, 50.0, 71.775),
            (35.0, 80.0, 90.93),
            (5.0, 20.0, 48.48),
            (0.0, 0.0, 46.3),
            (-10.0, 30.0, 30.94),
            (25.0, 100.0, 77.0),
        ];

        for (temperature, humidity, expected) in cases {
            let actual = calc_thi(temperature, humidity);
            assert!(
                (actual - expected).abs() < 1e-10,
                "temperature={temperature}, humidity={humidity}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn test_custom_character_data() {
        assert_eq!(CUSTOM_CHAR_DATA.len(), 2);
        assert_eq!(CUSTOM_CHAR_DATA[0].0, 0x01);
        assert_eq!(CUSTOM_CHAR_DATA[1].0, 0x02);

        for (index, rows) in CUSTOM_CHAR_DATA {
            assert!(index < 8);
            assert!(rows.iter().all(|row| *row <= 0b1_1111));
        }
    }

    #[test]
    fn test_indicator_cycle() {
        let actual: Vec<&str> = INDICATOR.iter().cycle().take(8).copied().collect();

        assert_eq!(actual, ["\x01", "|", "/", "-", "\x01", "|", "/", "-"]);
    }

    #[test]
    fn test_display_format_strings() {
        let now = NaiveDate::from_ymd_opt(2025, 6, 15)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap();

        let [line1, line2] = format_display_lines(&now, 23.7, 65.2, 72.5, "|");

        assert_eq!(line1, "2025/06/15 12:34");
        assert_eq!(line2, "+23.7\x02 65.2% 72|");

        assert_eq!(line1.len(), DISPLAY_WIDTH);
        assert_eq!(line2.len(), DISPLAY_WIDTH);
    }

    #[test]
    fn test_display_format_strings_at_sensor_limits() {
        let now = NaiveDate::from_ymd_opt(2025, 6, 15)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap();

        let [_, lower_line] = format_display_lines(&now, -40.0, 0.0, -40.0, "-");
        let [_, upper_line] = format_display_lines(&now, 85.0, 100.0, 185.0, "/");

        assert_eq!(lower_line, "-40.0\x02  0.0%-40-");
        assert_eq!(upper_line, "+85.0\x02  100%185/");
        assert_eq!(lower_line.len(), DISPLAY_WIDTH);
        assert_eq!(upper_line.len(), DISPLAY_WIDTH);
    }
}
