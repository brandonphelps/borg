mod utils;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("communication error: {0}")]
    CommErr(#[from] std::io::Error),
}

/// Arduino flashing utility.
/// specifically aimed at the arduino nano io 33.

/// device specific information.
trait Device {}

pub struct Flasher<C> {
    comm: C,
}

impl<C> Flasher<C>
where
    C: std::io::Write + std::io::Read,
{
    pub fn new(comm: C) -> Self {
        Self { comm }
    }
}

/// low level protocol handler for
/// communicating to arduino bootloader.
pub struct ArduinoBootComm<C> {
    comm: C,
}

impl<C> ArduinoBootComm<C>
where
    C: std::io::Write + std::io::Read,
{
    pub fn new(comm: C) -> Self {
        Self { comm }
    }

    /// read address of memory and place it into vector.
    pub fn read_memory(&mut self, address: u32, size: u32) -> Result<Vec<u8>> {
        self.comm.write_all(format!("{:x},", address).as_bytes())?;
        self.comm.write_all(b"w#")?;
        let mut bytes = [0; 4];
        // self.comm.read_exact(&mut bytes)?;
        Ok(bytes.to_vec())
    }
}

// bi directional channel.

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use crate::arduino::Bootloader;

    use super::*;

    use super::utils::BiChannel;

    #[test]
    fn test_read() {
        let channel = BiChannel::new();
        let mut arduio_com = ArduinoBootComm::new(channel.clone());
        let mut bootloader = Bootloader::new(channel);
        //channel.write_all(&[1, 2, 3, 4, 5]).unwrap();
        
        let res = arduio_com.read_memory(0, 4).unwrap();
        bootloader.update_loop().unwrap();
        // assert_eq!(res.len(), 4);


        // let expected_write = b"0,w#";
        // let mut read_bytes = vec![];
        // channel.read_to_end(&mut read_bytes).unwrap();
        // assert_eq!(&read_bytes[..4], expected_write);
    }
}
