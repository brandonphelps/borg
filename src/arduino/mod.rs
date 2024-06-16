use std::usize;

pub type Result<T> = core::result::Result<T, Error>;

mod xmd_serial;

#[derive(Debug)]
pub enum Error {
    FlashOutOfBounds(u32),
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

struct FlashBlock<const N: usize> {
    address: u32,
    data: [u8; N],
}

impl<const N: usize> FlashBlock<N> {
    fn new(address: u32) -> Self {
        Self {
            // how to make address const like N? 
            address,
            data: [0; N]
        }
    }

    fn write(&mut self, address: u32, data: &[u8]) -> Result<()> {
        println!("Flash write: {:x} {:x?}", address, data);
        if address < self.address || address as usize + data.len() > self.address as usize + self.data.len() {
            return Err(Error::FlashOutOfBounds(address))
        }
        for (addr, d) in data.iter().enumerate() {
            let w_addr = address - self.address + addr as u32;
            self.data[w_addr as usize] = *d;
        }
        Ok(())
    }

    fn read(&mut self, address: u32, length: usize) -> Result<Vec<u8>> {
        if address < self.address || address as usize + length > self.address as usize + length {
            return Err(Error::FlashOutOfBounds(address))
        }
        let mut data = Vec::new();

        for d in 0..length {
            let r_addr = address - self.address + d as u32;
            data.push(self.data[r_addr as usize]);
        }

        Ok(data)
    }
}

// arduino side bootloader mock implementation.
pub struct Bootloader<T> {
    comm_inter: T,
    ptr: u32,
    ptr_data: u32,
    command: u8,
    current_number: u32,
    terminal_mode: bool,
    data: [u8; 200],
    version_str: &'static str,
    attempt: u32,

    flash_block1: FlashBlock<0x300>,
}

impl<T> Bootloader<T>
where
    T: std::io::Read + std::io::Write,
{
    pub fn new(comm_inter: T) -> Self {
        let data = [0; 200];
        let version_str = "v2.0 [Arduino:XYZ] Apr 19 2019 14:38:48";

        Self {
            attempt: 0,
            comm_inter,
            ptr: 0,
            ptr_data: 0,
            command: 0,
            current_number: 0,
            terminal_mode: false,
            // flash/ram emulation.
            data,
            version_str,
            flash_block1: FlashBlock::new(0x20004000),
        }
    }

    pub fn update_loop(&mut self) -> Result<()> {
        // read from serial chunk.
        let mut data_chunk = [0xff; 64];
        // this is mocking out the serial id. 
        let tmp = 0x10010005_u32.to_le_bytes(); //self.attempt.to_be_bytes();
        self.data[4] = tmp[0];
        self.data[5] = tmp[1];
        self.data[6] = tmp[2];
        self.data[7] = tmp[3];
        println!("Attempt: {:?}", self.attempt);
        let length = self.comm_inter.read(&mut data_chunk)?;
        println!("Data chunk: {:x?}", data_chunk);
        let mut index = 0;
        let mut j: u8 = 0;
        // data_chunk[..length as usize].iter().enumerate()
        while index < length {
            println!("{:x}:{}:{}", index, data_chunk[index] as char, index.to_string());
            if data_chunk[index] == 0xff {
                continue;
            }
            if data_chunk[index] == b'#' {
                println!("Process {} current numb {:x} length {} index {}", self.command as char, self.current_number, length, index);
                if self.command == b'S' {
                    if length > index {
                        index += 1;
                        index += 1;

                        let u32tmp = if (length - index) < self.current_number as usize {
                            length - index
                        } else {
                            self.current_number as usize
                        };

                        println!("index_data: {:x} u32 tmp: {}", self.ptr_data, u32tmp);
                        if self.ptr_data >= self.flash_block1.address &&
                            (self.ptr_data as usize + u32tmp as usize) < self.flash_block1.address as usize + self.flash_block1.data.len() 
                        {
                            println!("Write to flash block 1: {:x?}", &data_chunk[index..index+u32tmp]);
                            self.flash_block1.write(self.ptr_data, &data_chunk[index..index+u32tmp])?;
                        }
                        index += u32tmp;
                        j = u32tmp as u8;
                    }
                    index -= 1;

                    println!("J: {}, CN: {}", j, self.current_number);
                    if (j as u32) < self.current_number {
                        println!("Reading data: {}", self.current_number - j as u32);

                        let mut s = xmd_serial::XmdSerial::new();
                        println!("Getting xmd data");
                        let data = s.serial_getdata_xmd(&mut self.comm_inter, self.current_number - j as u32)?;
                        
                        println!("Data received: {:x}, {:x?}", self.ptr_data, data);
                        self.flash_block1.write(self.ptr_data, &data)?;
                    }
                }
                else if self.command == b'N' {
                    if self.terminal_mode {
                        self.comm_inter.write_all(b"\n\r")?;
                    }
                    self.terminal_mode = false;
                } else if self.command == b'w' {
                    let d= &self.data
                        [self.current_number as usize..(self.current_number + 4) as usize];
                    self.current_number = self.ptr_data;
                    self.comm_inter.write_all(d)?;
                } else if self.command == b'V' {
                    // note the 'v' is important. 
                    self.comm_inter.write_all(&self.version_str.as_bytes())?;
                    self.comm_inter.write_all(b"\n\r")?;
                    self.attempt += 1;
                }
                else {
                    if self.command == 0 {
                        
                    } else {
                        todo!()
                    }
                }
            } else {
                if (b'0' <= data_chunk[index]) && (data_chunk[index] <= b'9') {
                    self.current_number = self.current_number << 4 | (data_chunk[index] - b'0') as u32;
                } else if (b'A' <= data_chunk[index]) && (data_chunk[index] <= b'F') {
                    self.current_number = self.current_number << 4 | (data_chunk[index] - b'A' + 0xa) as u32;
                } else if (b'a' <= data_chunk[index]) && (data_chunk[index] <= b'f') {
                    self.current_number = self.current_number << 4 | (data_chunk[index] - b'a' + 0xa) as u32;
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
        let mut flash_b = FlashBlock::<10>::new(0x100);
        assert!(flash_b.write(0, &[1,2,3]).is_err());
        assert!(flash_b.write(20, &[1,2,3]).is_err());
        assert!(flash_b.write(90, &[1,2,3]).is_err());
        assert!(flash_b.write(0x90, &[1,2,3]).is_err());
        assert!(flash_b.write(0x99, &[1,2,3]).is_err());
        assert!(flash_b.write(0x10B, &[1,2,3]).is_err());

        assert!(flash_b.write(0x100, &[1,2,3]).is_ok());

        assert!(flash_b.read(0, 3).is_err());
        assert!(flash_b.read(90, 3).is_err());
        assert_eq!(flash_b.read(0x100, 3).unwrap(), vec![1,2,3]);

    }

}
