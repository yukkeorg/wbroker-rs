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

use std::error::Error;

use chrono::prelude::*;
use clap::Parser;
use tokio::time::{Duration, interval};

use peripheral::bme280;
use peripheral::so1602a;

mod config;
mod database;
use config::Config;
use database::{Database, SensorData};

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
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let (config, config_loaded) = Config::load_or_default_with_status(&args.config_filepath);

    let so1602a = so1602a::SO1602A::new(so1602a::SO1602A_ADDR)?;
    let bme280 = bme280::Bme280::new(bme280::BME280_ADDR)?;

    let database = if config_loaded {
        Some(
            Database::new(&config.database.url)
                .await
                .map_err(|e| format!("Failed to initialize database: {}", e))?,
        )
    } else {
        println!("No config file found. Running without database logging.");
        None
    };
    let indicator: [u8; 4] = [0x01, b'|', b'/', b'-'];
    let mut counter: usize = 0;

    // Custom characters data
    let char_data: [(u8, [u8; 8]); 1] = [(
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
    )];

    so1602a.setup().await?;
    for (index, data) in char_data {
        so1602a.register_char(index, data)?;
    }

    let mut interval = interval(Duration::from_millis(200));

    loop {
        interval.tick().await;

        let now = Local::now();
        let measurement = bme280.make_measurement().await?;
        let thi = calc_thi(measurement.temperature_c, measurement.humidity_relative);

        so1602a.put_str(
            so1602a::SO1602A_1ST_LINE,
            &format!("{}", now.format("%Y/%m/%d %H:%M")),
        )?;
        so1602a.put_str(
            so1602a::SO1602A_2ND_LINE,
            &format!(
                "{: >2.1}C {: >3.1}% {: >3.0}",
                measurement.temperature_c, measurement.humidity_relative, thi,
            ),
        )?;

        so1602a.put_u8(so1602a::SO1602A_2ND_LINE + 15, indicator[counter])?;

        if let Some(ref database) = database {
            let sensor_data = SensorData::from_measurement(measurement, thi);
            if let Err(e) = database.save_async(sensor_data) {
                eprintln!("Failed to queue sensor data for saving: {}", e);
            }
        }

        counter = (counter + 1) & 0x03;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_thi_normal_conditions() {
        let temperature = 25.0;
        let humidity = 50.0;
        let thi = calc_thi(temperature, humidity);

        let expected = 0.81 * 25.0 + 0.01 * 50.0 * (0.99 * 25.0 - 14.3) + 46.3;
        assert_eq!(thi, expected);
        assert!(thi > 20.0 && thi < 100.0);
    }

    #[test]
    fn test_calc_thi_hot_humid() {
        let temperature = 35.0;
        let humidity = 80.0;
        let thi = calc_thi(temperature, humidity);

        assert!(thi > 30.0);
        assert!(thi < 120.0);
    }

    #[test]
    fn test_calc_thi_cold_dry() {
        let temperature = 5.0;
        let humidity = 20.0;
        let thi = calc_thi(temperature, humidity);

        assert!(thi < 60.0);
        assert!(thi > 0.0);
    }

    #[test]
    fn test_calc_thi_zero_values() {
        let thi = calc_thi(0.0, 0.0);
        assert_eq!(thi, 46.3);
    }

    #[test]
    fn test_calc_thi_formula_components() {
        let temperature = 20.0;
        let humidity = 60.0;

        let component1 = 0.81 * temperature;
        let component2 = 0.01 * humidity * (0.99 * temperature - 14.3);
        let component3 = 46.3;

        let expected = component1 + component2 + component3;
        let actual = calc_thi(temperature, humidity);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_calc_thi_negative_temperature() {
        let temperature = -10.0;
        let humidity = 30.0;
        let thi = calc_thi(temperature, humidity);

        assert!(thi < 50.0);
    }

    #[test]
    fn test_calc_thi_high_humidity() {
        let temperature = 25.0;
        let humidity = 100.0;
        let thi = calc_thi(temperature, humidity);

        let thi_low_humidity = calc_thi(temperature, 0.0);
        assert!(thi > thi_low_humidity);
    }

    #[test]
    fn test_calc_thi_precision() {
        let temperature = 22.5;
        let humidity = 55.5;
        let thi = calc_thi(temperature, humidity);

        let rounded_thi = (thi * 10.0).round() / 10.0;
        assert!((thi - rounded_thi).abs() < 0.1);
    }

    #[test]
    fn test_char_data_format() {
        let char_data: [(u8, [u8; 8]); 1] = [(
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
        )];

        assert_eq!(char_data.len(), 1);
        assert_eq!(char_data[0].0, 0x01);
        assert_eq!(char_data[0].1.len(), 8);
        assert!(char_data[0].1.iter().all(|&b| b <= 0b11111));
    }

    #[test]
    fn test_indicator_array() {
        let indicator: [u8; 4] = [0x01, b'|', b'/', b'-'];

        assert_eq!(indicator.len(), 4);
        assert_eq!(indicator[0], 0x01);
        assert_eq!(indicator[1], b'|');
        assert_eq!(indicator[2], b'/');
        assert_eq!(indicator[3], b'-');
    }

    #[test]
    fn test_counter_masking() {
        let indicator: [u8; 4] = [0x01, b'|', b'/', b'-'];

        for counter in 0..16 {
            let index = counter & 0x3;
            assert!(index < 4);
            assert_eq!(indicator[index], indicator[counter % 4]);
        }
    }

    #[test]
    fn test_display_format_strings() {
        let temperature = 23.7;
        let humidity = 65.2;
        let thi = 72.5;

        let line2_format = format!("{: >2.1}C {: >3.1}% {: >3.0}", temperature, humidity, thi);

        assert!(line2_format.contains("23.7"));
        assert!(line2_format.contains("65.2"));
        assert!(line2_format.contains("72"));
    }
}
