use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use rppal::i2c::I2c;

pub const SO1602A_ADDR: u16 = 0x3c;

pub struct SO1602A {
    i2c: I2c,
}

impl SO1602A {
    pub fn new(addr: u16) -> Result<SO1602A, Box<dyn Error>> {
        let mut i2c = I2c::new()?;
        i2c.set_slave_address(addr)?;
        Ok(SO1602A { i2c })
    }

    pub fn send_command(&self, data: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.smbus_write_byte(0, data)?;
        Ok(())
    }

    pub fn send_data(&self, data: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.smbus_write_byte(0x40, data)?;
        Ok(())
    }

    pub fn setup(&self) -> Result<(), Box<dyn Error>> {
        self.send_command(0x38)?;
        self.send_command(0x39)?;
        self.send_command(0x14)?;
        self.send_command(0x70)?;
        self.send_command(0x56)?;
        self.send_command(0x6c)?;
        sleep(Duration::from_millis(200));
        self.send_command(0x38)?;
        self.send_command(0x0c)?;
        self.send_command(0x01)?;
        sleep(Duration::from_millis(2));
        Ok(())
    }
}
