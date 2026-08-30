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

pub use bme280;
pub use so1602a;

// Re-exported so downstream crates can name the bus trait and the concrete
// Linux implementation without depending on them directly.
pub use embedded_hal;
pub use linux_embedded_hal::I2cdev;
pub use linux_embedded_hal::i2cdev::linux::LinuxI2CError;

/// I2C bus bound to physical pins 3 (SDA) and 5 (SCL) on the 40-pin header.
///
/// Open one [`I2cdev`] per device rather than sharing a single handle:
/// `I2cdev` keeps the slave address on the open file and reopens the device
/// file whenever that address changes, so a shared handle would pay an
/// open/ioctl/close on every switch between devices. The kernel serialises the
/// transfers on the bus itself, so separate handles do not race.
pub const DEFAULT_I2C_BUS: &str = "/dev/i2c-1";
