// MIT License
//
// Modified by Copyright (c) 2025 Yukke.org
// Original by Copyright (c) 2021 Neutroni
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

// https://www.bosch-sensortec.com/bst/products/all_products/bme280

//! BME280 Driver for Raspberry Pi

use rppal::i2c::{Error, I2c};
use std::thread;
use std::time::Duration;

/// BME280 I2C Address 1
pub const BME280_ADDR: u16 = 0x76;
/// BME280 I2C Address 2
pub const BME280_ADDR2: u16 = 0x77;

/// BME280 Driver
pub struct Bme280 {
    bus: I2c,
    calibration: CalibrationData,
}

impl Bme280 {
    /// Create a new BME280 instance.
    /// # Arguments
    /// * `addr` - I2C address of the BME280.
    /// # Returns
    /// * Result<Bme280, Error>
    pub fn new(addr: u16) -> Result<Bme280, Error> {
        let mut bus: I2c = I2c::new()?;
        //Default BME280 address is 0x76, but it can be set to 0x77
        bus.set_slave_address(addr)?;
        let calibration: CalibrationData = read_calibration(&bus)?;
        return Result::Ok(Bme280 { bus, calibration });
    }

    /// Make a measurement.
    /// # Returns
    /// * Result<Measurement, Error>
    pub fn make_measurement(&self) -> Result<Measurement, Error> {
        //Oversampling settings
        const OVERSAMPLE_TEMP: u8 = 1;
        const OVERSAMPLE_PRES: u8 = 1;
        const OVERSAMPLE_HUM: u8 = 1;
        //Forced mode: perform one measurement, store result and return to sleep mode
        const MODE: u8 = 1;
        const CONTROL: u8 = OVERSAMPLE_TEMP << 5 | OVERSAMPLE_PRES << 2 | MODE;
        //Register locations
        const REG_DATA: u8 = 0xF7;
        const REG_CONTROL: u8 = 0xF4;
        const REG_CONTROL_HUM: u8 = 0xF2;
        //Start the measurement
        self.bus.smbus_write_byte(REG_CONTROL_HUM, OVERSAMPLE_HUM)?;
        self.bus.smbus_write_byte(REG_CONTROL, CONTROL)?;
        //Wait for measurement to complete
        const WAIT_TIME: u64 = ((1.25
            + (2.3 * (OVERSAMPLE_TEMP as f64))
            + ((2.3 * (OVERSAMPLE_PRES as f64)) + 0.575)
            + ((2.3 * OVERSAMPLE_HUM as f64) + 0.575)) as u64)
            + 1;
        thread::sleep(Duration::from_millis(WAIT_TIME));
        //Read measured data
        let mut data: [u8; 8] = [0; 8];
        self.bus.block_read(REG_DATA, &mut data)?;
        //Parse read data to i32 values
        let pres_raw: i32 =
            ((data[0] as i32) << 12) | ((data[1] as i32) << 4) | ((data[2] as i32) >> 4);
        let temp_raw: i32 =
            ((data[3] as i32) << 12) | ((data[4] as i32) << 4) | ((data[5] as i32) >> 4);
        let hum_raw: i32 = ((data[6] as i32) << 8) | (data[7] as i32);
        //Refine read values
        let temperature_data: TemperatureData = refine_temperature(temp_raw, &self.calibration);
        let t_fine: i32 = temperature_data.t_fine;
        let temperature_c: f64 = temperature_data.temperature_c;
        let humidity_relative: f64 = refine_humidity(hum_raw, &self.calibration, t_fine);
        let pressure_pa: f64 = refine_pressure(pres_raw, &self.calibration, t_fine);

        return Result::Ok(Measurement {
            temperature_c,
            pressure_pa,
            humidity_relative,
        });
    }
}

