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

use tokio::time::{sleep, Duration};

use rppal::i2c;

/// SO1602A I2C Address 1
pub const SO1602A_ADDR: u16 = 0x3c;
/// SO1602A I2C Address 2
pub const SO1602A_ADDR2: u16 = 0x3d;

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
pub struct SO1602A {
    i2c: i2c::I2c,
}

impl SO1602A {
    /// Create a new SO1602A instance
    /// # Arguments
    /// * `addr` - I2C Address
    /// # Returns
    /// * SO1602A instance
    pub fn new(addr: u16) -> Result<SO1602A, i2c::Error> {
        let mut i2c = i2c::I2c::new()?;
        i2c.set_slave_address(addr)?;
        Ok(SO1602A { i2c })
    }

    /// Send Command
    /// # Arguments
    /// * `data` - Command
    /// # Returns
    /// * Result<(), i2c::Error>
    pub fn send_command(&self, data: u8) -> Result<(), i2c::Error> {
        self.i2c.smbus_write_byte(SO1602A_COMMAND, data)?;
        Ok(())
    }

    /// Send Data
    /// # Arguments
    /// * `data` - Data
    /// # Returns
    /// * Result<(), i2c::Error>
    pub fn send_data(&self, data: u8) -> Result<(), i2c::Error> {
        self.i2c.smbus_write_byte(SO1602A_DATA, data)?;
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
    /// * Result<(), i2c::Error>
    pub fn send_oled_command(&self, d1: u8, d2: u8) -> Result<(), i2c::Error> {
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
    /// * Result<(), i2c::Error>
    pub async fn setup(&self) -> Result<(), i2c::Error> {
        // Contrast Setting
        self.send_oled_command(SO1602A_OLED_CONSTRAST, 0x7F)?;
        // Display ON, Cursor OFF, Blink OFF
        self.send_command(SO1602A_DISPLAYCONTROL | SO1602A_DISPLAYCONTROL_DISPLAY_ON)?;
        // Clear Display
        self.send_command(SO1602A_BASIC_CLEARDISPLAY)?;
        // Position to Home
        self.send_command(SO1602A_BASIC_HOMEPOSITION)?;

        // wait
        self.wait(20).await;

        Ok(())
    }

    /// Register Custom Character
    /// # Arguments
    /// * `index` - Character Index
    /// * `data` - Character Data
    /// # Returns
    /// * Result<(), i2c::Error>
    pub fn register_char(&self, index: u8, data: [u8; 8]) -> Result<(), i2c::Error> {
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
    /// * Result<(), i2c::Error>
    pub fn put_u8(&self, position: u8, data: u8) -> Result<(), i2c::Error> {
        self.send_command(position)?;
        self.send_data(data)?;
        Ok(())
    }

    /// Print a string at the specified line
    /// # Arguments
    /// * `line` - Line
    /// * `s` - String
    /// # Returns
    /// * Result<(), i2c::Error>
    pub fn put_str(&self, line_addr: u8, s: &str) -> Result<(), i2c::Error> {
        self.send_command(line_addr)?;
        for c in s.as_bytes() {
            self.send_data(*c)?;
        }
        Ok(())
    }

    /// Clear Display and Home Position
    /// # Returns
    /// * Result<(), i2c::Error>
    pub fn clear_home(&self) -> Result<(), i2c::Error> {
        self.send_command(0x01)?;
        self.send_command(0x02)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(SO1602A_ADDR, 0x3c);
        assert_eq!(SO1602A_ADDR2, 0x3d);
        assert_eq!(SO1602A_1ST_LINE, 0x80);
        assert_eq!(SO1602A_2ND_LINE, 0xA0);
        assert_eq!(SO1602A_COMMAND, 0x00);
        assert_eq!(SO1602A_DATA, 0x40);
    }

    #[test]
    fn test_display_control_flags() {
        assert_eq!(SO1602A_DISPLAYCONTROL, 0x08);
        assert_eq!(SO1602A_DISPLAYCONTROL_DISPLAY_ON, 0x04);
        assert_eq!(SO1602A_DISPLAYCONTROL_CURSOR_ON, 0x02);
        assert_eq!(SO1602A_DISPLAYCONTROL_BLINK_ON, 0x01);
    }

    #[test]
    fn test_function_set_flags() {
        assert_eq!(SO1602A_FUNCTIONSET, 0x20);
        assert_eq!(SO1602A_FUNCTIONSET_2OR4LINE, 0x08);
        assert_eq!(SO1602A_FUNCTIONSET_DOUBLEHEIGHT, 0x04);
        assert_eq!(SO1602A_FUNCTIONSET_IS, 0x01);
        assert_eq!(SO1602A_FUNCTIONSET_RE, 0x02);
        assert_eq!(SO1602A_FUNCTIONSET_RE_BLINKENABLE, 0x04);
        assert_eq!(SO1602A_FUNCTIONSET_RE_REVERSE, 0x01);
    }

    #[test]
    fn test_oled_commands() {
        assert_eq!(SO1602A_OLED_ON, 0x79);
        assert_eq!(SO1602A_OLED_OFF, 0x78);
        assert_eq!(SO1602A_OLED_CONSTRAST, 0x81);
    }

    #[test]
    fn test_basic_commands() {
        assert_eq!(SO1602A_BASIC_CLEARDISPLAY, 0x01);
        assert_eq!(SO1602A_BASIC_HOMEPOSITION, 0x02);
    }

    #[test]
    fn test_character_data_format() {
        let char_data: [u8; 8] = [
            0b00000,
            0b10000,
            0b01000,
            0b00100,
            0b00010,
            0b00001,
            0b00000,
            0b00000,
        ];
        
        assert_eq!(char_data.len(), 8);
        assert!(char_data.iter().all(|&b| b <= 0b11111));
    }

    #[test]
    fn test_line_addresses() {
        assert!(SO1602A_1ST_LINE < SO1602A_2ND_LINE);
        assert_eq!(SO1602A_2ND_LINE - SO1602A_1ST_LINE, 0x20);
    }

    #[test]
    fn test_command_data_difference() {
        assert_eq!(SO1602A_DATA - SO1602A_COMMAND, 0x40);
    }

    #[test]
    fn test_display_control_combinations() {
        let display_on = SO1602A_DISPLAYCONTROL | SO1602A_DISPLAYCONTROL_DISPLAY_ON;
        let display_cursor_on = display_on | SO1602A_DISPLAYCONTROL_CURSOR_ON;
        let all_on = display_cursor_on | SO1602A_DISPLAYCONTROL_BLINK_ON;
        
        assert_eq!(display_on, 0x0C);
        assert_eq!(display_cursor_on, 0x0E);
        assert_eq!(all_on, 0x0F);
    }

    #[test]
    fn test_function_set_combinations() {
        let basic_config = SO1602A_FUNCTIONSET | SO1602A_FUNCTIONSET_2OR4LINE;
        let extended_config = basic_config | SO1602A_FUNCTIONSET_RE;
        let instruction_set_config = basic_config | SO1602A_FUNCTIONSET_IS;
        
        assert_eq!(basic_config, 0x28);
        assert_eq!(extended_config, 0x2A);
        assert_eq!(instruction_set_config, 0x29);
    }

    #[test]
    fn test_character_index_bounds() {
        let max_custom_chars = 8;
        for i in 0..max_custom_chars {
            let cgram_addr = 0x40 | (i << 3);
            assert!(cgram_addr >= 0x40 && cgram_addr < 0x80);
        }
    }
}
