use std::fs::File;
use std::io::Read;
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
    fn load(input: &[u8], n: usize) -> Result<Vec<Self>, MCError> {
        let mut frame = Vec::<Self>::new();
        let (mut next, mut df) = Self::from_bytes((input, 0))?;
        frame.push(df);
        loop {
            if frame.len() == n {
                break;
            }
            (next, df) = Self::from_bytes(next)?;
            frame.push(df);
        }
        Ok(frame)
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
    fn load(input: &[u8], n: usize) -> Result<Vec<Self>, MCError> {
        let mut frame = Vec::<Self>::new();
        let (mut next, mut df) = Self::from_bytes((input, 0))?;
        frame.push(df);
        loop {
            if frame.len() == n {
                break;
            }
            (next, df) = Self::from_bytes(next)?;
            frame.push(df);
        }
        Ok(frame)
    }
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Frame {
    data: [u8; FRAME],
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Block {
    data: [u8; BLOCK],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataBlock {
    title_frame: TitleFrame,
    // len 3
    icon_frames: Vec<Frame>,
}

impl DataBlock {
    pub fn load_data_block(b: Block) -> Result<Self, MCError> {
        let (_, title_frame) = TitleFrame::from_bytes((&b.data, 0))?;
        println!("{}", title_frame);

        let icon_frames = DataBlock::read_icon_frames(&b.data[FRAME..])?;
        for (i, f) in icon_frames.iter().enumerate() {
            println!("IF{} => {:02x?}", i, f);
        }

        Ok(DataBlock {
            title_frame,
            icon_frames,
        })
    }

    pub fn load_all_data_blocks(v: &[Block]) -> Result<Vec<Self>, MCError> {
        let mut out = Vec::<Self>::new();
        for i in v {
            out.push(Self::load_data_block(*i)?);
        }

        Ok(out)
    }

    fn read_icon_frames(input: &[u8]) -> Result<Vec<Frame>, MCError> {
        let mut frame = Vec::<Frame>::new();
        let (mut next, mut f) = Frame::from_bytes((input, 0))?;
        frame.push(f);
        loop {
            if frame.len() == 3 {
                break;
            }
            (next, f) = Frame::from_bytes(next)?;
            frame.push(f);
        }
        Ok(frame)
    }
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct TitleFrame {
    id: [u8; 2],
    display: u8,
    block_num: u8,
    title: [u8; 64],
    reserved: [u8; 28],
    icon_palette: [u8; 16],
}

impl TitleFrame {
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
            "\n Display: {}\n Block Number: {}\n Filename: {}\nIcon Palette: {:02x?}",
            self.display, self.block_num, name, self.icon_palette
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoBlock {
    header: Header,
    //#[deku(len = 15)]
    dir_frames: Vec<DirectoryFrame>,
    //#[deku(len = 20)]
    broken_frames: Vec<BrokenFrame>,
    //#[deku(len = frame*7)]
    //unused_frames: Vec<Frame>,
    //wr_test_frame: Header,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemCard {
    info_block: InfoBlock,
    //#[deku(len = 15)]
    data_blocks: Vec<Block>,
}

impl InfoBlock {
    pub fn open(b: Block) -> Result<(), MCError> {
        //let header = Header::read(&mut reader)?;
        let (_, header) = Header::from_bytes((&b.data, 0))?;
        println!("{:?}", header);

        // Read directory frames
        let dir_frames = DirectoryFrame::load(&b.data[FRAME..], 15)?;
        for (i, d) in dir_frames.iter().enumerate() {
            println!("DF{} => {}", i, d);
        }

        // Read broken frames
        let broken_frames = BrokenFrame::load(&b.data[FRAME * 16..], 20)?;
        for (i, d) in broken_frames.iter().enumerate() {
            println!("BF{} => {:?}", i, d);
        }

        /*
                let uf = Frame::read_unused(&mut reader)?;
                for (i, u) in uf.iter().enumerate() {
                    //println!("UnusedFrame{} => {:?}", i, u);
                }

                let wr_test_frame = Header::read(&mut reader)?;
                //println!("{:?}", wtheader);
        */
        Ok(())
    }
}

impl MemCard {
    pub fn open(filename: String) -> Result<(), MCError> {
        let mut file = File::open(&filename)?;

        // Load Info Block
        let mut block0 = Block { data: [0u8; BLOCK] };
        file.read_exact(&mut block0.data)?;
        InfoBlock::open(block0)?;

        // Load Data Blocks
        let mut blocks = Vec::<Block>::new();
        loop {
            let mut block = Block { data: [0u8; BLOCK] };
            file.read_exact(&mut block.data)?;
            blocks.push(block);
            if blocks.len() == 15 {
                break;
            }
        }
        let _ = DataBlock::load_all_data_blocks(&blocks)?;

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
