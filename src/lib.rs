use std::fs::File;
use std::io::BufReader;
use std::{fmt, str};

use deku::prelude::*;

mod errors;
use crate::errors::MCError;

const BLOCK: usize = 0x2000;
const FRAME: usize = 0x80;

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Header {
    id: [u8; 2],
    pad: [u8; 125],
    checksum: u8,
}

impl Header {
    fn read<T: std::io::Read>(mut input: T) -> Result<Self, MCError> {
        let mut i = vec![0u8; FRAME];
        input.read_exact(&mut i)?;

        validate_checksum(&i)?;

        let (_, h) = Self::from_bytes((&i, 0))?;

        Ok(h)
    }
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct DirectoryFrame {
    state: u32,
    filesize: u32,
    next_block: u16,
    filename: [u8; 21],
    pad: [u8; 96],
    checksum: u8,
}

impl DirectoryFrame {
    fn read<T: std::io::Read>(mut input: T) -> Result<Self, MCError> {
        let mut i = vec![0u8; FRAME];
        input.read_exact(&mut i)?;

        validate_checksum(&i)?;

        let (_, f) = Self::from_bytes((&i, 0))?;

        Ok(f)
    }

    fn read_all<T: std::io::Read>(mut input: T) -> Result<Vec<Self>, MCError> {
        let mut f = Vec::<Self>::new();
        for _ in 0..15 {
            let mut v = vec![0u8; FRAME];
            input.read_exact(&mut v)?;
            f.push(DirectoryFrame::read(&*v)?);
        }

        Ok(f)
    }
}

impl fmt::Display for DirectoryFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match str::from_utf8(&self.filename) {
            Ok(s) => s.to_string(),
            Err(_) => "Unknown".to_string(),
        };
        write!(
            f,
            "\n State: {}\n Filesize: {}\n Next block: {}\n Filename: {}\n Checksum: {}",
            self.state, self.filesize, self.next_block, name, self.checksum
        )
    }
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct BrokenFrame {
    broken_frame: u32,
    pad: [u8; 123],
    checksum: u8,
}

impl BrokenFrame {
    fn read<T: std::io::Read>(mut input: T) -> Result<Self, MCError> {
        let mut i = vec![0u8; FRAME];
        input.read_exact(&mut i)?;

        validate_checksum(&i)?;

        let (_, f) = Self::from_bytes((&i, 0))?;

        Ok(f)
    }

    fn read_all<T: std::io::Read>(mut input: T) -> Result<Vec<Self>, MCError> {
        let mut f = Vec::<Self>::new();
        for _ in 0..20 {
            let mut v = vec![0u8; FRAME];
            input.read_exact(&mut v)?;
            f.push(BrokenFrame::read(&*v)?);
        }

        Ok(f)
    }
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Frame {
    data: [u8; FRAME],
}

impl Frame {
    fn read<T: std::io::Read>(mut input: T) -> Result<Self, MCError> {
        let mut i = vec![0u8; FRAME];
        input.read_exact(&mut i)?;

        let (_, f) = Self::from_bytes((&i, 0))?;

        // TODO validate?
        Ok(f)
    }

    fn read_unused<T: std::io::Read>(mut input: T) -> Result<Vec<Self>, MCError> {
        let mut f = Vec::<Self>::new();
        for _ in 0..7 {
            let mut v = vec![0u8; FRAME];
            input.read_exact(&mut v)?;
            f.push(Frame::read(&*v)?);
        }

        Ok(f)
    }
}
#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Block {
    data: [u8; BLOCK],
}

impl Block {
    fn read<T: std::io::Read>(mut input: T) -> Result<Self, MCError> {
        let mut i = vec![0u8; BLOCK];
        input.read_exact(&mut i)?;

        let (_, f) = Self::from_bytes((&i, 0))?;

        // TODO validate?
        Ok(f)
    }

    fn read_all<T: std::io::Read>(mut input: T) -> Result<Vec<Self>, MCError> {
        let mut f = Vec::<Self>::new();
        for _ in 0..15 {
            let mut v = vec![0u8; BLOCK];
            input.read_exact(&mut v)?;
            f.push(Self::read(&*v)?);
        }

        Ok(f)
    }
}

const SAVE_MAGIC: [u8; 2] = [b'S', b'C'];

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct TitleFrame {
    id: [u8; 2],
    display: u8,
    block_num: u8,
    title: [u8; 64],
    reserved: [u8; 28],
    icon_palette: [u8; 32],
}

