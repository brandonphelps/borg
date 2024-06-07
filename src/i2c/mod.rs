use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use ggez::{
    glam::Vec2,
    graphics::{self, Canvas, Color},
    Context, GameResult,
};

use crate::factorio::Input;

pub struct Timer {
    time_stamp: u128,
    trigger_rate: u128,
    triggered: bool,
}

impl Timer {
    pub fn new(trigger_rate: u128) -> Self {
        Self {
            time_stamp: 0,
            trigger_rate,
            triggered: false,
        }
    }

    fn update(&mut self, dt: Duration) {
        self.time_stamp += dt.as_millis();
        if self.time_stamp >= self.trigger_rate {
            self.time_stamp -= self.trigger_rate;
            if self.triggered {
                println!("Missed a trigger");
            }
            self.triggered = true;
        }
    }

    fn trged(&mut self) -> bool {
        let b = self.triggered;
        if self.triggered {
            self.triggered = false;
        }
        return b;
    }
}

pub struct Clock {
    time_stamp: Instant,
}

impl Clock {
    pub fn update(&mut self, dt: Duration) {
        self.time_stamp += dt;
    }
}

pub struct Line {
    //
    high: f32,
    low: f32,

    voltage_level: f32,
}

impl Line {
    pub fn new(low: f32, high: f32) -> Self {
        Self {
            low,
            high,
            voltage_level: 0.0,
        }
    }

    pub fn set_voltage(&mut self, volts: f32) {
        self.voltage_level = volts;
    }

    pub fn flip(&mut self) {
        if self.voltage_level == self.high {
            self.voltage_level = self.low;
        } else {
            self.voltage_level = self.high;
        }
    }
}

struct Frame {}

pub struct Message {
    // start condition
    // address frame
    // read/write bit
    // ack/nack bit
    // data frame 1
    // ack/nack bit
    // data frame 2
    // ack/nack bit
    // stop condition.
}

pub struct Master {
    // track of how much time has occured
    //milliseconds
    time_past: u128,

    clock_rate: u32, // milliseconds.

    // serial data
    sda: Line,

    // serial clock
    scl: Line,
}

impl Master {
    fn new(clock_rate: u32) -> Self {
        Self {
            time_past: 0,
            clock_rate,
            sda: Line::new(0.0, 12.0),
            scl: Line::new(0.0, 12.0),
        }
    }

    // dt: how much time has passed since last call.
    fn update(&mut self, dt: Duration) {
        self.time_past += dt.as_millis();
        if self.time_past >= self.clock_rate as u128 {
            self.time_past -= self.clock_rate as u128;
            self.scl.flip();
        }
    }
}

pub struct Slave {
    sda: Line,
    scl: Line,
}

pub struct I2cBus {}

/// Container for a renderedable view
/// of values over some line.
pub struct Scope {
    line_values: VecDeque<f32>,
    sample_timer: Timer,
    buffer_size: usize,
    last_value: f32,
}

impl Scope {
    pub fn new(sample_rate: Duration) -> Self {
        Self {
            line_values: VecDeque::<f32>::default(),
            sample_timer: Timer::new(sample_rate.as_millis()),
            last_value: 0.0,
            buffer_size: 100,
        }
    }

    pub fn update(&mut self, dt: Duration, value: f32) {
        self.sample_timer.update(dt);
        if self.sample_timer.trged() {
            self.last_value = value;
        }
        self.push_value(self.last_value);
    }

    fn push_value(&mut self, value: f32) {
        while self.line_values.len() > self.buffer_size {
            self.line_values.pop_front();
        }
        self.line_values.push_back(value);
    }
}

pub struct ProtocolState {
    master: Master,
    scope_scl: Scope,
}

impl ProtocolState {
    pub fn default() -> Self {
        Self {
            master: Master::new(20),
            scope_scl: Scope::new(Duration::from_millis(10)),
        }
    }

    pub fn update(&mut self, dt: Duration, input: Input) {
        self.master.update(dt);
        self.scope_scl.update(dt, self.master.scl.voltage_level);

        if input.up_pressed {
            self.scope_scl.sample_timer.trigger_rate += 1;
            println!("Trigger rate: {}", self.scope_scl.sample_timer.trigger_rate);
        } else if input.down_pressed {
            if self.scope_scl.sample_timer.trigger_rate - 1 == 0 {
                
            } else { 
                self.scope_scl.sample_timer.trigger_rate -= 1;
            }
            println!("Trigger rate: {}", self.scope_scl.sample_timer.trigger_rate);
        }
        if input.left_pressed {
            if self.scope_scl.buffer_size - 1 != 0 {
                self.scope_scl.buffer_size -= 1;
                println!("buffer size: {}", self.scope_scl.buffer_size);
            } 
        } else if input.right_pressed {
            self.scope_scl.buffer_size += 1;
            println!("buffer size: {}", self.scope_scl.buffer_size);
        }
    }

    // if using ggez renderer.
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // where to render
        let base_x = 100.0;
        let base_y = 40.0;
        // each millisecond is represented by "10.0" units.
        // smallest unit of time.
        let y_delta = 5.0;
        // time_interval_delta_per_millis
        let x_delta = 10.0;

        // clock line.
        let points: Vec<[f32; 2]> = self
            .scope_scl
            .line_values
            .iter()
            .enumerate()
            .map(|(index, y)| [(index as f32 * x_delta), y * y_delta])
            .collect();
        if points.len() > 2 {
            let line = graphics::Mesh::new_line(ctx, &points, 5.0, Color::WHITE)?;
            canvas.draw(&line, Vec2::new(base_x, base_y));
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_clock() {
        let mut m = Master::new(10);

        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 0.0);
        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 12.0);

        m.update(Duration::from_millis(1));
        assert_eq!(m.scl.voltage_level, 12.0);
    }
}
