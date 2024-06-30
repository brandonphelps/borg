/// Arduino related components
/// Currently investigating how the arduino bootloader
/// and flashing tools work.

/// Currently main is setup to act a arduino bootloader samd21
/// device and with the following command it will respond as such.
/// `home/brandon/snap/arduino-cli/48/.arduino15/packages/arduino/tools/bossac/1.7.0-arduino3/bossac -i -p /dev/pts/6 --debug`
/// the end goal currently is to implement my own flashing utility for
/// flash the arduino samd21 bootloader and then build a second stage bootloader
/// to write down the knowledge I have over elf file formats.
/// use the following command to create a virtual serial device
/// `socat -d -d pty,rawer,echo=0 pty,rawer,echo=0`

pub type Result<T> = core::result::Result<T, Error>;

mod flash;
mod flash_utility;
mod xmd_serial;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Flash out of bounds: {0:x}: {1:x}")]
    FlashOutOfBounds(u32, u32),
    #[error("Io error: {0}")]
    Io(std::io::Error),

    #[error("Flash overlap")]
    FlashOverLap,

    #[error("Xmodem communication error: {0}")]
    XModem(xmd_serial::Error),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<xmd_serial::Error> for Error {
    fn from(value: xmd_serial::Error) -> Self {
        Self::XModem(value)
    }
}

// arduino side bootloader mock implementation.
pub struct Bootloader<T> {
    comm_inter: T,
    ptr_data: u32,
    command: u8,
    current_number: u32,
    src_buff_addr: u32,
    terminal_mode: bool,
    version_str: &'static str,
    attempt: u32,

    flash: flash::Flash,
}

