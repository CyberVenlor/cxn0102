pub mod cli;
pub mod commands;
pub mod cxn0102;
pub mod gpio;
pub mod i2c;

use std::io::{self, BufRead};
use std::thread;

use cli::parse_request_line;
use cxn0102::CXN0102;
use gpio::GpioController;

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
    let mut gpio = GpioController::open_rising_edge(cxn0102.gpio_chip, cxn0102.gpio_line_offset)?;

    loop {
        gpio.wait_rising_edge()?;

        match cxn0102.read_notify() {
            Ok(notify) => println!("notify: {notify:?}"),
            Err(error) => eprintln!("notify read error: {error}"),
        }
    }
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
