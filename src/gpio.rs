use std::io;

use gpiod::{Bias, Chip, Edge, EdgeDetect, Input, Lines, Options};

pub struct GpioController {
    lines: Lines<Input>,
}

impl GpioController {
    pub fn open_rising_edge(chip: &str, line_offset: u32) -> io::Result<Self> {
        let chip = Chip::new(chip)?;
        let options = Options::input([line_offset])
            .bias(Bias::PullUp)
            .edge(EdgeDetect::Rising)
            .consumer("cxn0102");
        let lines = chip.request_lines(options)?;

        Ok(Self { lines })
    }

    pub fn read(&self) -> io::Result<bool> {
        let values = self.lines.get_values([false])?;
        Ok(values[0])
    }

    pub fn wait_rising_edge(&mut self) -> io::Result<()> {
        loop {
            let event = self.lines.read_event()?;
            if event.edge == Edge::Rising {
                return Ok(());
            }
        }
    }
}