/// Measurement data
#[derive(Copy, Clone, Debug)]
pub struct Measurement {
    /// Temperature in Celsius (°C)  
    /// Range: -40.0 to 85.0 +/- 0.01  
    /// Resolution: 0.01
    pub temperature_c: f64,
    /// Pressure in pascal (Pa)  
    /// Range: 30000.0 to 110000.0 +/- 100.0  
    /// Resolution: 0.18
    pub pressure_pa: f64,
    /// Humidity in percent (%)  
    /// Range: 0.0 to 100.0 +/- 3.0  
    /// Resolution: 0.008
    pub humidity_relative: f64,
}

/// Calibration data
#[derive(Debug)]
struct CalibrationData {
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    dig_p1: u16,
    dig_p2: i16,
    dig_p3: i16,
    dig_p4: i16,
    dig_p5: i16,
    dig_p6: i16,
    dig_p7: i16,
    dig_p8: i16,
    dig_p9: i16,
    dig_h1: u8,
    dig_h2: i16,
    dig_h3: u8,
    dig_h4: i16,
    dig_h5: i16,
    dig_h6: i8,
}

/// Temperature data
#[derive(Debug)]
struct TemperatureData {
    /// Temperature fine
    t_fine: i32,
    /// Temperature in Celsius
    temperature_c: f64,
}

/// Get i16 value from u8 array
/// # Arguments
/// * `arr` - u8 array
/// * `index` - index
/// # Returns
/// * i16
fn get_i16_from_u8_array(arr: &[u8], index: usize) -> i16 {
    return ((arr[index + 1] as i16) << 8) | (arr[index] as i16);
}

/// Get u16 value from u8 array
/// # Arguments
/// * `arr` - u8 array
/// * `index` - index
/// # Returns
/// * u16
fn get_u16_from_u8_array(arr: &[u8], index: usize) -> u16 {
    return ((arr[index + 1] as u16) << 8) | (arr[index] as u16);
}

/// Read calibration data
/// # Arguments
/// * `bus` - I2c
/// # Returns
/// * Result<CalibrationData, Error>
fn read_calibration(bus: &I2c) -> Result<CalibrationData, Error> {
    let mut cal1: [u8; 24] = [0; 24];
    bus.block_read(0x88, &mut cal1)?;
    let cal2: u8 = bus.smbus_read_byte(0xA1)?;
    let mut cal3: [u8; 7] = [0; 7];
    bus.block_read(0xE1, &mut cal3)?;

    //Convert byte data to word values
    let dig_t1: u16 = get_u16_from_u8_array(&cal1, 0);
    let dig_t2: i16 = get_i16_from_u8_array(&cal1, 2);
    let dig_t3: i16 = get_i16_from_u8_array(&cal1, 4);

    let dig_p1: u16 = get_u16_from_u8_array(&cal1, 6);
    let dig_p2: i16 = get_i16_from_u8_array(&cal1, 8);
    let dig_p3: i16 = get_i16_from_u8_array(&cal1, 10);
    let dig_p4: i16 = get_i16_from_u8_array(&cal1, 12);
    let dig_p5: i16 = get_i16_from_u8_array(&cal1, 14);
    let dig_p6: i16 = get_i16_from_u8_array(&cal1, 16);
    let dig_p7: i16 = get_i16_from_u8_array(&cal1, 18);
    let dig_p8: i16 = get_i16_from_u8_array(&cal1, 20);
    let dig_p9: i16 = get_i16_from_u8_array(&cal1, 22);

    let dig_h1: u8 = cal2;
    let dig_h2: i16 = get_i16_from_u8_array(&cal3, 0);
    let dig_h3: u8 = cal3[2];

    let e4: u8 = cal3[3];
    let e5: u8 = cal3[4];
    let e6: u8 = cal3[5];

    let dig_h4: i16 = ((e4 as i16) << 4) | ((e5 & 0x0F) as i16);
    let dig_h5: i16 = ((e6 as i16) << 4) | ((e5 >> 4) as i16);
    let dig_h6: i8 = cal3[6] as i8;

    return Result::Ok(CalibrationData {
        dig_t1,
        dig_t2,
        dig_t3,
        dig_p1,
        dig_p2,
        dig_p3,
        dig_p4,
        dig_p5,
        dig_p6,
        dig_p7,
        dig_p8,
        dig_p9,
        dig_h1,
        dig_h2,
        dig_h3,
        dig_h4,
        dig_h5,
        dig_h6,
    });
}

