use std::io;

use crate::commands::{Request, ShutdownOption, ShutdownReboot};
use crate::i2c::I2cBus;

pub struct CXN0102 {
    pub i2c_address: u16,
    pub i2c_path: &'static str,
    pub gpio_chip_path: &'static str,
    pub gpio_line_offset: u32,
}

impl Default for CXN0102 {
    fn default() -> Self {
        Self {
            i2c_address: 0x77,
            i2c_path: "/dev/i2c-7",
            gpio_chip_path: "/dev/gpiochip0",
            gpio_line_offset: 9,
        }
    }
}

impl CXN0102 {
    pub fn shutdown(&self) -> io::Result<()> {
        self.write(
            &ShutdownReboot {
                option: ShutdownOption::StopsAllFunctions,
            }
            .to_bytes(),
        )
    }

    pub fn write(&self, data: &[u8]) -> io::Result<()> {
        let mut bus = I2cBus::open(self.i2c_path)?;
        bus.write(self.i2c_address, data)
    }

    #[allow(dead_code)]
    pub fn read(&self, data: &mut [u8]) -> io::Result<()> {
        let mut bus = I2cBus::open(self.i2c_path)?;
        bus.read(self.i2c_address, data)
    }
}
