// MIT License
//
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
use std::thread;
use std::time;

use chrono::prelude::*;

use peripheral::bme280;
use peripheral::so1602a;

/// Entry point of the program.
/// This program reads temperature and humidity data from a BME280 sensor
/// and displays it on a SO1602A LCD. It also shows a custom character
/// (backslash dot) on the LCD.
/// The program runs indefinitely, updating the display every 200 milliseconds.
/// # Returns
/// * `Ok(())` if the program runs successfully.
/// * `Err(e)` if there is an error during execution.
fn main() -> Result<(), Box<dyn Error>> {
    let so1602a = so1602a::SO1602A::new(so1602a::SO1602A_ADDR)?;
    let bme280 = bme280::Bme280::new(bme280::BME280_ADDR)?;
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

    so1602a.setup()?;
    for (index, data) in char_data {
        so1602a.register_char(index, data)?;
    }

    loop {
        let now = Local::now();
        let measurement = bme280.make_measurement()?;

        so1602a.put_str(
            so1602a::SO1602A_1ST_LINE,
            &format!("{}", now.format("%Y/%m/%d %H:%M")),
        )?;
        so1602a.put_str(
            so1602a::SO1602A_2ND_LINE,
            &format!(
                "{: >2.1}C {: >3.1}% {: >3.0}",
                measurement.temperature_c,
                measurement.humidity_relative,
                calc_thi(measurement.temperature_c, measurement.humidity_relative),
            ),
        )?;
        so1602a.put_u8(so1602a::SO1602A_2ND_LINE + 15, indicator[counter & 0x3])?;

        thread::sleep(time::Duration::from_millis(200));

        counter += 1;
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
pub fn calc_thi(temperature: f64, humidity: f64) -> f64 {
    0.81 * temperature + 0.01 * humidity * (0.99 * temperature - 14.3) + 46.3
}
