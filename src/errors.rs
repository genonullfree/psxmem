use gif::EncodingError as GifEncodingError;
use png::EncodingError;
use std::io;
use std::str::Utf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MCError {
    #[error("DekuError: {0}")]
    Deku(#[from] deku::DekuError),

    #[error("IoError: {0}")]
    Io(#[from] io::Error),

    #[error("Uft8Error: {0}")]
    Utf8Error(#[from] Utf8Error),

    #[error("Unable to encode to PNG")]
    PngEncodingError(#[from] EncodingError),

    #[error("Unable to encode to GIF")]
    GifEncodingError(#[from] GifEncodingError),

    #[error("Checksum does not match expected value")]
    BadChecksum,
}
