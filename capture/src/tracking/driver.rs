use std::{cmp::Ordering, thread::sleep};

use rppal::gpio::{Gpio, OutputPin, Level};

pub struct Driver {
    pos: isize,

    current_direction: Direction,

    enn_pin: OutputPin,
    step_pin: OutputPin,
    dir_pin: OutputPin,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Track,
    Home,
}

impl Driver {
    pub fn new() -> Result<Driver, rppal::gpio::Error> {
        let pos = 0;
        let current_direction = Direction::Track;

        let gpio = Gpio::new()?;

        let mut enn_pin = gpio.get(crate::CONFIG.tracking.enn_pin)?.into_output();
        enn_pin.write(Level::High);

        let mut step_pin = gpio.get(crate::CONFIG.tracking.step_pin)?.into_output();
        step_pin.write(Level::Low);

        let mut dir_pin = gpio.get(crate::CONFIG.tracking.dir_pin)?.into_output();
        dir_pin.write(dir_to_level(current_direction));

        Ok(Driver { pos, current_direction, enn_pin, step_pin, dir_pin, })
    }

    pub fn enable(&mut self) {
        self.enn_pin.write(Level::Low);
    }

    pub fn pos(&self) -> isize {
        self.pos
    }

    pub fn step(&mut self, direction: Direction) {
        if self.current_direction != direction {
            self.dir_pin.write(dir_to_level(direction));
            self.current_direction = direction;
        }
        let half_cycle = crate::CONFIG.tracking.cycle / 2;
        self.step_pin.write(Level::High);
        self.pos += match direction {
            Direction::Track => 1,
            Direction::Home => -1,
        };
        sleep(half_cycle);
        self.step_pin.write(Level::Low);
        sleep(half_cycle);
    }

    pub fn goto(&mut self, pos: isize) {
        let direction = match self.pos.cmp(&pos) {
            Ordering::Less => Direction::Track,
            Ordering::Equal => { return },
            Ordering::Greater => Direction::Home,
        };
        for _ in 0..self.pos.abs_diff(pos) {
            self.step(direction);
        }
        assert_eq!(self.pos, pos);
    }
}

fn dir_to_level(direction: Direction) -> Level {
    match direction {
        Direction::Track => {
            if crate::CONFIG.tracking.tracking_direction == 0 { Level::Low }
            else if crate::CONFIG.tracking.tracking_direction == 1 { Level::High }
            else { unreachable!() }
        },
        Direction::Home => {
            if crate::CONFIG.tracking.tracking_direction == 0 { Level::High }
            else if crate::CONFIG.tracking.tracking_direction == 1 { Level::Low }
            else { unreachable!() }
        },
    }
}