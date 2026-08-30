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

//! # SO1602A Driver for Raspberry Pi

use tokio::time::{Duration, sleep};

use embedded_hal::i2c::{I2c, SevenBitAddress};

/// SO1602A I2C Address 1
pub const SO1602A_ADDR: SevenBitAddress = 0x3c;
/// SO1602A I2C Address 2
pub const SO1602A_ADDR2: SevenBitAddress = 0x3d;

/// SO1602A start of 1st Line Address
pub const SO1602A_1ST_LINE: u8 = 0x80;
/// SO1602A start of 2nd Line Address
pub const SO1602A_2ND_LINE: u8 = 0xA0;

/// SO1602A Command
pub const SO1602A_COMMAND: u8 = 0x00;
/// SO1602A Data
pub const SO1602A_DATA: u8 = 0x40;

/// Clear Display Command
pub const SO1602A_BASIC_CLEARDISPLAY: u8 = 0x01;
/// Home Position Command
pub const SO1602A_BASIC_HOMEPOSITION: u8 = 0x02;

/// Display Control Command
pub const SO1602A_DISPLAYCONTROL: u8 = 0x08;
/// Display ON in Display Control
pub const SO1602A_DISPLAYCONTROL_DISPLAY_ON: u8 = 0x04;
/// Cursor ON in Display Control
pub const SO1602A_DISPLAYCONTROL_CURSOR_ON: u8 = 0x02;
/// Blink ON in Display Control
pub const SO1602A_DISPLAYCONTROL_BLINK_ON: u8 = 0x01;

/// Function Set Command
pub const SO1602A_FUNCTIONSET: u8 = 0x20;
/// Function Set 2 or 4 Line in Function Set
pub const SO1602A_FUNCTIONSET_2OR4LINE: u8 = 0x08;
/// Function Set Double Height in Function Set
pub const SO1602A_FUNCTIONSET_DOUBLEHEIGHT: u8 = 0x04;
/// Function Set IS flug in Function Set
pub const SO1602A_FUNCTIONSET_IS: u8 = 0x01;
/// Function Set RE flug in Function Set
pub const SO1602A_FUNCTIONSET_RE: u8 = 0x02;
/// Function Set Blink Enable in Function Set when RE=1
pub const SO1602A_FUNCTIONSET_RE_BLINKENABLE: u8 = 0x04;
/// Function Set Reverse in Function Set when RE=1
pub const SO1602A_FUNCTIONSET_RE_REVERSE: u8 = 0x01;

/// SD flag ON Command
pub const SO1602A_OLED_ON: u8 = 0x79;
/// SD flag OFF Command
pub const SO1602A_OLED_OFF: u8 = 0x78;
/// OLED Contrast Command
pub const SO1602A_OLED_CONSTRAST: u8 = 0x81;

/// SO1602A Driver
pub struct SO1602A<I2C> {
    i2c: I2C,
    addr: SevenBitAddress,
}

impl<I2C: I2c> SO1602A<I2C> {
    /// Create a new SO1602A instance
    /// # Arguments
    /// * `i2c` - I2C bus handle the display is reached through
    /// * `addr` - I2C Address
    /// # Returns
    /// * SO1602A instance
    pub fn new(i2c: I2C, addr: SevenBitAddress) -> SO1602A<I2C> {
        SO1602A { i2c, addr }
    }

