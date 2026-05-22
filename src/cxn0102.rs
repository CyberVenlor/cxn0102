use std::io;

use crate::commands::{CXN0102Notify, Request, ShutdownOption, ShutdownReboot};
use crate::i2c::I2cBus;

const NOTIFY_READ_SIZE: usize = 32;

#[derive(Debug, Clone, Copy)]
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
            gpio_line_offset: 144,
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

    pub fn read_notify(&self) -> io::Result<CXN0102Notify> {
        let mut data = [0xff; NOTIFY_READ_SIZE];
        self.read(&mut data)?;

        let payload_size = *data.get(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "notify data is missing OP0")
        })? as usize;
        let notify_size = payload_size.checked_add(2).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "notify payload size overflow")
        })?;

        if notify_size > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "notify payload size {payload_size} exceeds {} byte notify read",
                    data.len()
                ),
            ));
        }

        CXN0102Notify::from_bytes(&data[..notify_size]).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse notify data: {error:?}"),
            )
        })
    }
}
