use std::collections::HashMap;

/// this is named flash but its more just like block memory stuff.
use super::{Error, Result};

pub struct FlashBlock {
    address: u32,
    size: u32,
    data: Vec<u8>,
}

impl FlashBlock {
    pub fn new(address: u32, size: u32) -> Self {
        Self {
            // how to make address const like N?
            address,
            size,
            data: vec![0xFF; size as usize],
        }
    }

    pub fn write(&mut self, address: u32, data: &[u8]) -> Result<()> {
        println!("Flash write: {:x} {:x?}", address, data);
        if address < self.address || address + data.len() as u32 > self.address + self.size {
            return Err(Error::FlashOutOfBounds(address, data.len() as u32));
        }
        for (addr, d) in data.iter().enumerate() {
            let w_addr = address - self.address + addr as u32;
            self.data[w_addr as usize] = *d;
        }
        Ok(())
    }

    pub fn read(&mut self, address: u32, length: u32) -> Result<Vec<u8>> {
        if address < self.address || address + length > self.address + self.size {
            println!("Out of bounds: {:x} {}", address, length);
            return Err(Error::FlashOutOfBounds(address, length));
        }
        let mut data = Vec::new();

        for d in 0..length {
            let r_addr = address - self.address + d;
            data.push(self.data[r_addr as usize]);
        }

        Ok(data)
    }

    pub fn erase(&mut self, address: u32, length: u32) -> Result<()> {
        if address < self.address || address + length > self.address + self.size {
            return Err(Error::FlashOutOfBounds(address, length));
        }

        for addr in 0..length {
            let w_addr = address - self.address + addr as u32;
            self.data[w_addr as usize] = 0xFF;
        }

        Ok(())
    }
}

// mock ram/ flash
#[derive(Default)]
pub struct Flash {
    flash_blocks: HashMap<u32, FlashBlock>,
}

impl Flash {
    /// will error if the address and data is invalid for the set of
    /// flash blocks available.
    pub fn write(&mut self, address: u32, data: &[u8]) -> Result<()> {
        println!("Writing data to {:x} of {}", address, data.len());
        for (key, block) in self.flash_blocks.iter_mut() {
            if address >= *key && (address + data.len() as u32) <= (key + block.size) {
                return block.write(address, data);
            }
            println!("Checking {:x} {}", key, block.size);
        }
        Err(Error::FlashOutOfBounds(address, data.len() as u32))
    }

    pub fn read(&mut self, address: u32, length: u32) -> Result<Vec<u8>> {
        println!("Reading {} bytes at {:x}", length, address);
        for (key, block) in self.flash_blocks.iter_mut() {
            println!("Checking the following blocks: {key:x} {}", block.size);
            if address >= *key && (address + length) <= key + block.size {
                // println!("Reading from block: {:x}", key);
                return block.read(address, length);
            }
        }
        Err(Error::FlashOutOfBounds(address, length))
    }

    pub fn erase(&mut self, address: u32, length: u32) -> Result<()> {
        for (key, block) in self.flash_blocks.iter_mut() {
            if address >= *key && (address + length) <= key + block.size {
                // println!("Reading from block: {:x}", key);
                return block.erase(address, length);
            }
        }
        Err(Error::FlashOutOfBounds(address, length))
    }

    pub fn add_block(&mut self, start_address: u32, size: u32) -> Result<()> {
        for (block_start_address, block) in self.flash_blocks.iter_mut() {
            println!("Checking {block_start_address} on {0}", block.size);
            println!(
                "{} >= {} && {} <= {}",
                start_address,
                *block_start_address,
                start_address,
                *block_start_address + block.size
            );
            if start_address >= *block_start_address
                && start_address < *block_start_address + block.size
            {
                return Err(Error::FlashOverLap);
            }
            println!(
                "{} >= {} && {} <= {}",
                start_address + size,
                *block_start_address,
                start_address + size,
                *block_start_address + block.size
            );
            if start_address + size >= *block_start_address
                && start_address + size < *block_start_address + block.size
            {
                return Err(Error::FlashOverLap);
            }
        }
        // todo: add in a smoosh of flash blocks if they are contigious.
        self.flash_blocks
            .insert(start_address, FlashBlock::new(start_address, size));
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

    #[test]
    fn add_block_overlap() {
        let mut flash_b = Flash::default();
        flash_b.add_block(30, 100).unwrap();
        assert!(flash_b.add_block(40, 100).is_err());
        assert!(flash_b.add_block(131, 100).is_ok());
        assert!(flash_b.add_block(20, 100).is_err());
        assert!(flash_b.add_block(20, 9).is_ok());
        assert!(flash_b.add_block(20, 10).is_err());
    }
}
