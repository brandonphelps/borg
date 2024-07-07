use std::time::Duration;

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
        self.comm.read_exact(&mut bytes)?;
        Ok(bytes.to_vec())
    }
}

// bi directional channel.

#[cfg(test)]
mod tests {

    use crate::arduino::Bootloader;

    use super::*;

    use super::utils::BiChannel;

    #[test]
    fn test_read() {
        let channel = BiChannel::new();
        let mut channel_clone = channel.clone();
        channel_clone.set_timeout(Duration::from_secs(2));
        
        let mut bootloader = Bootloader::new(channel);
        

        let k = std::thread::spawn(|| { 
            let mut arduio_com = ArduinoBootComm::new(channel_clone);
            println!("before read");
            let res = arduio_com.read_memory(0, 4).unwrap();
            println!("faile");
            assert_eq!(res.len(), 4);
            assert_eq!(&res, &[1,2,3,4]);
            println!("res: {:x?}", res);
        });
        let j = std::thread::spawn(move || {
            for _ in 0..100 {
                bootloader.update_loop().unwrap();
            }
        });
        k.join().unwrap();
        j.join().unwrap();
    }
}
