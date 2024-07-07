use std::{
    collections::VecDeque,
    sync::{Arc, Mutex}, time::{Duration, Instant},
};

pub struct BiChannel {
    id: usize,
    // if id == 0,
    // incoming is then data being recived.
    // else it is outgoing.
    incoming: Arc<Mutex<VecDeque<u8>>>,

    // if id == 0,
    // out going is data being sent
    // else it is incoming.
    outgoing: Arc<Mutex<VecDeque<u8>>>,

    // timeout for both reading and writing operations. 
    timeout: Duration,
}

impl BiChannel {
    pub fn new() -> Self {
        Self {
            id: 0,
            incoming: Arc::new(Mutex::new(VecDeque::new())),
            outgoing: Arc::new(Mutex::new(VecDeque::new())),
            timeout: Duration::from_secs(0),
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

impl Clone for BiChannel {
    fn clone(&self) -> Self {
        if self.id != 0 {
            panic!("Invalid to a cloned channel");
        }
        Self {
            id: self.id + 1,
            incoming: self.incoming.clone(),
            outgoing: self.outgoing.clone(),
            timeout: self.timeout,
        }
    }
}

impl std::io::Read for BiChannel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start_timeout = Instant::now();
        loop { 
            let mut read_source = if self.id == 0 {
                self.incoming.lock().unwrap()
            } else {
                self.outgoing.lock().unwrap()
            };

            let read_count = if read_source.len() > buf.len() {
                buf.len() 
            } else {
                read_source.len()
            };
            println!("Read count: {}", read_count);
            if read_count == 0 {
                if start_timeout.elapsed() < self.timeout { 
                    continue;
                } else {
                    return Ok(0);
                }
            }
            
            for i in 0..read_count {
                buf[i] = read_source.pop_front().ok_or(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "out of bytes to read",
                ))?;
            }
            return Ok(buf.len());
        }
    }
}

impl std::io::Write for BiChannel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.id == 0 {
            let mut v = self.outgoing.lock().unwrap();
            for i in buf.iter() {
                v.push_back(*i);
            }
        } else {

            let mut v = self.incoming.lock().unwrap();
            for i in buf.iter() {
                println!("Writing: {}", i);
                v.push_back(*i);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;

    #[test]
    fn test_mock_bi_channel_read() {
        let mut bi_channel = BiChannel::new();
        bi_channel
            .write_all(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
            .unwrap();
        let mut bi_channel_clone = bi_channel.clone();
        let mut b = [0; 10];
        let k = bi_channel_clone.read(&mut b).unwrap();
        assert_eq!(k, 10);
        assert_eq!(&b, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_mock_bi_channel_clone_read() {
        let mut bi_channel = BiChannel::new();
        let mut bi_clone_channel = bi_channel.clone();
        bi_clone_channel
            .write_all(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
            .unwrap();
        let mut b = [0; 10];
        let k = bi_channel.read(&mut b).unwrap();
        assert_eq!(k, 10);
        assert_eq!(&b, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
}