    /// Send Command
    /// # Arguments
    /// * `data` - Command
    /// # Returns
    /// * Result<(), I2C::Error>
    pub fn send_command(&mut self, data: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.addr, &[SO1602A_COMMAND, data])?;
        Ok(())
    }

    /// Send Data
    /// # Arguments
    /// * `data` - Data
    /// # Returns
    /// * Result<(), I2C::Error>
    pub fn send_data(&mut self, data: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.addr, &[SO1602A_DATA, data])?;
        Ok(())
    }

    /// Wait
    /// # Arguments
    /// * `ms` - Wait time in milliseconds
    async fn wait(&self, ms: u64) {
        sleep(Duration::from_millis(ms)).await;
    }

    /// Send OLED Command
    /// # Arguments
    /// * `d1` - Command 1
    /// * `d2` - Command 2
    /// # Returns
    /// * Result<(), I2C::Error>
    pub fn send_oled_command(&mut self, d1: u8, d2: u8) -> Result<(), I2C::Error> {
        // Extended register mode (RE=1)
        self.send_command(
            SO1602A_FUNCTIONSET | SO1602A_FUNCTIONSET_2OR4LINE | SO1602A_FUNCTIONSET_RE,
        )?;
        // OLED Command Set (SD=1)
        self.send_command(SO1602A_OLED_ON)?;

        // Send OLED Command
        self.send_command(d1)?;
        self.send_command(d2)?;

        // Reset to OLED Command Set (SD=0)
        self.send_command(SO1602A_OLED_OFF)?;
        // Reset to Extended Command Set (RE=0)
        self.send_command(SO1602A_FUNCTIONSET | SO1602A_FUNCTIONSET_2OR4LINE)?;

        Ok(())
    }

    /// Setup SO1602A Device
    /// # Returns
    /// * Result<(), I2C::Error>
    pub async fn setup(&mut self) -> Result<(), I2C::Error> {
        self.send_setup_commands()?;

        // wait
        self.wait(20).await;

        Ok(())
    }

    fn send_setup_commands(&mut self) -> Result<(), I2C::Error> {
        // Contrast Setting
        self.send_oled_command(SO1602A_OLED_CONSTRAST, 0x7F)?;
        // Display ON, Cursor OFF, Blink OFF
        self.send_command(SO1602A_DISPLAYCONTROL | SO1602A_DISPLAYCONTROL_DISPLAY_ON)?;
        // Clear Display
        self.send_command(SO1602A_BASIC_CLEARDISPLAY)?;
        // Position to Home
        self.send_command(SO1602A_BASIC_HOMEPOSITION)?;

        Ok(())
    }

    /// Register Custom Character
    /// # Arguments
    /// * `index` - Character Index
    /// * `data` - Character Data
    /// # Returns
    /// * Result<(), I2C::Error>
    pub fn register_char(&mut self, index: u8, data: [u8; 8]) -> Result<(), I2C::Error> {
        self.send_command(0x40 | (index << 3))?;
        for d in data {
            self.send_data(d)?;
        }
        Ok(())
    }

    /// Put a character at the specified position
    /// # Arguments
    /// * `position` - Position
    /// * `data` - Character
    /// # Returns
    /// * Result<(), I2C::Error>
    pub fn put_u8(&mut self, position: u8, data: u8) -> Result<(), I2C::Error> {
        self.send_command(position)?;
        self.send_data(data)?;
        Ok(())
    }

    /// Print a string at the specified line
    /// # Arguments
    /// * `line` - Line
    /// * `s` - String
    /// # Returns
    /// * Result<(), I2C::Error>
    pub fn put_str(&mut self, line_addr: u8, s: &str) -> Result<(), I2C::Error> {
        self.send_command(line_addr)?;
        for c in s.as_bytes() {
            self.send_data(*c)?;
        }
        Ok(())
    }

    /// Clear Display and Home Position
    /// # Returns
    /// * Result<(), I2C::Error>
    pub fn clear_home(&mut self) -> Result<(), I2C::Error> {
        self.send_command(0x01)?;
        self.send_command(0x02)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTransaction};

    /// A command frame (control byte first) addressed to the display.
    fn command(data: u8) -> I2cTransaction {
        I2cTransaction::write(SO1602A_ADDR, vec![SO1602A_COMMAND, data])
    }

    /// A data frame (control byte first) addressed to the display.
    fn data(byte: u8) -> I2cTransaction {
        I2cTransaction::write(SO1602A_ADDR, vec![SO1602A_DATA, byte])
    }

    /// A driver over a bus expecting exactly `expectations`, plus a handle to
    /// assert with once the test has run.
    fn driver(expectations: &[I2cTransaction]) -> (SO1602A<I2cMock>, I2cMock) {
        let i2c = I2cMock::new(expectations);
        (SO1602A::new(i2c.clone(), SO1602A_ADDR), i2c)
    }

    #[test]
    fn test_send_oled_command_sequence() {
        let (mut display, mut i2c) = driver(&[
            command(0x2A),
            command(0x79),
            command(0x81),
            command(0x7F),
            command(0x78),
            command(0x28),
        ]);

        display
            .send_oled_command(SO1602A_OLED_CONSTRAST, 0x7F)
            .unwrap();

        i2c.done();
    }

    #[test]
    fn test_setup_command_sequence() {
        let (mut display, mut i2c) = driver(&[
            command(0x2A),
            command(0x79),
            command(0x81),
            command(0x7F),
            command(0x78),
            command(0x28),
            command(0x0C),
            command(0x01),
            command(0x02),
        ]);

        display.send_setup_commands().unwrap();

        i2c.done();
    }

    #[test]
    fn test_register_char_sequence() {
        let character = [0x00, 0x10, 0x08, 0x04, 0x02, 0x01, 0x00, 0x00];
        let (mut display, mut i2c) = driver(&[
            command(0x50),
            data(0x00),
            data(0x10),
            data(0x08),
            data(0x04),
            data(0x02),
            data(0x01),
            data(0x00),
            data(0x00),
        ]);

        display.register_char(2, character).unwrap();

        i2c.done();
    }

    #[test]
    fn test_put_str_sequence() {
        let (mut display, mut i2c) = driver(&[command(SO1602A_2ND_LINE), data(b'A'), data(0x01)]);

        display.put_str(SO1602A_2ND_LINE, "A\x01").unwrap();

        i2c.done();
    }

    #[test]
    fn test_put_u8_sequence() {
        let (mut display, mut i2c) = driver(&[command(SO1602A_1ST_LINE + 3), data(b'X')]);

        display.put_u8(SO1602A_1ST_LINE + 3, b'X').unwrap();

        i2c.done();
    }

    #[test]
    fn test_clear_home_sequence() {
        let (mut display, mut i2c) = driver(&[
            command(SO1602A_BASIC_CLEARDISPLAY),
            command(SO1602A_BASIC_HOMEPOSITION),
        ]);

        display.clear_home().unwrap();

        i2c.done();
    }
}
