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
            data: vec![0xFF;size as usize],
        }
    }

    pub fn write(&mut self, address: u32, data: &[u8]) -> Result<()> {
        println!("Flash write: {:x} {:x?}", address, data);
        if address < self.address || address + data.len() as u32 > self.address + self.size {
            return Err(Error::FlashOutOfBounds(address));
        }
        for (addr, d) in data.iter().enumerate() {
            let w_addr = address - self.address + addr as u32;
            self.data[w_addr as usize] = *d;
        }
        Ok(())
    }

    pub fn read(&mut self, address: u32, length: u32) -> Result<Vec<u8>> {
        if address < self.address || address + length > self.address + length {
            return Err(Error::FlashOutOfBounds(address));
        }
        let mut data = Vec::new();

        for d in 0..length {
            let r_addr = address - self.address + d;
            data.push(self.data[r_addr as usize]);
        }

        Ok(data)
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
        for (key, block) in self.flash_blocks.iter_mut() {
            if address >= *key && (address + data.len() as u32) < key + block.size {
                return block.write(address, data);
            }
        }
        Err(Error::FlashOutOfBounds(address))
    }

    pub fn read(&mut self, address: u32, length: u32) -> Result<Vec<u8>> {
        println!("Reading {} bytes at {:x}", length, address);
        for (key, block) in self.flash_blocks.iter_mut() {
            if address >= *key && (address + length) < key + block.size {
                // println!("Reading from block: {:x}", key);
                return block.read(address, length);
            }
        }
        Err(Error::FlashOutOfBounds(address))
    }

    pub fn add_block(&mut self, start_address: u32, size: u32) -> Result<()> {
        for (key, block) in self.flash_blocks.iter_mut() {
            if start_address >= *key && (start_address + size) < key + block.size {
                return Err(Error::FlashOverLap);
            }
        }
        // todo: add in a smooth of flash blocks if they are contigious.
        self.flash_blocks
            .insert(start_address, FlashBlock::new(start_address, size));
        Ok(())
    }
}
