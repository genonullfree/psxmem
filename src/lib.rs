use std::io::{Read, BufReader};
use std::fs::File;

use deku::prelude::*;

mod errors;
use crate::errors::MCError;

const BLOCK: usize = 0x2000;
const FRAME: usize = 0x80;

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(endian = "little")]
pub struct Header {
    id: [u8; 2],
    unused: [u8; 125],
    checksum: u8,
}
#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(endian = "little")]
pub struct DirectoryFrame {
    state: u32,
    filesize: u32,
    next_block: u16,
    filename: [u8; 21],
    pad: [u8; 95],
    checksum: u8,
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(endian = "little")]
pub struct BrokenFrame {
    broken_frame: u32,
    pad: [u8; 123],
    checksum: u8,
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(endian = "little")]
pub struct Block {
    data: [u8; BLOCK],
}

#[derive(Clone, Debug, PartialEq)]
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
        let mut h = vec![0u8; 128];
        reader.read_exact(&mut h)?;
        let (_, header) = Header::from_bytes((&h, 0)).unwrap();
        println!("{:?}", header);
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
