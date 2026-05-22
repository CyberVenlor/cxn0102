pub mod commands;
pub mod cxn0102;
pub mod gpio;
pub mod i2c;

use std::io;
use std::thread;
use std::time::Duration;

use cxn0102::CXN0102;
use gpio::GpioController;

const GPIO_POLL_INTERVAL: Duration = Duration::from_millis(50);

fn main() -> io::Result<()> {
    let cxn0102 = CXN0102::default();
    let gpio =
        GpioController::open_input_pull_down(cxn0102.gpio_chip_path, cxn0102.gpio_line_offset)?;

    println!(
        "GPIO09 level test: {} line {} with input pull-down",
        cxn0102.gpio_chip_path, cxn0102.gpio_line_offset
    );

    let mut was_high = gpio.read()?;
    println!("GPIO09 initial level: {}", level_name(was_high));

    loop {
        thread::sleep(GPIO_POLL_INTERVAL);

        let is_high = gpio.read()?;
        if is_high != was_high {
            println!(
                "GPIO09 level changed: {} -> {}",
                level_name(was_high),
                level_name(is_high)
            );
            was_high = is_high;
        }
    }
}

fn level_name(high: bool) -> &'static str {
    if high {
        "high"
    } else {
        "low"
    }
}
