use std::usize;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
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
}

impl<T> Bootloader<T>
where
    T: std::io::Read + std::io::Write,
{
    pub fn new(comm_inter: T) -> Self {

        let data = [0; 200];
        
        Self {
            comm_inter,
            ptr: 0,
            ptr_data: 0,
            command: 0,
            current_number: 0,
            terminal_mode: false,
            // flash/ram emulation.
            data,
        }
    }

    pub fn update_loop(&mut self) -> Result<()> {
        // read from serial chunk.
        let mut data_chunk = [0xff; 64];
        let length = self.comm_inter.read(&mut data_chunk)?;
        for ptr in data_chunk[..length as usize].iter() {
            println!("\t{}", *ptr as char);
            if *ptr == 0xff {
                continue;
            }
            if *ptr == b'#' {
                if self.command == b'N' {
                    if self.terminal_mode {
                        self.comm_inter.write_all(b"\n\r")?;
                    }
                    self.terminal_mode = false;
                } else if self.command == b'w' {
                    println!("Current number: {}", self.current_number);
                    let d= &self.data
                        [self.current_number as usize..(self.current_number + 4) as usize];
                    println!("{:x?}", d);
                    self.current_number = self.ptr_data;
                    self.comm_inter.write_all(d)?;
                } else if self.command == b'V' {
                    // note the 'v' is important. 
                    println!("Sending version info");
                    self.comm_inter.write_all("v2.0 [Arduino:XYZ] Apr 19 2019 14:38:48".as_bytes())?;
                }
            } else {
                println!("Processing {}", *ptr as char);
                if (b'0' <= *ptr) && (*ptr <= b'9') {
                    self.current_number = self.current_number << 4 | (*ptr - b'0') as u32;
                } else if (b'A' <= *ptr) && (*ptr <= b'F') {
                    self.current_number = self.current_number << 4 | (*ptr - b'A' + 0xa) as u32;
                } else if (b'a' <= *ptr) && (*ptr <= b'f') {
                    self.current_number = self.current_number << 4 | (*ptr - b'a' + 0xa) as u32;
                } else if *ptr == b',' {
                    self.ptr_data = self.current_number;
                    self.current_number = 0;
                } else {
                    self.command = *ptr;
                    self.current_number = 0;
                }
            }
        }
        Ok(())
    }
}