impl<T> Bootloader<T>
where
    T: std::io::Read + std::io::Write,
{
    pub fn new(comm_inter: T) -> Self {
        let version_str = "v2.0 [Arduino:XYZ] Apr 19 2019 14:38:48";
        let mut flash = flash::Flash::default();
        // reverse pulled addresses
        // todo: look at the samd21g memory space for legit ranges.
        flash.add_block(0x0, 0x300).unwrap();
        flash.add_block(0x2000, 0x20000).unwrap();
        flash.add_block(0xe000ed00, 0x300).unwrap();
        flash.add_block(0x400e0740, 0x300).unwrap();
        flash.add_block(0x41004020, 0x300).unwrap();
        flash.add_block(0x40000834, 0x300).unwrap();
        // mock out the chip id.
        flash.write(0x4, &0x10010005_u32.to_le_bytes()).unwrap();
        flash
            .write(0xe000ed00, &0x10010005_u32.to_le_bytes())
            .unwrap();
        flash
            .write(0x400e0740, &0x10010005_u32.to_le_bytes())
            .unwrap();
        flash.add_block(0x20004000, 0x2000).unwrap();
        Self {
            attempt: 0,
            comm_inter,
            ptr_data: 0,
            command: 0,
            src_buff_addr: 0,
            current_number: 0,
            terminal_mode: false,
            version_str,
            flash,
        }
    }

    pub fn update_loop(&mut self) -> Result<()> {
        // read from serial chunk.
        let mut data_chunk = [0xff; 64];
        println!("Attempt: {:?}", self.attempt);
        let length = match self.comm_inter.read(&mut data_chunk) {
            Ok(r) => r,
            Err(f) => {
                // todo: filter by timeout error.
                return Ok(());
            }
        };
        // .inspect_err(|f| println!("comm iter read error: {f}"))?;
        let k: Vec<char> = data_chunk.iter().map(|f| *f as char).collect();
        println!("Data chunk: {:x?}", k);
        let mut index = 0;
        let mut j: u8 = 0;
        while index < length {
            println!(
                "Index: {}  Command: {}(0x{:02x}) Ptr data: {:x}",
                index, self.command as char, self.command, self.ptr_data
            );
            if data_chunk[index] == 0xff {
                continue;
            }
            if data_chunk[index] == b'#' {
                println!(
                    "Process {} current numb {:x} length {} index {}",
                    self.command as char, self.current_number, length, index
                );
                if self.command == b'S' {
                    if length > index {
                        index += 1;

                        let u32tmp = if (length - index) < self.current_number as usize {
                            length - index
                        } else {
                            self.current_number as usize
                        };

                        self.flash
                            .write(self.ptr_data, &data_chunk[index..index + u32tmp])?;
                        index += u32tmp;
                        j = u32tmp as u8;
                    }
                    index -= 1;

                    if (j as u32) < self.current_number {
                        let mut s = xmd_serial::XmdSerial::new();
                        let data = s.serial_getdata_xmd(
                            &mut self.comm_inter,
                            self.current_number - j as u32,
                        )?;
                        self.flash.write(self.ptr_data, &data)?;
                    }
                } else if self.command == b'W' {
                    self.ptr_data = self.current_number;
                } else if self.command == b'N' {
                    if self.terminal_mode {
                        self.comm_inter.write_all(b"\n\r")?;
                    }
                    self.terminal_mode = false;
                } else if self.command == b'w' {
                    self.current_number = self.ptr_data;
                    let d = self.flash.read(self.current_number, 4)?;
                    self.comm_inter.write_all(&d)?;
                } else if self.command == b'V' {
                    // note the 'v' is important.
                    self.comm_inter.write_all(self.version_str.as_bytes())?;
                    self.comm_inter.write_all(b"\n\r")?;
                    self.attempt += 1;
                } else if self.command == b'X' {
                    self.erase_flash(self.current_number);
                    // oddly enough the bossa continue even if
                    // we don't send a response.
                    self.comm_inter.write_all(b"X\n\r")?;
                } else if self.command == b'Y' {
                    if self.current_number == 0 {
                        println!("Setting src buffer addr: {:x}", self.ptr_data);
                        self.src_buff_addr = self.ptr_data;
                    } else {
                        // todo: why divide by page size?
                        let size = self.current_number / 4;
                        let data = self
                            .flash
                            .read(self.src_buff_addr, size)
                            .inspect_err(|f| println!("flash read error: {f}"))?;
                        let dst_addr = self.ptr_data;
                        println!(
                            "Updating flash with sram {:x}({}) to {:x}",
                            self.src_buff_addr, size, dst_addr
                        );
                        self.flash
                            .write(dst_addr, &data)
                            .inspect_err(|f| println!("flash write error: {f}"))?;
                    }
                    println!("Send response to w/e");
                    self.comm_inter
                        .write_all(b"Y\n\r")
                        .inspect_err(|f| println!("got error: {f}"))?;
                    println!("finished sending response");
                } else {
                    if self.command == 0 || self.command == 0x80 {
                    } else {
                        todo!()
                    }
                }
            } else {
                if (b'0' <= data_chunk[index]) && (data_chunk[index] <= b'9') {
                    self.current_number =
                        self.current_number << 4 | (data_chunk[index] - b'0') as u32;
                } else if (b'A' <= data_chunk[index]) && (data_chunk[index] <= b'F') {
                    self.current_number =
                        self.current_number << 4 | (data_chunk[index] - b'A' + 0xa) as u32;
                } else if (b'a' <= data_chunk[index]) && (data_chunk[index] <= b'f') {
                    self.current_number =
                        self.current_number << 4 | (data_chunk[index] - b'a' + 0xa) as u32;
                } else if data_chunk[index] == b',' {
                    // ptr data is like index, in that
                    // it points to an address.
                    // however it does not point to data_chunk, but
                    // rather a ram/flash address of sorts.
                    self.ptr_data = self.current_number;
                    self.current_number = 0;
                } else {
                    self.command = data_chunk[index];
                    self.current_number = 0;
                }
            }
            index += 1;
        }
        Ok(())
    }

    fn erase_flash(&mut self, dst_addr: u32) {
        println!("Erase flash: {:x}", dst_addr);
        // todo
    }
}

#[cfg(test)]
mod test {
    // tests use dummy ttys

    use std::time::Duration;

    use super::Bootloader;

    #[test]
    fn write_buffer() {
        let port_name = "/dev/pts/5";
        let mut port = serialport::new(port_name, 9600)
            .open()
            .expect("Failed to open serial port");
        port.set_timeout(Duration::from_millis(40000))
            .expect("Failed to set timeout");
        port.clear(serialport::ClearBuffer::Input)
            .expect("failed to clear buffer");

        let mut r_port = serialport::new("/dev/pts/6", 9600)
            .open()
            .expect("Failed to open serial port");
        port.set_timeout(Duration::from_millis(40000))
            .expect("Failed to set timeout");
        port.clear(serialport::ClearBuffer::Input)
            .expect("failed to clear buffer");

        let j = std::thread::spawn(|| {
            let mut bootloader = Bootloader::new(r_port);
            loop {
                bootloader.update_loop().expect("failed bootloader loop");
            }
        });

        port.write_all(b"Y20005000,0#").unwrap();
        let mut buf = [0; 3];
        println!("Waiting for data from module");
        port.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], b'Y');
        println!("Transfer data to address 2000 of size 20");
        port.write_all(b"Y2000,20#").unwrap();
        let mut buf = [0; 3];
        port.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], b'Y');

        j.join();
        assert!(false);
    }
}
