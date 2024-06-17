use factorio::{FactorioState, Input};
use ggez::event;
use ggez::glam::*;
use ggez::graphics::{self, Color};
use ggez::{Context, GameResult};
use i2c::ProtocolState;
use serialport::available_ports;
use serialport::SerialPort;
use serialport::SerialPortType;
use std::io;

use std::time::Duration;
use std::time::Instant;

mod arduino;
mod factorio;
mod i2c;

struct MainState {
    pos_x: f32,
    previous: Instant,
    factorio: FactorioState,
    input: Input,
    i2c_protocol: ProtocolState,
}

impl MainState {
    fn new() -> GameResult<MainState> {
        let s = MainState {
            pos_x: 0.0,
            previous: Instant::now(),
            factorio: FactorioState::new(),
            input: Input::default(),
            i2c_protocol: ProtocolState::default(),
        };
        Ok(s)
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        self.pos_x = self.pos_x % 800.0 + 1.0;
        self.previous = Instant::now();

        self.factorio.update(self.input);
        self.i2c_protocol
            .update(Duration::from_millis(1), self.input);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.1, 0.2, 0.3, 1.0]));

        let circle = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            Vec2::new(0.0, 0.0),
            100.0,
            2.0,
            Color::WHITE,
        )?;
        canvas.draw(
            &circle,
            Vec2::new(
                self.factorio.player.position.0,
                self.factorio.player.position.1,
            ),
        );
        self.i2c_protocol.draw(ctx, &mut canvas)?;
        canvas.finish(ctx)?;
        Ok(())
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> Result<(), ggez::GameError> {
        println!("Key down: {:?}", input);

        match input.keycode.unwrap() {
            ggez::winit::event::VirtualKeyCode::Up => {
                self.input.up_pressed = true;
            }
            ggez::winit::event::VirtualKeyCode::Down => {
                self.input.down_pressed = true;
            }
            ggez::winit::event::VirtualKeyCode::Left => {
                self.input.left_pressed = true;
            }
            ggez::winit::event::VirtualKeyCode::Right => {
                self.input.right_pressed = true;
            }
            other_input => {
                println!("unhandled key: {:?}", other_input);
            }
        }

        Ok(())
    }

    fn key_up_event(
        &mut self,
        _ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
    ) -> Result<(), ggez::GameError> {
        println!("Key up: {:?}", input);
        match input.keycode.unwrap() {
            ggez::winit::event::VirtualKeyCode::Up => {
                self.input.up_pressed = false;
            }
            ggez::winit::event::VirtualKeyCode::Down => {
                self.input.down_pressed = false;
            }
            ggez::winit::event::VirtualKeyCode::Left => {
                self.input.left_pressed = false;
            }
            ggez::winit::event::VirtualKeyCode::Right => {
                self.input.right_pressed = false;
            }
            _ => {
                println!("unhandled input");
            }
        }
        Ok(())
    }
}

pub fn get_port() -> Option<String> {
    match available_ports() {
        Ok(ports) => {
            match ports.len() {
                0 => println!("No ports found."),
                1 => println!("Found 1 port:"),
                n => println!("Found {} ports:", n),
            }
            for p in ports {
                println!("   {}", p.port_name);
                match p.port_type {
                    SerialPortType::UsbPort(info) => {
                        println!(" Type::USB");
                        println!(" VID: 0x{:04x} PID: 0x{:04x}", info.vid, info.pid);
                        if info.vid == 0x2341 {
                            println!("Found arduino port thingy");
                            return Some(p.port_name);
                        }
                        println!(
                            "     Serial Number: {}",
                            info.serial_number.as_ref().map_or("", String::as_str)
                        );
                        println!(
                            "      Manufacturer: {}",
                            info.manufacturer.as_ref().map_or("", String::as_str)
                        );
                        println!(
                            "           Product: {}",
                            info.product.as_ref().map_or("", String::as_str)
                        );
                        #[cfg(feature = "usbportinfo-interface")]
                        println!(
                            "         Interface: {}",
                            info.interface
                                .as_ref()
                                .map_or("".to_string(), |x| format!("{:02x}", *x))
                        );
                    }
                    SerialPortType::PciPort => todo!(),
                    SerialPortType::BluetoothPort => todo!(),
                    SerialPortType::Unknown => todo!(),
                }
            }
        }
        Err(_) => todo!(),
    }
    None
}

/// expects port to have a timeout.
fn dump_memory(mut port: Box<dyn SerialPort>, mut start_addr: u32, length: usize) {
    let mut buffer = [0; 1];
    let mut byte_read_count = 0;

    while byte_read_count < length {
        let addr = start_addr.to_be_bytes();
        println!("Reading data from: {:x?}", addr);
        port.write_all(&addr).expect("Failed to write address");
        port.write_all(b",").expect("Failed to write numeric eol");
        port.write_all(b"o#")
            .expect("Failed to send command to arduino");
        match port.read(&mut buffer) {
            Ok(bytes) => {
                if bytes == 1 {
                    println!("{:#02x}: {:#02x}", start_addr, buffer[0]);
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => break,
            Err(e) => eprintln!("{:?}", e),
        }
        byte_read_count += 1;

        start_addr += 1;
    }
}

pub fn main() -> GameResult {
    env_logger::init();
    println!("Ready");
    //let port_name = get_port().expect("Failed to find port");
    let port_name = "/dev/pts/5";
    let mut port = serialport::new(port_name, 9600)
        .open()
        .expect("Failed to open serial port");
    port.set_timeout(Duration::from_millis(40000))
        .expect("Failed to set timeout");
    port.clear(serialport::ClearBuffer::Input)
        .expect("failed to clear buffer");
    // not sure if needed.
    //port.clear(ClearBuffer::Output).expect("Failed to clear output buffer");

    let mut bootloader = arduino::Bootloader::new(port);
    loop {
        bootloader.update_loop().expect("failed bootloader loop");
    }

    return Ok(());
    let mut buffer: [u8; 1] = [0; 1];
    println!("Attempting to read out version information");
    port.write_all(&[b'V', b'#'])
        .expect("failed to write some bytes");
    let mut reading_version = false;
    loop {
        match port.read(&mut buffer) {
            Ok(bytes) => {
                if bytes == 1 {
                    println!("Received: {:?}", buffer[0] as char);
                    reading_version = true;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                if reading_version {
                    break;
                }
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
    println!("Attempting to read some bytes");
    dump_memory(port, 0x2000, 0x100);
    return Ok(());

    let cb = ggez::ContextBuilder::new("super_simple", "ggez")
        .window_mode(ggez::conf::WindowMode::default().dimensions(2000., 600.));
    let (ctx, event_loop) = cb.build()?;
    let state = MainState::new()?;
    event::run(ctx, event_loop, state)
}
