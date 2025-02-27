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

#[allow(dead_code)]
use std::thread::sleep;
use std::time::Duration;

use rppal::i2c;

pub const SO1602A_ADDR: u16 = 0x3c;
pub const SO1602A_ADDR2: u16 = 0x3d;
pub const SO1602A_1ST_LINE: u8 = 0x80;
pub const SO1602A_2ND_LINE: u8 = 0xA0;

pub struct SO1602A {
    i2c: i2c::I2c,
}

impl SO1602A {
    pub fn new(addr: u16) -> Result<SO1602A, i2c::Error> {
        let mut i2c = i2c::I2c::new()?;
        i2c.set_slave_address(addr)?;
        Ok(SO1602A { i2c })
    }

    pub fn send_command(&self, data: u8) -> Result<(), i2c::Error> {
        self.i2c.smbus_write_byte(0, data)?;
        Ok(())
    }

    pub fn send_data(&self, data: u8) -> Result<(), i2c::Error> {
        self.i2c.smbus_write_byte(0x40, data)?;
        Ok(())
    }

    fn wait(&self, ms: u64) {
        sleep(Duration::from_millis(ms));
    }

    pub fn setup(&self) -> Result<(), i2c::Error> {
        // Set Basic Mode
        // self.send_command(0x28)?;
        // Scroll lock
        // self.send_command(0x10)?;
        // Display OFF, Cursor OFF, Blink OFF
        // self.send_command(0x40)?;

        // Extended register mode (RE=1)
        self.send_command(0x2a)?;
        // OLED Command Set (SD=1)
        self.send_command(0x79)?;
        // Change Contrast
        self.send_command(0x81)?;
        self.send_command(0xFF)?;
        // Reset to OLED Command Set (SD=0)
        self.send_command(0x78)?;
        // Reset to Extended Command Set (RE=0)
        self.send_command(0x28)?;

        // wait
        self.wait(200);

        // Display ON, Cursor OFF, Blink OFF
        self.send_command(0x0c)?;
        // Clear Display
        self.send_command(0x01)?;

        // wait
        self.wait(20);

        Ok(())
    }

    pub fn register_char(&self, index: u8, data: [u8; 8]) -> Result<(), i2c::Error> {
        self.send_command(0x40 | (index << 3))?;
        for d in data {
            self.send_data(d)?;
        }
        Ok(())
    }

    pub fn put_u8(&self, position: u8, data: u8) -> Result<(), i2c::Error> {
        self.send_command(position)?;
        self.send_data(data)?;
        Ok(())
    }

    pub fn print(&self, line: u8, s: &str) -> Result<(), i2c::Error> {
        self.send_command(line)?;
        for c in s.as_bytes() {
            self.send_data(*c)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn clear_home(&self) -> Result<(), i2c::Error> {
        self.send_command(0x01)?;
        self.send_command(0x02)?;
        Ok(())
    }
}
