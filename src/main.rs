pub mod cli;
pub mod commands;
pub mod cxn0102;
pub mod gpio;
pub mod i2c;

use std::io::{self, BufRead};
use std::thread;
use std::time::Duration;

use cli::parse_request_line;
use cxn0102::CXN0102;
use gpio::GpioController;

const GPIO_POLL_INTERVAL: Duration = Duration::from_millis(50);

fn main() -> io::Result<()> {
    let cxn0102 = CXN0102::default();
    let notify_cxn0102 = cxn0102;

    let notify_thread = thread::spawn(move || {
        if let Err(error) = run_notification_loop(notify_cxn0102) {
            eprintln!("notification loop stopped: {error}");
        }
    });

    println!("Enter 'help' for command examples. Press Ctrl-D to exit stdin loop.");
    run_stdin_loop(cxn0102)?;

    let _ = notify_thread.join();
    Ok(())
}

fn run_stdin_loop(cxn0102: CXN0102) -> io::Result<()> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let Some(bytes) = (match parse_request_line(&line) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("command error: {error}");
                continue;
            }
        }) else {
            continue;
        };

        cxn0102.write(&bytes)?;
        println!("sent: {}", format_hex(&bytes));
    }

    Ok(())
}

fn run_notification_loop(cxn0102: CXN0102) -> io::Result<()> {
    let gpio =
        GpioController::open_input_pull_down(cxn0102.gpio_chip_path, cxn0102.gpio_line_offset)?;

    println!(
        "GPIO notification input: {} line {} with input pull-down",
        cxn0102.gpio_chip_path, cxn0102.gpio_line_offset
    );

    let mut was_high = gpio.read()?;
    println!("GPIO initial level: {}", level_name(was_high));

    loop {
        thread::sleep(GPIO_POLL_INTERVAL);

        let is_high = gpio.read()?;
        if !was_high && is_high {
            match cxn0102.read_notify() {
                Ok(notify) => println!("notify: {notify:?}"),
                Err(error) => eprintln!("notify read error: {error}"),
            }
        }
        was_high = is_high;
    }
}

fn level_name(high: bool) -> &'static str {
    if high { "high" } else { "low" }
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
