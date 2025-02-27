use std::error::Error;
use std::thread;
use std::time;

use chrono::prelude::*;

use wbroker_rs::{bme280, helper, so1602a};

fn main() -> Result<(), Box<dyn Error>> {
    let so1602a = so1602a::SO1602A::new(so1602a::SO1602A_ADDR)?;
    let bme280 = bme280::Bme280::new(bme280::BME280_ADDR)?;
    let indicator: [u8; 4] = [0x01, b'|', b'/', b'-'];
    let mut counter: usize = 0;

    so1602a.setup()?;
    so1602a.register_char(0x01, [0b00000000,
                                 0b00010000,
                                 0b00001000,
                                 0b00000100,
                                 0b00000010,
                                 0b00000001,
                                 0b00000000,
                                 0b00000000])?;

    loop {
        let now = Local::now();
        let measurement = bme280.make_measurement()?;

        so1602a.print(
            so1602a::SO1602A_1ST_LINE,
            &format!("{}", now.format("%Y/%m/%d %H:%M")),
        )?;
        so1602a.print(
            so1602a::SO1602A_2ND_LINE,
            &format!(
                "{: >2.1}C {: >2.1}% {: >3.0}",
                measurement.temperature_c,
                measurement.humidity_relative,
                helper::calc_thi(measurement.temperature_c, measurement.humidity_relative),
            ),
        )?;
        so1602a.put_u8(so1602a::SO1602A_2ND_LINE+15, indicator[counter & 0x3])?;

        thread::sleep(time::Duration::from_millis(200));

        counter += 1;
    }

    #[allow(unreachable_code)]
    Ok(())
}
