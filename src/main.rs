pub mod commands;
pub mod cxn0102;
pub mod gpio;
pub mod i2c;

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cxn0102::CXN0102;

use crate::commands::{Request, SetBrightness, StartInput};
use crate::gpio::GpioController;

const GPIO_POLL_INTERVAL: Duration = Duration::from_millis(5);

fn main() -> io::Result<()> {
    let cxn0102 = Arc::new(Mutex::new(CXN0102::default()));

    spawn_notify_thread(Arc::clone(&cxn0102))?;

    let request = StartInput {}.to_bytes();
    cxn0102
        .lock()
        .map_err(|_| io::Error::other("CXN0102 mutex poisoned"))?
        .write(&request)?;

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
                eprintln!(
                    "brightness must be a number from {} to {}",
                    i8::MIN,
                    i8::MAX
                );
                continue;
            }
        };

        let request = SetBrightness { brightness }.to_bytes();
        cxn0102
            .lock()
            .map_err(|_| io::Error::other("CXN0102 mutex poisoned"))?
            .write(&request)?;
    }

    Ok(())
}

fn spawn_notify_thread(cxn0102: Arc<Mutex<CXN0102>>) -> io::Result<thread::JoinHandle<()>> {
    let gpio = {
        let cxn0102 = cxn0102
            .lock()
            .map_err(|_| io::Error::other("CXN0102 mutex poisoned"))?;
        GpioController::open_input(cxn0102.gpio_chip_path, cxn0102.gpio_line_offset)?
    };

    Ok(thread::spawn(move || {
        let mut was_high = match gpio.read() {
            Ok(high) => high,
            Err(error) => {
                eprintln!("failed to read initial COM_REQ GPIO level: {error}");
                false
            }
        };

        loop {
            thread::sleep(GPIO_POLL_INTERVAL);

            let is_high = match gpio.read() {
                Ok(high) => high,
                Err(error) => {
                    eprintln!("failed to read COM_REQ GPIO level: {error}");
                    continue;
                }
            };

            if is_high && !was_high {
                match cxn0102.lock() {
                    Ok(cxn0102) => match cxn0102.read_notify() {
                        Ok(notify) => println!("notify: {notify:?}"),
                        Err(error) => eprintln!("failed to read notify: {error}"),
                    },
                    Err(_) => {
                        eprintln!("failed to read notify: CXN0102 mutex poisoned");
                        break;
                    }
                }
            }

            was_high = is_high;
        }
    }))
}
