// MIT License
// Original by Copyright (c) 2021 Neutroni
// Modified by Copyright (c) 2025 Yukke.org
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

use embedded_hal::i2c::{I2c, SevenBitAddress};
use tokio::time::{Duration, sleep};

/// BME280 I2C Address 1
pub const BME280_ADDR: SevenBitAddress = 0x76;
/// BME280 I2C Address 2
pub const BME280_ADDR2: SevenBitAddress = 0x77;

/// BME280 Driver
pub struct Bme280<I2C> {
    bus: I2C,
    addr: SevenBitAddress,
    calibration: CalibrationData,
}

impl<I2C: I2c> Bme280<I2C> {
    /// Create a new BME280 instance.
    /// # Arguments
    /// * `bus` - I2C bus handle the sensor is reached through.
    /// * `addr` - I2C address of the BME280.
    /// # Returns
    /// * Result<Bme280<I2C>, I2C::Error>
    pub fn new(mut bus: I2C, addr: SevenBitAddress) -> Result<Bme280<I2C>, I2C::Error> {
        //Default BME280 address is 0x76, but it can be set to 0x77
        let calibration: CalibrationData = read_calibration(&mut bus, addr)?;
        Result::Ok(Bme280 {
            bus,
            addr,
            calibration,
        })
    }

