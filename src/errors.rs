use std::io;
use std::str::Utf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MCError {
    #[error("DekuError: {0}")]
    Deku(#[from] deku::DekuError),

    #[error("IoError: {0}")]
    Io(#[from] io::Error),

    #[error("Error in conversion of oct_to_dev")]
    Utf8Error(#[from] Utf8Error),

    #[error("Checksum does not match expected value")]
    BadChecksum,
}
