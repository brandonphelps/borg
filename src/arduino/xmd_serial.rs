use std::io;
// this is the XModem protocol
// http://ee6115.mit.edu/amulet/xmodem.htm


/// Some sort of xmd serial protocol that is ontop of serial
/// there is some kinda of like sync / ackn setup going on.


const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid packet seq: {0:x?}")]
    InvalidPacketSeq([u8; 2]),

    #[error("I o error")]
    Io(#[from] std::io::Error),
}

const PKTLEN_128: u32 = 128;

pub struct XmdSerial {
    size_of_data: u32,
    mode_of_transfer: u32,
}

impl XmdSerial  {
    pub fn new() -> Self {
        Self {
            size_of_data: 0,
            mode_of_transfer: 0,
        }
    }
    
    pub fn serial_getdata_xmd<P: io::Read + io::Write>(&mut self,
                                                       comm: &mut P,
                                                       mut length: u32) -> Result<Vec<u8>> {
        println!("Serial get data xmd: length: {}", length);
        // todo: supposed to continously send 'C'
        // untill remote responds with first byte. 
        let mut data = vec![];
        comm.write(&[b'C'])?;
        let mut tmp_buffer = [0; 1];

        if length == 0 {
            self.mode_of_transfer = 1;
        } else {
            self.size_of_data = length;
            self.mode_of_transfer = 0;
        }

        // round up to 128 bytes. 
        // if (length & PKTLEN_128 -1) != 0 {
        //     length += PKTLEN_128;
        //     length &= !(PKTLEN_128 - 1);
        // }

        let mut sno = 1;
        let mut data_transfered = 0;
        let mut b_run = true;
        while b_run {    // assumes timeout is set.
            println!("Data transfered: {}", data_transfered);
            comm.read(&mut tmp_buffer)?;
            println!("Read buffer: {:x}", tmp_buffer[0]);
            match tmp_buffer[0] {
                SOH => {
                    match self.get_packet(comm, sno) {
                        Ok(r) => {
                            data.extend(r);
                        },
                        Err(f) => {
                            b_run = false;
                        }
                    };
                    if b_run {
                        println!("\t increment sequence number");
                        sno += 1;
                        data_transfered += PKTLEN_128;
                    }
                },
                EOT => {
                    println!("Sending ACK");
                    comm.write(&[ACK])?;
                    b_run = false;
                },
                _ => {
                    b_run = false;
                }
            }
        }
        self.mode_of_transfer = 0;
        Ok(data)
    }

    /// Reads a package with the given sequence number
    fn get_packet<P: io::Read + io::Write>(&mut self, com: &mut P, sno: u8) -> Result<Vec<u8>>
    {
        println!("Reading packet with sno: {}", sno);
        // sequence buffer, likely a counter of some sort. 
        let (seq, _) = self.get_bytes(com, 2)?;
        println!("Seq {:x} + {:x} = {:x}", seq[0], seq[1], seq[0] + seq[1]);
        if seq[0] + seq[1] != 0xFF {
            return Err(Error::InvalidPacketSeq([seq[0], seq[1]]));
        }
        // xcrc is the transfered crc
        // crc is then the calculated crc. 
        let (buffer, xcrc) = self.get_bytes(com, PKTLEN_128)?;
        let mut tmp_buffer = [0; 2];
        
        com.read(&mut tmp_buffer)?;
        let mut crc = (tmp_buffer[0] as u16) << 8;
        crc += tmp_buffer[1] as u16;

        println!("Crc: {:x} XCrc: {:x}, Seq: {:?}, snow: {}", crc, xcrc, seq, sno);
        if (crc != xcrc) || seq[0] != sno || seq[1] != !sno {
            println!("Sending CAN in get packet");
            // if this happen there is room for a bug to occur.
           com.write(&[CAN])?; 
        } else {
            println!("Sending ack in get packet");
            com.write(&[ACK])?;
        }
        Ok(buffer)
    }

    // returns the data read and a crc if successfful. 
    fn get_bytes<P: io::Read>(&mut self, com: &mut P, len: u32) -> Result<(Vec<u8>, u16)>{
        println!("Getting bytes: {}", len);
        let mut buffer = Vec::with_capacity(len as usize);
        let mut crc: u16 = 0;
        let mut cpt = 0;
        let mut c = [0; 1];
        while cpt < len {
            print!("Getting a single byte: ");
            com.read(&mut c)?;
            println!("{}", c[0]);
            crc = serial_add_crc(c[0] as u16, crc);
            
            if self.size_of_data != 0 || self.mode_of_transfer != 0 {
                buffer.push(c[0]);
                if len == PKTLEN_128 {
                    self.size_of_data -= 1;
                }
            }
            cpt += 1;
        }
        Ok((buffer, crc))
    }
}

fn serial_add_crc(ptr: u16, crc: u16) -> u16 {
    crc << 8 ^ crc16Table[((crc >> 8) ^ ptr) as usize & 0xff]
}

