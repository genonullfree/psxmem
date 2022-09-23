use std::fs::File;
use std::io::{BufWriter, Read};
use std::{fmt, str};

use deku::prelude::*;
use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat};
use png::Encoder;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BAState {
    AllocFirst = 0x51,
    AllocMid = 0x52,
    AllocLast = 0x53,
    Free = 0xa0,
    FreeFirst = 0xa1,
    FreeMid = 0xa2,
    FreeLast = 0xa3,
    UNKNOWN,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Japan,
    America,
    Europe,
    UNKNOWN,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum License {
    Sony,
    Licensed,
    UNKNOWN,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionInfo {
    region: Region,
    license: License,
    name: String,
}

impl DirectoryFrame {
    fn load(input: &[u8], n: usize) -> Result<Vec<Self>, MCError> {
        let mut frame = Vec::<Self>::new();
        validate_checksum(input)?;
        let (mut next, mut df) = Self::from_bytes((input, 0))?;
        frame.push(df);
        loop {
            if frame.len() == n {
                break;
            }
            let (input, _) = next;
            validate_checksum(input)?;
            (next, df) = Self::from_bytes(next)?;
            frame.push(df);
        }
        Ok(frame)
    }

    fn get_alloc_state(&self) -> BAState {
        match self.state {
            0x51 => BAState::AllocFirst,
            0x52 => BAState::AllocMid,
            0x53 => BAState::AllocLast,
            0xa0 => BAState::Free,
            0xa1 => BAState::FreeFirst,
            0xa2 => BAState::FreeMid,
            0xa3 => BAState::FreeLast,
            _ => BAState::UNKNOWN,
        }
    }

    fn get_region_info(&self) -> Result<RegionInfo, MCError> {
        let region = match self.filename[1] {
            b'I' => Region::Japan,
            b'A' => Region::America,
            b'E' => Region::Europe,
            _ => Region::UNKNOWN,
        };

        let license = match self.filename[3] {
            b'C' => License::Sony,
            b'L' => License::Licensed,
            _ => License::UNKNOWN,
        };

        let name = str::from_utf8(&self.filename[12..])?.to_string();

        Ok(RegionInfo {
            region,
            license,
            name,
        })
    }
}

impl fmt::Display for DirectoryFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\n State: {:?}\n Filesize: {}\n Next block: {}\n Region Info: {:?}\n Checksum: {}",
            self.get_alloc_state(),
            self.filesize,
            self.next_block,
            self.get_region_info(),
            self.checksum
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
        validate_checksum(input)?;
        let (mut next, mut df) = Self::from_bytes((input, 0))?;
        frame.push(df);
        loop {
            if frame.len() == n {
                break;
            }
            let (input, _) = next;
            validate_checksum(input)?;
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

impl Frame {
    fn load(input: &[u8], n: usize) -> Result<Vec<Self>, MCError> {
        let mut frame = Vec::<Self>::new();
        validate_checksum(input)?;
        let (mut next, mut df) = Self::from_bytes((input, 0))?;
        frame.push(df);
        loop {
            if frame.len() == n {
                break;
            }
            let (input, _) = next;
            validate_checksum(input)?;
            (next, df) = Self::from_bytes(next)?;
            frame.push(df);
        }
        Ok(frame)
    }
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct Block {
    data: [u8; BLOCK],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataBlock {
    title_frame: TitleFrame,
    // len 1-3
    icon_frames: Vec<Frame>,
    data_frames: Vec<Frame>,
}

impl DataBlock {
    pub fn load_data_block(b: Block) -> Result<Self, MCError> {
        // Read title frame
        let (_, title_frame) = TitleFrame::from_bytes((&b.data, 0))?;

        // Read icon frame(s)
        let num_frames = title_frame.display as usize & 0x03;
        let icon_frames = DataBlock::read_n_frames(&b.data[FRAME..], num_frames)?;

        // Read data frame
        // title_frame len + (icon_frame len * num icon_frames)
        let next = FRAME + (FRAME * icon_frames.len());
        let num_frames = b.data[next..].len() / FRAME;
        let data_frames = DataBlock::read_n_frames(&b.data[next..], num_frames)?;

        Ok(DataBlock {
            title_frame,
            icon_frames,
            data_frames,
        })
    }

    pub fn load_all_data_blocks(v: &[Block]) -> Result<Vec<Self>, MCError> {
        let mut out = Vec::<Self>::new();
        for i in v {
            out.push(Self::load_data_block(*i)?);
        }

        Ok(out)
    }

    fn read_n_frames(input: &[u8], num_frames: usize) -> Result<Vec<Frame>, MCError> {
        let mut frame = Vec::<Frame>::new();
        let (mut next, mut f) = Frame::from_bytes((input, 0))?;
        frame.push(f);
        loop {
            if frame.len() == num_frames {
                break;
            }
            (next, f) = Frame::from_bytes(next)?;
            frame.push(f);
        }
        Ok(frame)
    }

    pub fn write<T: std::io::Write>(&self, out: &mut T) -> Result<(), MCError> {
        let t = self.title_frame.to_bytes()?;
        out.write_all(&t)?;

        for ic in &self.icon_frames {
            let i = ic.to_bytes()?;
            out.write_all(&i)?;
        }

        for df in &self.data_frames {
            let d = df.to_bytes()?;
            out.write_all(&d)?;
        }

        Ok(())
    }

    pub fn export_all_images(&self) -> Result<(), MCError> {
        // Extract out individual frames
        for (n, i) in self.icon_frames.iter().enumerate() {
            let filename = format!("{}_frame{}.png", self.title_frame.decode_title()?, n);
            let file = File::create(filename)?;
            let mut w = BufWriter::new(file);
            let mut enc = Encoder::new(&mut w, 16, 16);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);

            let mut writer = enc.write_header()?;

            let pixel_data = self.translate_bmp_to_rgba(i)?;

            writer.write_image_data(&pixel_data)?;
        }

        // If > 1 frame, extract it out as a gif too
        if self.icon_frames.len() > 1 {
            self.export_gif()?;
        }

        Ok(())
    }

    fn export_gif(&self) -> Result<(), MCError> {
        let w = 16;
        let h = 16;
        let filename = format!("{}.gif", self.title_frame.decode_title()?);
        let mut file = File::create(filename)?;
        let mut enc = GifEncoder::new(&mut file, w, h, &[])?;
        enc.set_repeat(Repeat::Infinite)?;
        for i in self.icon_frames.iter() {
            let mut pixels = self.translate_bmp_to_rgba(i)?;
            let gifframe = GifFrame::from_rgba(w, h, &mut *pixels);
            enc.write_frame(&gifframe)?;
        }

        Ok(())
    }

    fn translate_bmp_to_rgba(&self, f: &Frame) -> Result<Vec<u8>, MCError> {
        let mut rgba = Vec::<u8>::new();

        // Each byte in the data array is 2x 4bit addresses into the 16x u16 array palette
        for v in f.data {
            for s in 0..2 {
                let index = (v >> (4 * s as u8)) & 0x0f;
                let pixel: u16 = self.title_frame.icon_palette[index as usize];
                // format is abgr, needs to be pushed rgba
                //
                // push red
                rgba.push(((pixel & 0x001f) as u16) as u8 * 8);
                // push green
                rgba.push(((pixel & (0x001f << 5)) as u16 >> 5) as u8 * 8);
                // push blue
                rgba.push(((pixel & (0x001f << 10)) as u16 >> 10) as u8 * 8);
                // push alpha alpha is either 1 or 0, best results are simply ignored, lol
                rgba.push(255);
            }
        }

        Ok(rgba)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconDisplay {
    OneFrame,
    TwoFrames,
    ThreeFrames,
    UNKNOWNFrames,
}

#[derive(Clone, Copy, Debug, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(endian = "little")]
pub struct TitleFrame {
    id: [u8; 2],
    display: u8,
    block_num: u8,
    title: [u8; 64],
    reserved: [u8; 28],
    icon_palette: [u16; 16],
}

impl TitleFrame {
    pub fn decode_title(self) -> Result<String, MCError> {
        // Shift JIS decode the Title
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

    fn get_icon_display(&self) -> IconDisplay {
        match self.display {
            0x11 => IconDisplay::OneFrame,
            0x12 => IconDisplay::TwoFrames,
            0x13 => IconDisplay::ThreeFrames,
            _ => IconDisplay::UNKNOWNFrames,
        }
    }
}

impl fmt::Display for TitleFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self.decode_title() {
            Ok(s) => s,
            Err(_) => "Unknown".to_string(),
        };
        write!(
            f,
            "\n Filename: {}\n Icon: {:?}\n Block Number: {}",
            name,
            self.get_icon_display(),
            self.block_num
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoBlock {
    header: Header,
    //len = 15
    dir_frames: Vec<DirectoryFrame>,
    //len = 20
    broken_frames: Vec<BrokenFrame>,
    //len = 20
    unused_frames: Vec<Frame>,
    wr_test_frame: Header,
}

impl InfoBlock {
    pub fn open(b: Block) -> Result<Self, MCError> {
        // Validate and load header
        validate_checksum(&b.data)?;
        let (_, header) = Header::from_bytes((&b.data, 0))?;

        // Read directory frames
        let dir_frames = DirectoryFrame::load(&b.data[FRAME..], 15)?;

        // Read broken frames
        let mut offset = (dir_frames.len() * FRAME) + FRAME;
        let broken_frames = BrokenFrame::load(&b.data[offset..], 20)?;

        offset += broken_frames.len() * FRAME;
        let unused_frames = Frame::load(&b.data[offset..], 27)?;

        offset += unused_frames.len() * FRAME;
        validate_checksum(&b.data[offset..])?;
        let (_, wr_test_frame) = Header::from_bytes((&b.data[offset..], 0))?;

        Ok(InfoBlock {
            header,
            dir_frames,
            broken_frames,
            unused_frames,
            wr_test_frame,
        })
    }

    pub fn write<T: std::io::Write>(&self, out: &mut T) -> Result<(), MCError> {
        let mut h = self.header.to_bytes()?;
        out.write_all(update_checksum(&mut h)?)?;

        for df in &self.dir_frames {
            let mut d = df.to_bytes()?;
            out.write_all(update_checksum(&mut d)?)?;
        }

        for bf in &self.broken_frames {
            let mut b = bf.to_bytes()?;
            out.write_all(update_checksum(&mut b)?)?;
        }

        for uf in &self.unused_frames {
            let mut f = uf.to_bytes()?;
            out.write_all(update_checksum(&mut f)?)?;
        }

        let mut wrt = self.wr_test_frame.to_bytes()?;
        out.write_all(update_checksum(&mut wrt)?)?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemCard {
    info: InfoBlock,
    //#[deku(len = 15)]
    data: Vec<DataBlock>,
}

impl MemCard {
    pub fn open(filename: String) -> Result<Self, MCError> {
        let mut file = File::open(&filename)?;

        // Load Info Block
        let mut block0 = Block { data: [0u8; BLOCK] };
        file.read_exact(&mut block0.data)?;
        let info = InfoBlock::open(block0)?;

        // Read Data Blocks
        let mut blocks = Vec::<Block>::new();
        loop {
            let mut block = Block { data: [0u8; BLOCK] };
            file.read_exact(&mut block.data)?;
            blocks.push(block);
            if blocks.len() == 15 {
                break;
            }
        }

        // Load Data Blocks
        let data = DataBlock::load_all_data_blocks(&blocks)?;

        Ok(MemCard { info, data })
    }

    pub fn write(&self, filename: String) -> Result<(), MCError> {
        let mut file = File::create(&filename)?;

        self.info.write(&mut file)?;
        for d in &self.data {
            d.write(&mut file)?;
        }

        Ok(())
    }

    pub fn find_game(&self, search: &str) -> Result<Vec<DataBlock>, MCError> {
        let mut found = Vec::<DataBlock>::new();
        let mut needle = String::from(search);
        needle.make_ascii_lowercase();

        // Find names that match in the data blocks
        for info in &self.data {
            let mut haystack = info.title_frame.decode_title()?;
            haystack.make_ascii_lowercase();

            if haystack.contains(&needle) {
                found.push(info.clone());
            }
        }

        Ok(found)
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

pub fn update_checksum(d: &mut [u8]) -> Result<&[u8], MCError> {
    let c = calc_checksum(d);
    d[FRAME - 1] = c;

    validate_checksum(d)?;

    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memcard_open() {
        let _ = MemCard::open("epsxe000.mcr".to_string()).unwrap();

        /*
        // Export images
        for d in m.data {
            d.export_all_images().unwrap();
        }
        */
    }

    #[test]
    fn memcard_write() {
        let m = MemCard::open("epsxe000.mcr".to_string()).unwrap();

        let w = m.find_game("WILD").unwrap();
        for i in w {
            println!("{}", i.title_frame);
        }

        m.write("test.mcr".to_string()).unwrap();
    }

    #[test]
    fn memcard_modify() {
        let mut a = MemCard::open("epsxe000.mcr".to_string()).unwrap();
        a.info.header.id = [0x11, 0x22];
        a.write("test.mcr".to_string()).unwrap();

        let mut b = MemCard::open("test.mcr".to_string()).unwrap();
        b.info.dir_frames[0].filesize = 4000000;
        b.write("test.mcr".to_string()).unwrap();

        let mut c = MemCard::open("test.mcr".to_string()).unwrap();
        c.info.broken_frames[0].broken_frame = 12345;
        c.write("test.mcr".to_string()).unwrap();
    }
}
