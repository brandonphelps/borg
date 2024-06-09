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

    pub fn is_high(&mut self) -> bool {
        self.voltage_level == self.high
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
    address: u16,
    // read/write bit
    read_bit: bool,
    data_frame1: u8,
    // ack/nack bit
    // data frame 1
    data_frame2: u8,
    // ack/nack bit
    // data frame 2
    // ack/nack bit
    // stop condition.
}

impl Message {
    fn to_bits(&self) -> Vec<u8> {
        let a = self.address.to_be_bytes();
        vec![a[0], a[1]]
    }
}

pub struct BitArray {
    buffer: Vec<u8>,
    cursor: u8,
}

impl BitArray {
    fn new(value: &[u8]) -> Self {
        Self {
            buffer: value.to_vec(),
            cursor: 0,
        }
    }

    fn get_current_bit(&self) -> bool {
        let bit_cursor = self.cursor % 8;
        let byte_cursor = self.cursor / 8;
        (self.buffer[byte_cursor as usize] & (0x1 << bit_cursor)) == 1 << bit_cursor
    }

    // returns true if cursor is done.
    fn inc_cursor(&mut self) -> bool {
        self.cursor += 1;
        (self.cursor / 8) as usize >= self.buffer.len()
    }

    fn reset_cursor(&mut self) {
        self.cursor = 0;
    }
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

    data: BitArray,
}

impl Master {
    fn new(clock_rate: u32) -> Self {
        let msg = Message {
            address: 0b0101101,
            read_bit: false,
            data_frame1: 0x23,
            data_frame2: 0x30,
        };

        let data_bits = msg.to_bits();
        Self {
            time_past: 0,
            clock_rate,
            sda: Line::new(0.0, 12.0),
            scl: Line::new(0.0, 12.0),
            data: BitArray::new(&data_bits),
        }
    }

    // dt: how much time has passed since last call.
    fn update(&mut self, dt: Duration) {
        self.time_past += dt.as_millis();
        if self.time_past >= self.clock_rate as u128 {
            self.time_past -= self.clock_rate as u128;
            self.scl.flip();
            if !self.scl.is_high() {
                if self.data.inc_cursor() {
                    self.data.reset_cursor();
                }
            }
        }
        // the note is due to how the drawing of the graph works currently.
        if self.scl.is_high() {
            if self.data.get_current_bit() {
                if !self.sda.is_high() {
                    self.sda.flip()
                }
            } else {
                if self.sda.is_high() {
                    self.sda.flip()
                }
            }
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
    pub fn new(sample_rate: Duration, buffer_size: usize) -> Self {
        Self {
            line_values: VecDeque::<f32>::default(),
            sample_timer: Timer::new(sample_rate.as_millis()),
            last_value: 0.0,
            buffer_size,
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
    scope_sda: Scope,
}

impl ProtocolState {
    pub fn default() -> Self {
        Self {
            master: Master::new(10),
            scope_scl: Scope::new(Duration::from_millis(5), 200),
            scope_sda: Scope::new(Duration::from_millis(5), 200),
        }
    }

    pub fn update(&mut self, dt: Duration, input: Input) {
        self.master.update(dt);
        self.scope_scl.update(dt, self.master.scl.voltage_level);
        self.scope_sda.update(dt, self.master.sda.voltage_level);

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
        let base_y = 60.0;
        // each millisecond is represented by "10.0" units.
        // smallest unit of time.
        let y_delta = -5.0;
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

        let base_y = 160.0;

        // sda scope
        let points: Vec<[f32; 2]> = self
            .scope_sda
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

    #[test]
    fn test_bit_array() {
        let mut bit_array = BitArray::new(&[0xAF, 0xDF]);
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), false);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), false);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);

        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
        bit_array.inc_cursor();
        assert_eq!(bit_array.get_current_bit(), true);
    }
}