/// Refine temperature
/// # Arguments
/// * `temp_raw` - Raw temperature value
/// * `calibration` - Calibration data
/// # Returns
/// * TemperatureData - Refined temperature data
fn refine_temperature(temp_raw: i32, calibration: &CalibrationData) -> TemperatureData {
    let var1: f64 = ((temp_raw as f64) / 16384.0 - (calibration.dig_t1 as f64) / 1024.0)
        * (calibration.dig_t2 as f64);
    let var2: f64 = (((temp_raw as f64) / 131072.0 - (calibration.dig_t1 as f64) / 8192.0)
        * ((temp_raw as f64) / 131072.0 - (calibration.dig_t1 as f64) / 8192.0))
        * (calibration.dig_t3 as f64);
    let sum: f64 = var1 + var2;
    let t_fine: i32 = sum as i32;
    let temperature_c: f64 = sum / 5120.0;
    return TemperatureData {
        t_fine,
        temperature_c,
    };
}

/// Refine pressure
/// # Arguments
/// * `pres_raw` - Raw pressure value
/// * `calibration` - Calibration data
/// * `t_fine` - Temperature fine
/// # Returns
/// * f64 - Pressure in pascal
fn refine_pressure(pres_raw: i32, calibration: &CalibrationData, t_fine: i32) -> f64 {
    let mut var1: f64 = ((t_fine as f64) / 2.0) - 64000.0;
    let mut var2: f64 = var1 * var1 * (calibration.dig_p6 as f64) / 32768.0;
    var2 = var2 + var1 * (calibration.dig_p5 as f64) * 2.0;
    var2 = (var2 / 4.0) + ((calibration.dig_p4 as f64) * 65536.0);
    var1 = ((calibration.dig_p3 as f64) * var1 * var1 / 524288.0
        + (calibration.dig_p2 as f64) * var1)
        / 524288.0;
    var1 = (1.0 + var1 / 32768.0) * (calibration.dig_p1 as f64);
    if var1 == 0.0 {
        return 0.0; // avoid exception caused by division by zero
    }
    let mut p: f64 = 1048576.0 - (pres_raw as f64);
    p = (p - (var2 / 4096.0)) * 6250.0 / var1;
    var1 = (calibration.dig_p9 as f64) * p * p / 2147483648.0;
    var2 = p * (calibration.dig_p8 as f64) / 32768.0;
    p = p + (var1 + var2 + (calibration.dig_p7 as f64)) / 16.0;
    return p;
}

/// Refine humidity
/// # Arguments
/// * `hum_raw` - Raw humidity value
/// * `calibration` - Calibration data
/// * `t_fine` - Temperature fine
/// # Returns
/// * f64 - Humidity in percent
fn refine_humidity(hum_raw: i32, calibration: &CalibrationData, t_fine: i32) -> f64 {
    let mut var_h = (t_fine as f64) - 76800.0;
    var_h = ((hum_raw as f64)
        - ((calibration.dig_h4 as f64) * 64.0 + (calibration.dig_h5 as f64) / 16384.0 * var_h))
        * ((calibration.dig_h2 as f64) / 65536.0
            * (1.0
                + (calibration.dig_h6 as f64) / 67108864.0
                    * var_h
                    * (1.0 + (calibration.dig_h3 as f64) / 67108864.0 * var_h)));
    var_h = var_h * (1.0 - (calibration.dig_h1 as f64) * var_h / 524288.0);
    if var_h > 100.0 {
        var_h = 100.0;
    } else if var_h < 0.0 {
        var_h = 0.0;
    }
    return var_h;
}