impl TitleFrame {
    fn read<T: std::io::Read>(mut input: T) -> Result<Self, MCError> {
        let mut i = vec![0u8; FRAME];
        input.read_exact(&mut i)?;

        let (_, f) = Self::from_bytes((&i, 0))?;

        // TODO validate?
        Ok(f)
    }

    fn read_n<T: std::io::Read>(mut input: T, n: usize) -> Result<Vec<Self>, MCError> {
        let mut f = Vec::<Self>::new();
        for _ in 0..n {
            let mut v = vec![0u8; FRAME];
            input.read_exact(&mut v)?;
            f.push(Self::read(&*v)?);
        }

        Ok(f)
    }

    fn shift_jis_decode(self) -> Result<String, MCError> {
        let mut s = String::new();

        let mut p = 0;
        loop {
            match self.title[p] {
                // TODO: This does not match punctuation marks [0x81, 0x43..0x97]
                0x81 => {
                    if self.title[p + 1] == 0x40 {
                        s.push(' ');
                    }
                }
                0x82 => {
                    if (self.title[p + 1] >= 0x4f && self.title[p + 1] <= 0x58)
                        || (self.title[p + 1] >= 0x60 && self.title[p + 1] <= 0x79)
                    {
                        // Translate 0..9 and A..Z
                        s.push((self.title[p + 1] - 0x1f) as char);
                    } else if self.title[p + 1] >= 0x81 && self.title[p + 1] <= 0x9a {
                        // Translate a..z
                        s.push((self.title[p + 1] - 0x20) as char);
                    }
                }
                0x00 => break,
                _ => (),
            }
            p += 2;
            if p >= self.title.len() {
                break;
            }
        }

        Ok(s)
    }
}

impl fmt::Display for TitleFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self.shift_jis_decode() {
            Ok(s) => s,
            Err(_) => "Unknown".to_string(),
        };
        write!(
            f,
            "\n Display: {}\n Block Number: {}\n Filename: {}",
            self.display, self.block_num, name
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemCard {
    header: Header,
    //#[deku(len = 15)]
    dir_frames: Vec<DirectoryFrame>,
    //#[deku(len = 20)]
    broken_frames: Vec<BrokenFrame>,
    //#[deku(len = frame*7)]
    unused_frames: Vec<Frame>,
    wr_test_frame: Header,
    //#[deku(len = 15)]
    blocks: Vec<Block>,
}

impl MemCard {
    pub fn open(filename: String) -> Result<(), MCError> {
        let file = File::open(&filename)?;
        let mut reader = BufReader::new(file);

        let header = Header::read(&mut reader)?;
        println!("{:?}", header);

        let df = DirectoryFrame::read_all(&mut reader)?;
        for (i, d) in df.iter().enumerate() {
            println!("DirectoryFrame{} => {}", i, d);
        }

        let bf = BrokenFrame::read_all(&mut reader)?;
        for (i, b) in bf.iter().enumerate() {
            //println!("BrokenFrame{} => {:?}", i, b);
        }

        let uf = Frame::read_unused(&mut reader)?;
        for (i, u) in uf.iter().enumerate() {
            //println!("UnusedFrame{} => {:?}", i, u);
        }

        let wr_test_frame = Header::read(&mut reader)?;
        //println!("{:?}", wtheader);

        let blocks = Block::read_all(&mut reader)?;
        for (i, b) in blocks.iter().enumerate() {
            //println!("Block{} => {:?}", i, b);
        }
        /*
        let tf = TitleFrame::read_n(&mut reader, 400)?;
        for (i, t) in tf.iter().enumerate() {
            if t.id == SAVE_MAGIC {
                println!("TitleFrame{}: {}", i, t);
            }
        }
        */
        Ok(())
    }
}

pub fn calc_checksum(d: &[u8]) -> u8 {
    let mut c = 0;
    for i in d.iter().take(FRAME - 1) {
        c ^= *i;
    }
    c
}

pub fn validate_checksum(d: &[u8]) -> Result<(), MCError> {
    let c = calc_checksum(d);
    if c != d[FRAME - 1] {
        return Err(MCError::BadChecksum);
    }

    Ok(())
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn memcard_open() {
        MemCard::open("epsxe000.mcr".to_string()).unwrap();
    }
}