const crc16Table: [u16; 256] =[
	0x0000,0x1021,0x2042,0x3063,0x4084,0x50a5,0x60c6,0x70e7,
	0x8108,0x9129,0xa14a,0xb16b,0xc18c,0xd1ad,0xe1ce,0xf1ef,
	0x1231,0x0210,0x3273,0x2252,0x52b5,0x4294,0x72f7,0x62d6,
	0x9339,0x8318,0xb37b,0xa35a,0xd3bd,0xc39c,0xf3ff,0xe3de,
	0x2462,0x3443,0x0420,0x1401,0x64e6,0x74c7,0x44a4,0x5485,
	0xa56a,0xb54b,0x8528,0x9509,0xe5ee,0xf5cf,0xc5ac,0xd58d,
	0x3653,0x2672,0x1611,0x0630,0x76d7,0x66f6,0x5695,0x46b4,
	0xb75b,0xa77a,0x9719,0x8738,0xf7df,0xe7fe,0xd79d,0xc7bc,
	0x48c4,0x58e5,0x6886,0x78a7,0x0840,0x1861,0x2802,0x3823,
	0xc9cc,0xd9ed,0xe98e,0xf9af,0x8948,0x9969,0xa90a,0xb92b,
	0x5af5,0x4ad4,0x7ab7,0x6a96,0x1a71,0x0a50,0x3a33,0x2a12,
	0xdbfd,0xcbdc,0xfbbf,0xeb9e,0x9b79,0x8b58,0xbb3b,0xab1a,
	0x6ca6,0x7c87,0x4ce4,0x5cc5,0x2c22,0x3c03,0x0c60,0x1c41,
	0xedae,0xfd8f,0xcdec,0xddcd,0xad2a,0xbd0b,0x8d68,0x9d49,
	0x7e97,0x6eb6,0x5ed5,0x4ef4,0x3e13,0x2e32,0x1e51,0x0e70,
	0xff9f,0xefbe,0xdfdd,0xcffc,0xbf1b,0xaf3a,0x9f59,0x8f78,
	0x9188,0x81a9,0xb1ca,0xa1eb,0xd10c,0xc12d,0xf14e,0xe16f,
	0x1080,0x00a1,0x30c2,0x20e3,0x5004,0x4025,0x7046,0x6067,
	0x83b9,0x9398,0xa3fb,0xb3da,0xc33d,0xd31c,0xe37f,0xf35e,
	0x02b1,0x1290,0x22f3,0x32d2,0x4235,0x5214,0x6277,0x7256,
	0xb5ea,0xa5cb,0x95a8,0x8589,0xf56e,0xe54f,0xd52c,0xc50d,
	0x34e2,0x24c3,0x14a0,0x0481,0x7466,0x6447,0x5424,0x4405,
	0xa7db,0xb7fa,0x8799,0x97b8,0xe75f,0xf77e,0xc71d,0xd73c,
	0x26d3,0x36f2,0x0691,0x16b0,0x6657,0x7676,0x4615,0x5634,
	0xd94c,0xc96d,0xf90e,0xe92f,0x99c8,0x89e9,0xb98a,0xa9ab,
	0x5844,0x4865,0x7806,0x6827,0x18c0,0x08e1,0x3882,0x28a3,
	0xcb7d,0xdb5c,0xeb3f,0xfb1e,0x8bf9,0x9bd8,0xabbb,0xbb9a,
	0x4a75,0x5a54,0x6a37,0x7a16,0x0af1,0x1ad0,0x2ab3,0x3a92,
	0xfd2e,0xed0f,0xdd6c,0xcd4d,0xbdaa,0xad8b,0x9de8,0x8dc9,
	0x7c26,0x6c07,0x5c64,0x4c45,0x3ca2,0x2c83,0x1ce0,0x0cc1,
	0xef1f,0xff3e,0xcf5d,0xdf7c,0xaf9b,0xbfba,0x8fd9,0x9ff8,
	0x6e17,0x7e36,0x4e55,0x5e74,0x2e93,0x3eb2,0x0ed1,0x1ef0
];


#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, io::{ErrorKind, Read, Write}};

    pub struct MockSerial {
        buffer: VecDeque<u8>,
    }

    unsafe impl Send for MockSerial {}
    unsafe impl Sync for MockSerial {}

    impl MockSerial {
        fn new() -> Self {
            Self {
                buffer: VecDeque::new(),
            }
        }
    }

    impl std::io::Read for MockSerial {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.buffer.len() > 1 {
                buf[0] = self.buffer.pop_front().unwrap();
                Ok(1)
            } else {
                Err(std::io::Error::new(ErrorKind::TimedOut, "hello"))
            }
        }
    }

    impl std::io::Write for MockSerial {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            for i in buf {
                self.buffer.push_back(*i)
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_mockserial() {
        let mut mock_serial = MockSerial::new();
        let mut buffer = [1,2];
        assert!(mock_serial.read(&mut buffer).is_err());
        let mut buffer = [1];
        assert!(mock_serial.read(&mut buffer).is_err());
        assert!(mock_serial.write(&[1,2,3,4,5,6]).is_ok());
        let mut buffer = [1];
        let r = mock_serial.read(&mut buffer).unwrap();
        assert_eq!(r, 1);
        assert_eq!(buffer[0], 1);
        let r = mock_serial.read(&mut buffer).unwrap();
        assert_eq!(r, 1);
        assert_eq!(buffer[0], 2);
        let r = mock_serial.read(&mut buffer).unwrap();
        assert_eq!(r, 1);
        assert_eq!(buffer[0], 3);
    }

    #[test]
    fn test_put_xmd() {
        
    }
}
