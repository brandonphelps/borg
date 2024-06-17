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
mod xmd_serial;

#[derive(Debug)]
pub enum Error {
    FlashOutOfBounds(u32),
    Io(std::io::Error),

    FlashOverLap,

    XModem(xmd_serial::Error),
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
        flash.add_block(0x0, 0x300).unwrap();
        flash.add_block(0xe000ed00, 0x300).unwrap();
        flash.add_block(0x400e0740, 0x300).unwrap();
        flash.add_block(0x41004020, 0x300).unwrap();
        flash.add_block(0x41004018, 0x300).unwrap();
        flash.add_block(0x40000834, 0x300).unwrap();
        // mock out the chip id.
        flash.write(0x4, &0x10010005_u32.to_le_bytes()).unwrap();
        flash
            .write(0xe000ed00, &0x10010005_u32.to_le_bytes())
            .unwrap();
        flash
            .write(0x400e0740, &0x10010005_u32.to_le_bytes())
            .unwrap();
        flash.add_block(0x20004000, 0x300).unwrap();

        Self {
            attempt: 0,
            comm_inter,
            ptr_data: 0,
            command: 0,
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
        let length = self.comm_inter.read(&mut data_chunk)?;
        println!("Data chunk: {:x?}", data_chunk);
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

                        println!("index_data: {:x} u32 tmp: {}", self.ptr_data, u32tmp);
                        self.flash
                            .write(self.ptr_data, &data_chunk[index..index + u32tmp])?;
                        index += u32tmp;
                        j = u32tmp as u8;
                    }
                    index -= 1;

                    println!("J: {}, CN: {}", j, self.current_number);
                    if (j as u32) < self.current_number {
                        println!("Reading data: {}", self.current_number - j as u32);

                        let mut s = xmd_serial::XmdSerial::new();
                        println!("Getting xmd data");
                        let data = s.serial_getdata_xmd(
                            &mut self.comm_inter,
                            self.current_number - j as u32,
                        )?;

                        println!("Data received: {:x}, {:x?}", self.ptr_data, data);
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn flash_read_write_block() {
        let mut flash_b = FlashBlock::new(0x100, 10);
        assert!(flash_b.write(0, &[1, 2, 3]).is_err());
        assert!(flash_b.write(20, &[1, 2, 3]).is_err());
        assert!(flash_b.write(90, &[1, 2, 3]).is_err());
        assert!(flash_b.write(0x90, &[1, 2, 3]).is_err());
        assert!(flash_b.write(0x99, &[1, 2, 3]).is_err());
        assert!(flash_b.write(0x10B, &[1, 2, 3]).is_err());

        assert!(flash_b.write(0x100, &[1, 2, 3]).is_ok());

        assert!(flash_b.read(0, 3).is_err());
        assert!(flash_b.read(90, 3).is_err());
        assert_eq!(flash_b.read(0x100, 3).unwrap(), vec![1, 2, 3]);
    }
}
