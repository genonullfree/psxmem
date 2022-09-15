use std::fs::File;
use std::io::BufReader;

use deku::prelude::*;

mod errors;
use crate::errors::MCError;

const BLOCK: usize = 0x2000;
const FRAME: usize = 0x80;

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Header {
    id: [u8; 2],
    unused: [u8; 125],
    checksum: u8,
}

impl Header {
    fn read<T: std::io::Read>(mut input: T) -> Result<Self, MCError> {
        let mut i = vec![0u8; FRAME];
        input.read_exact(&mut i)?;

        let (_, header) = Self::from_bytes((&i, 0))?;

        // TODO validate
        Ok(header)
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

        let (_, df) = Self::from_bytes((&i, 0))?;

        // TODO validate?
        Ok(df)
    }

    fn read_all<T: std::io::Read>(mut input: T) -> Result<Vec<Self>, MCError> {
        let mut df = Vec::<Self>::new();
        for _ in 0..15 {
            let mut v = vec![0u8; FRAME];
            input.read_exact(&mut v)?;
            df.push(DirectoryFrame::read(&*v)?);
        }

        Ok(df)
    }
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct BrokenFrame {
    broken_frame: u32,
    pad: [u8; 123],
    checksum: u8,
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Block {
    data: [u8; BLOCK],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemCard {
    header: Header,
    //#[deku(len = 15)]
    dir_frames: Vec<DirectoryFrame>,
    //#[deku(len = 20)]
    broken_frames: Vec<BrokenFrame>,
    //#[deku(len = 128*7)]
    unused_frames: Vec<u8>,
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
            println!("{} => {:?}", i, d);
        }

        Ok(())
    }
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
        MemCard::open("epsxe000.mcr".to_string());
    }
}