    /// Make a measurement.
    /// # Returns
    /// * Result<Measurement, I2C::Error>
    pub async fn make_measurement(&mut self) -> Result<Measurement, I2C::Error> {
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
        self.bus
            .write(self.addr, &[REG_CONTROL_HUM, OVERSAMPLE_HUM])?;
        self.bus.write(self.addr, &[REG_CONTROL, CONTROL])?;
        //Wait for measurement to complete
        const WAIT_TIME: u64 = ((1.25
            + (2.3 * (OVERSAMPLE_TEMP as f64))
            + ((2.3 * (OVERSAMPLE_PRES as f64)) + 0.575)
            + ((2.3 * OVERSAMPLE_HUM as f64) + 0.575)) as u64)
            + 1;
        sleep(Duration::from_millis(WAIT_TIME)).await;
        //Read measured data
        let mut data: [u8; 8] = [0; 8];
        self.bus.write_read(self.addr, &[REG_DATA], &mut data)?;
        let raw = parse_raw_measurement(&data);
        //Refine read values
        let temperature_data: TemperatureData =
            refine_temperature(raw.temperature, &self.calibration);
        let t_fine: i32 = temperature_data.t_fine;
        let temperature_c: f64 = temperature_data.temperature_c;
        let humidity_relative: f64 = refine_humidity(raw.humidity, &self.calibration, t_fine);
        let pressure_pa: f64 = refine_pressure(raw.pressure, &self.calibration, t_fine);

        Result::Ok(Measurement {
            temperature_c,
            pressure_pa,
            humidity_relative,
        })
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
#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
struct RawMeasurement {
    pressure: i32,
    temperature: i32,
    humidity: i32,
}

/// Parse the eight measurement registers into uncompensated sensor values.
fn parse_raw_measurement(data: &[u8; 8]) -> RawMeasurement {
    RawMeasurement {
        pressure: i32::from(data[0]) << 12 | i32::from(data[1]) << 4 | i32::from(data[2]) >> 4,
        temperature: i32::from(data[3]) << 12 | i32::from(data[4]) << 4 | i32::from(data[5]) >> 4,
        humidity: i32::from(data[6]) << 8 | i32::from(data[7]),
    }
}

/// Get i16 value from u8 array
/// # Arguments
/// * `arr` - u8 array
/// * `index` - index
/// # Returns
/// * i16
fn get_i16_from_u8_array(arr: &[u8], index: usize) -> i16 {
    i16::from_le_bytes([arr[index], arr[index + 1]])
}

/// Get u16 value from u8 array
/// # Arguments
/// * `arr` - u8 array
/// * `index` - index
/// # Returns
/// * u16
fn get_u16_from_u8_array(arr: &[u8], index: usize) -> u16 {
    u16::from_le_bytes([arr[index], arr[index + 1]])
}

/// Build a signed 12-bit calibration value from its upper 8 bits and lower 4 bits.
fn get_i12_from_u8_parts(msb: u8, lsb: u8) -> i16 {
    (i16::from(i8::from_ne_bytes([msb])) << 4) | i16::from(lsb & 0x0F)
}

/// Read calibration data
/// # Arguments
/// * `bus` - I2C bus handle
/// * `addr` - I2C address of the BME280
/// # Returns
/// * Result<CalibrationData, I2C::Error>
fn read_calibration<I2C: I2c>(
    bus: &mut I2C,
    addr: SevenBitAddress,
) -> Result<CalibrationData, I2C::Error> {
    let mut cal1: [u8; 24] = [0; 24];
    bus.write_read(addr, &[0x88], &mut cal1)?;
    let mut cal2: [u8; 1] = [0; 1];
    bus.write_read(addr, &[0xA1], &mut cal2)?;
    let mut cal3: [u8; 7] = [0; 7];
    bus.write_read(addr, &[0xE1], &mut cal3)?;

    Ok(parse_calibration(&cal1, cal2[0], &cal3))
}

/// Parse calibration register values independently from I2C access.
fn parse_calibration(cal1: &[u8; 24], cal2: u8, cal3: &[u8; 7]) -> CalibrationData {
    //Convert byte data to word values
    let dig_t1: u16 = get_u16_from_u8_array(cal1, 0);
    let dig_t2: i16 = get_i16_from_u8_array(cal1, 2);
    let dig_t3: i16 = get_i16_from_u8_array(cal1, 4);

    let dig_p1: u16 = get_u16_from_u8_array(cal1, 6);
    let dig_p2: i16 = get_i16_from_u8_array(cal1, 8);
    let dig_p3: i16 = get_i16_from_u8_array(cal1, 10);
    let dig_p4: i16 = get_i16_from_u8_array(cal1, 12);
    let dig_p5: i16 = get_i16_from_u8_array(cal1, 14);
    let dig_p6: i16 = get_i16_from_u8_array(cal1, 16);
    let dig_p7: i16 = get_i16_from_u8_array(cal1, 18);
    let dig_p8: i16 = get_i16_from_u8_array(cal1, 20);
    let dig_p9: i16 = get_i16_from_u8_array(cal1, 22);

    let dig_h1: u8 = cal2;
    let dig_h2: i16 = get_i16_from_u8_array(cal3, 0);
    let dig_h3: u8 = cal3[2];

    let dig_h4: i16 = get_i12_from_u8_parts(cal3[3], cal3[4]);
    let dig_h5: i16 = get_i12_from_u8_parts(cal3[5], cal3[4] >> 4);
    let dig_h6: i8 = cal3[6] as i8;

    CalibrationData {
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
    }
}

/// Refine temperature
/// # Arguments
/// * `temp_raw` - Raw temperature value
/// * `calibration` - Calibration data
/// # Returns
/// * TemperatureData - Refined temperature data
fn refine_temperature(temp_raw: i32, calibration: &CalibrationData) -> TemperatureData {
    let var1 = (((temp_raw >> 3) - ((calibration.dig_t1 as i32) << 1))
        * (calibration.dig_t2 as i32))
        >> 11;
    let diff = (temp_raw >> 4) - (calibration.dig_t1 as i32);
    let var2 = (((diff * diff) >> 12) * (calibration.dig_t3 as i32)) >> 14;
    let t_fine = var1 + var2;
    let temperature_c = ((t_fine * 5 + 0x80) >> 8) as f64 / 100.0;
    TemperatureData {
        t_fine,
        temperature_c,
    }
}

/// Refine pressure
/// # Arguments
/// * `pres_raw` - Raw pressure value
/// * `calibration` - Calibration data
/// * `t_fine` - Temperature fine
/// # Returns
/// * f64 - Pressure in pascal
fn refine_pressure(pres_raw: i32, calibration: &CalibrationData, t_fine: i32) -> f64 {
    let mut var1 = (t_fine as i64) - 0x1F400;
    let mut var2 = var1 * var1 * (calibration.dig_p6 as i64);
    var2 += (var1 * (calibration.dig_p5 as i64)) << 17;
    var2 += (calibration.dig_p4 as i64) << 35;
    var1 = ((var1 * var1 * (calibration.dig_p3 as i64)) >> 8)
        + ((var1 * (calibration.dig_p2 as i64)) << 12);
    var1 = (((1_i64 << 47) + var1) * (calibration.dig_p1 as i64)) >> 33;
    if var1 == 0 {
        return 0.0; // avoid exception caused by division by zero
    }
    let mut p = 0x100000_i64 - (pres_raw as i64);
    p = (((p << 31) - var2) * 0xC35) / var1;
    var1 = ((calibration.dig_p9 as i64) * (p >> 13) * (p >> 13)) >> 25;
    var2 = ((calibration.dig_p8 as i64) * p) >> 19;
    p = ((p + var1 + var2) >> 8) + ((calibration.dig_p7 as i64) << 4);
    // p is in Q24.8 format: divide by 256 to get Pa
    p as f64 / 256.0
}

/// Refine humidity
/// # Arguments
/// * `hum_raw` - Raw humidity value
/// * `calibration` - Calibration data
/// * `t_fine` - Temperature fine
/// # Returns
/// * f64 - Humidity in percent
fn refine_humidity(hum_raw: i32, calibration: &CalibrationData, t_fine: i32) -> f64 {
    let v = (t_fine - 0x12C00) as i64;
    let part_a = ((((hum_raw as i64) << 14)
        - ((calibration.dig_h4 as i64) << 20)
        - ((calibration.dig_h5 as i64) * v))
        + 0x8000)
        >> 15;
    let part_b = (((((((calibration.dig_h6 as i64) * v) >> 10)
        * ((((calibration.dig_h3 as i64) * v) >> 11) + 0x8000))
        >> 10)
        + 0x200000)
        * (calibration.dig_h2 as i64)
        + 0x2000)
        >> 14;
    let mut result = part_a * part_b;
    result -= ((((result >> 15) * (result >> 15)) >> 7) * (calibration.dig_h1 as i64)) >> 4;
    result = result.clamp(0, 0x19000000);
    // result is in Q22.10 format: divide by 1024 to get %rH
    (result >> 12) as f64 / 1024.0
}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_hal::i2c::{ErrorKind, ErrorType, Operation};

    /// Calibration registers of the BME280 datasheet's reference part.
    const CAL1: [u8; 24] = [
        0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27, 0x0B, 0x8C,
        0x00, 0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17,
    ];
    const CAL2: u8 = 75;
    const CAL3: [u8; 7] = [0x6A, 0x01, 0x00, 0x13, 0x2B, 0x03, 0x1E];
    /// Uncompensated measurement registers matching the reference part.
    const MEASUREMENT: [u8; 8] = [0x65, 0x5A, 0xCF, 0x7E, 0xED, 0x0F, 0x75, 0x30];

    /// I2C bus stub: answers register reads from a canned map and records every
    /// frame the driver puts on the bus.
    #[derive(Default)]
    struct MockI2c {
        registers: Vec<(u8, Vec<u8>)>,
        frames: Vec<(SevenBitAddress, Vec<u8>)>,
    }

    impl MockI2c {
        fn with_register(mut self, register: u8, data: &[u8]) -> MockI2c {
            self.registers.push((register, data.to_vec()));
            self
        }

        fn read_register(&self, register: u8) -> Result<&[u8], ErrorKind> {
            self.registers
                .iter()
                .find(|(candidate, _)| *candidate == register)
                .map(|(_, data)| data.as_slice())
                .ok_or(ErrorKind::Other)
        }
    }

    impl ErrorType for MockI2c {
        type Error = ErrorKind;
    }

    impl I2c for MockI2c {
        fn transaction(
            &mut self,
            address: SevenBitAddress,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            let mut selected: Option<u8> = None;
            for operation in operations {
                match operation {
                    Operation::Write(bytes) => {
                        self.frames.push((address, bytes.to_vec()));
                        selected = bytes.first().copied();
                    }
                    Operation::Read(buffer) => {
                        let register = selected.ok_or(ErrorKind::Other)?;
                        let data = self.read_register(register)?;
                        buffer.copy_from_slice(&data[..buffer.len()]);
                    }
                }
            }
            Ok(())
        }
    }

    /// A stub bus preloaded with the reference part's registers.
    fn bus() -> MockI2c {
        MockI2c::default()
            .with_register(0x88, &CAL1)
            .with_register(0xA1, &[CAL2])
            .with_register(0xE1, &CAL3)
            .with_register(0xF7, &MEASUREMENT)
    }

    fn reference_calibration() -> CalibrationData {
        CalibrationData {
            dig_t1: 27504,
            dig_t2: 26435,
            dig_t3: -1000,
            dig_p1: 36477,
            dig_p2: -10685,
            dig_p3: 3024,
            dig_p4: 2855,
            dig_p5: 140,
            dig_p6: -7,
            dig_p7: 15500,
            dig_p8: -14600,
            dig_p9: 6000,
            dig_h1: 75,
            dig_h2: 362,
            dig_h3: 0,
            dig_h4: 315,
            dig_h5: 50,
            dig_h6: 30,
        }
    }

    #[test]
    fn test_parse_calibration() {
        let actual = parse_calibration(&CAL1, CAL2, &CAL3);

        assert_eq!(actual, reference_calibration());
    }

    #[test]
    fn test_new_selects_the_calibration_registers() {
        let sensor = Bme280::new(bus(), BME280_ADDR).unwrap();

        assert_eq!(
            sensor.bus.frames,
            [
                (BME280_ADDR, vec![0x88]),
                (BME280_ADDR, vec![0xA1]),
                (BME280_ADDR, vec![0xE1]),
            ]
        );
        assert_eq!(sensor.calibration, reference_calibration());
    }

    #[tokio::test]
    async fn test_make_measurement_drives_the_bus_and_compensates() {
        let mut sensor = Bme280::new(bus(), BME280_ADDR).unwrap();
        sensor.bus.frames.clear();

        let measurement = sensor.make_measurement().await.unwrap();

        // Humidity oversampling, then control (forced mode), then a read of the
        // eight measurement registers starting at 0xF7.
        assert_eq!(
            sensor.bus.frames,
            [
                (BME280_ADDR, vec![0xF2, 0x01]),
                (BME280_ADDR, vec![0xF4, 0x25]),
                (BME280_ADDR, vec![0xF7]),
            ]
        );
        assert!((measurement.temperature_c - 25.08).abs() < 0.005);
        assert!((measurement.pressure_pa - 100_653.25).abs() < 0.01);
        assert!((measurement.humidity_relative - 54.29).abs() < 0.1);
    }

    #[test]
    fn test_parse_signed_humidity_calibration() {
        let cal1 = [0; 24];
        let cal3 = [0x00, 0x00, 0x00, 0xFF, 0x0F, 0x80, 0xFF];

        let actual = parse_calibration(&cal1, 0, &cal3);

        assert_eq!(actual.dig_h4, -1);
        assert_eq!(actual.dig_h5, -2048);
        assert_eq!(actual.dig_h6, -1);
    }

    #[test]
    fn test_parse_raw_measurement() {
        assert_eq!(
            parse_raw_measurement(&MEASUREMENT),
            RawMeasurement {
                pressure: 415_148,
                temperature: 519_888,
                humidity: 30_000,
            }
        );
    }

    #[test]
    fn test_parse_raw_measurement_boundaries() {
        assert_eq!(
            parse_raw_measurement(&[0; 8]),
            RawMeasurement {
                pressure: 0,
                temperature: 0,
                humidity: 0,
            }
        );
        assert_eq!(
            parse_raw_measurement(&[0xFF; 8]),
            RawMeasurement {
                pressure: 0xF_FFFF,
                temperature: 0xF_FFFF,
                humidity: 0xFFFF,
            }
        );
    }

    #[test]
    fn test_get_u16_from_u8_array() {
        let data = [0x34, 0x12, 0x78, 0x56];
        let result = get_u16_from_u8_array(&data, 0);
        assert_eq!(result, 0x1234);

        let result2 = get_u16_from_u8_array(&data, 2);
        assert_eq!(result2, 0x5678);
    }

    #[test]
    fn test_get_i16_from_u8_array() {
        let data = [0xFF, 0xFF, 0x00, 0x01];
        let result = get_i16_from_u8_array(&data, 0);
        assert_eq!(result, -1);

        let result2 = get_i16_from_u8_array(&data, 2);
        assert_eq!(result2, 256);
    }

    #[test]
    fn test_refine_temperature_with_reference_values() {
        let result = refine_temperature(519_888, &reference_calibration());

        assert_eq!(result.t_fine, 128_422);
        assert!((result.temperature_c - 25.08).abs() < 0.005);
    }

    #[test]
    fn test_refine_pressure_with_reference_values() {
        let pressure = refine_pressure(415_148, &reference_calibration(), 128_422);

        assert!((pressure - 100_653.25).abs() < 0.01);
    }

    #[test]
    fn test_refine_pressure_zero_division() {
        let calibration = CalibrationData {
            dig_t1: 0,
            dig_t2: 0,
            dig_t3: 0,
            dig_p1: 0,
            dig_p2: 0,
            dig_p3: 0,
            dig_p4: 0,
            dig_p5: 0,
            dig_p6: 0,
            dig_p7: 0,
            dig_p8: 0,
            dig_p9: 0,
            dig_h1: 0,
            dig_h2: 0,
            dig_h3: 0,
            dig_h4: 0,
            dig_h5: 0,
            dig_h6: 0,
        };

        let result = refine_pressure(100000, &calibration, 128000);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_refine_humidity_with_reference_values() {
        let humidity = refine_humidity(30_000, &reference_calibration(), 128_422);

        assert!((humidity - 54.29).abs() < 0.1);
    }

    #[test]
    fn test_refine_humidity_clamps_to_sensor_range() {
        let calibration = reference_calibration();

        assert_eq!(refine_humidity(0, &calibration, 128_422), 0.0);
        assert_eq!(
            refine_humidity(u16::MAX.into(), &calibration, 128_422),
            100.0
        );
    }
}
