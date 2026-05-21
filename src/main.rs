pub mod cxn0102;
pub mod gpio;
pub mod i2c;
pub mod commands;

use std::io::{self, Write};

use cxn0102::CXN0102;

use crate::commands::{Request, SetBrightness, StartInput};

fn main() -> io::Result<()> {
    let cxn0102 = CXN0102::default();
    let request = StartInput {}.to_bytes();
    cxn0102.write(&request)?;

    let stdin = io::stdin();

    loop {
        print!("brightness> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.read_line(&mut input)? == 0 {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let brightness = match input.parse::<i8>() {
            Ok(brightness) => brightness,
            Err(_) => {
                eprintln!("brightness must be a number from {} to {}", i8::MIN, i8::MAX);
                continue;
            }
        };

        let request = SetBrightness { brightness }.to_bytes();
        cxn0102.write(&request)?;
    }

    Ok(())
}
