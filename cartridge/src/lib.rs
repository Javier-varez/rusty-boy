//!
//! Abstraction over a physical Game Boy cartridge.
//!
//! Game Boy cartridges can contain:
//! - A ROM memory, maximum size depending on the mapper type. Typically mapped from 0x0000 to 0x8000.
//! - A RAM memory. Typically mapped from 0xA000 to 0xC000.
//! - An RTC, with memory-mapped registers. Address ranges depends on the specific cartridge type.
//!
//! ```rust,no_run
//! use cartridge::Cartridge;
//!
//! let data = std::fs::read("my_cartridge.gb").expect("Unable to read cartridge from disk");
//! let mut cartridge = cartridge::new_mapper(data).expect("Cartridge is not valid");
//!
//! // Query the cartridge header with:
//! let header = cartridge.header();
//! let title = header.title;
//! println!("Cartridge title is {title}");
//!
//! // Perform memory-mapped accesses with:
//! assert_eq!(cartridge.read(0x4000u16), 0xFA);
//! cartridge.write(0xA000u16, 0x53);
//! ```
//!
#![no_std]
#![warn(missing_docs)]

pub mod header;
pub mod mappers;

extern crate alloc;

use alloc::boxed::Box;
use mappers::Mapper;

/// A cartridge error
#[derive(Debug)]
pub enum Error {
    /// The cartridge does not have RAM memory, but an operation involving RAM memory was
    /// requested.
    CartridgeHasNoRam,

    /// The given RAM backup does not have the expected size of the actual RAM in the cartridge.
    UnexpectedRamSize {
        /// Expected RAM size in bytes
        expected: usize,
        /// Actual RAM size in bytes
        actual: usize,
    },

    /// The header of the cartridge is not valid.
    InvalidHeader(header::Error),

    /// The given mapper is not supported
    UnsupportedMapper(header::CartridgeType),
}

impl From<header::Error> for Error {
    fn from(value: header::Error) -> Self {
        Self::InvalidHeader(value)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// A cartridge with any mapper
pub type Cartridge = Box<dyn Mapper>;

pub use mappers::new_mapper;
