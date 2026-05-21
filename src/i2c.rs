use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;

const I2C_SLAVE: libc::c_ulong = 0x0703;

pub struct I2cBus {
    device: File,
    path: String,
}

impl I2cBus {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let device = OpenOptions::new().read(true).write(true).open(path)?;

        Ok(Self {
            device,
            path: path.display().to_string(),
        })
    }

    pub fn write(&mut self, address: u16, data: &[u8]) -> io::Result<()> {
        self.select_slave(address)?;
        self.device.write_all(data).map_err(|error| {
            with_context(
                error,
                format!(
                    "failed to write [{}] to I2C address 0x{address:02x} on {}",
                    format_hex(data),
                    self.path
                ),
            )
        })
    }

    pub fn read(&mut self, address: u16, data: &mut [u8]) -> io::Result<()> {
        self.select_slave(address)?;
        self.device.read_exact(data).map_err(|error| {
            with_context(
                error,
                format!(
                    "failed to read {} byte(s) from I2C address 0x{address:02x} on {}",
                    data.len(),
                    self.path
                ),
            )
        })
    }

    fn select_slave(&self, address: u16) -> io::Result<()> {
        if address > 0x7f {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only 7-bit I2C addresses are supported",
            ));
        }

        let result =
            unsafe { libc::ioctl(self.device.as_raw_fd(), I2C_SLAVE, address as libc::c_ulong) };
        if result < 0 {
            return Err(with_context(
                io::Error::last_os_error(),
                format!(
                    "failed to select I2C address 0x{address:02x} on {}",
                    self.path
                ),
            ));
        }

        Ok(())
    }
}

fn with_context(error: io::Error, context: String) -> io::Error {
    io::Error::new(error.kind(), format!("{context}: {error}"))
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
