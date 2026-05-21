pub mod cxn0102;
pub mod gpio;
pub mod i2c;
pub mod commands;

use std::io;

use cxn0102::CXN0102;

fn main() -> io::Result<()> {
    CXN0102::default().shutdown()
}
