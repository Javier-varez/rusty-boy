//! The cartridge header provides information about the game. It is always located at address
//! 0x150 and determines the type of mapper IC used by the cartridge, the size of the ROM, size of
//! the RAM (if any) and the title of the game, among other data.

/// The licensee of the game
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Licensee {
    /// Uses the old cartridge format
    Old(u8),
    /// Uses the new cartridge format
    New([u8; 2]),
}

/// Represents the RAM size in a game
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RamSize {
    /// There is no RAM
    None,
    /// 8 KiB of RAM
    KiloBytes8,
    /// 32 KiB of RAM
    KiloBytes32,
    /// 128 KiB of RAM
    KiloBytes128,
    /// 64 KiB of RAM
    KiloBytes64,
    /// Unknown RAM size. This variant is not found in comercial games.
    Unknown(u8),
}

impl RamSize {
    /// Converts the `RamSize` enum to the number of bytes, if the RAM size is known.
    pub fn into_usize(self) -> Option<usize> {
        match self {
            Self::None => Some(0),
            Self::KiloBytes8 => Some(8 * 1024),
            Self::KiloBytes32 => Some(32 * 1024),
            Self::KiloBytes128 => Some(128 * 1024),
            Self::KiloBytes64 => Some(64 * 1024),
            Self::Unknown(_) => None,
        }
    }
}

impl core::fmt::Display for RamSize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RamSize::None => write!(f, "None"),
            RamSize::KiloBytes8 => write!(f, "8 KiB"),
            RamSize::KiloBytes32 => write!(f, "32 KiB"),
            RamSize::KiloBytes64 => write!(f, "64 KiB"),
            RamSize::KiloBytes128 => write!(f, "128 KiB"),
            RamSize::Unknown(v) => write!(f, "Unknown ({})", v),
        }
    }
}

impl From<u8> for RamSize {
    fn from(value: u8) -> Self {
        match value {
            0 => RamSize::None,
            2 => RamSize::KiloBytes8,
            3 => RamSize::KiloBytes32,
            4 => RamSize::KiloBytes128,
            5 => RamSize::KiloBytes64,
            v => RamSize::Unknown(v),
        }
    }
}

/// The cartridge type indicates which type of mapper and hardware the cartridge contains
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CartridgeType {
    /// Fixed ROM memory with a fixed mapping and no additional controls
    RomOnly,
    /// MBC1 mapper, without RAM or battery
    Mbc1,
    /// MBC1 mapper, with RAM but without battery
    Mbc1Ram,
    /// MBC1 mapper, with RAM and battery
    Mbc1RamBattery,
    /// MBC2 mapper, without battery. MBC2 has embedded RAM into the mapper
    Mbc2,
    /// MBC2 mapper, with battery. MBC2 has embedded RAM into the mapper
    Mbc2Battery,
    /// Fixed ROM and RAM, without a mapper IC
    RomRam,
    /// Fixed ROM and RAM, without a mapper IC, but with battery
    RomRamBattery,
    /// MMM01 mapper, without RAM or battery
    Mmm01,
    /// MMM01 mapper, with RAM and without battery
    Mmm01Ram,
    /// MMM01 mapper, with RAM and battery
    Mmm01RamBattery,
    /// MBC3 mapper, with RTC and battery
    Mbc3TimerBattery,
    /// MBC3 mapper, with RTC, RAM and battery
    Mbc3TimerRamBattery,
    /// MBC3 mapper, without RTC, RAM or battery
    Mbc3,
    /// MBC3 mapper, with RAM
    Mbc3Ram,
    /// MBC3 mapper, with RAM and battery
    Mbc3RamBattery,
    /// MBC5 mapper, without RAM or battery
    Mbc5,
    /// MBC5 mapper, with RAM and without battery
    Mbc5Ram,
    /// MBC5 mapper, with RAM and battery
    Mbc5RamBattery,
    /// MBC5 mapper, with a rumble pack
    Mbc5Rumble,
    /// MBC5 mapper, with a rumble pack and RAM
    Mbc5RumbleRam,
    /// MBC5 mapper, with a rumble pack, RAM and battery
    Mbc5RumbleRamBattery,
    /// MBC6 mapper
    Mbc6,
    /// MBC7 mapper, with accelerometer, rumble pack, RAM and battery
    Mbc7SensorRumbleRamBattery,
    /// The cartridge is the game boy camera
    PocketCamera,
    /// Bandai Tama cartridge
    BandaiTama5,
    /// HUC3 mapper
    Huc3,
    /// HUC1 mapper
    Huc1RamBattery,
    /// Unknown mapper and cartridge type
    Unknown(u8),
}

impl From<u8> for CartridgeType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::Mbc1,
            0x02 => CartridgeType::Mbc1Ram,
            0x03 => CartridgeType::Mbc1RamBattery,
            0x05 => CartridgeType::Mbc2,
            0x06 => CartridgeType::Mbc2Battery,
            0x08 => CartridgeType::RomRam,
            0x09 => CartridgeType::RomRamBattery,
            0x0B => CartridgeType::Mmm01,
            0x0C => CartridgeType::Mmm01Ram,
            0x0D => CartridgeType::Mmm01RamBattery,
            0x0F => CartridgeType::Mbc3TimerBattery,
            0x10 => CartridgeType::Mbc3TimerRamBattery,
            0x11 => CartridgeType::Mbc3,
            0x12 => CartridgeType::Mbc3Ram,
            0x13 => CartridgeType::Mbc3RamBattery,
            0x19 => CartridgeType::Mbc5,
            0x1A => CartridgeType::Mbc5Ram,
            0x1B => CartridgeType::Mbc5RamBattery,
            0x1C => CartridgeType::Mbc5Rumble,
            0x1D => CartridgeType::Mbc5RumbleRam,
            0x1E => CartridgeType::Mbc5RumbleRamBattery,
            0x20 => CartridgeType::Mbc6,
            0x22 => CartridgeType::Mbc7SensorRumbleRamBattery,
            0xFC => CartridgeType::PocketCamera,
            0xFD => CartridgeType::BandaiTama5,
            0xFE => CartridgeType::Huc3,
            0xFF => CartridgeType::Huc1RamBattery,
            v => CartridgeType::Unknown(v),
        }
    }
}

impl core::fmt::Display for CartridgeType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Errors related to the cartridge header
#[derive(Debug)]
pub enum Error {
    /// The cartridge does not contain a header
    NoHeader,
    /// The cartridge contains a header with an invalid title
    InvalidTitle,
    /// The cartridge contains a header with an invalid RAM size
    InvalidRamSize,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// The cartridge contains a header with an invalid RAM size
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CartridgeHeader<'a> {
    /// The instructions in the entrypoint of the cartridge, located at address 0x100
    pub entrypoint: &'a [u8],
    /// The logo stored in the cartridge. In commercial cartridges this is the Nintendo logo.
    pub logo: &'a [u8],
    /// The title of the game
    pub title: &'a str,
    /// A code that identifies the manufacturer
    pub manufacturer_code: Option<&'a [u8]>,
    /// A code that determines if the game is a game boy color only game
    pub cgb_flag: Option<u8>,
    /// The licensee of the cartridge. Each licensee has a unique code assigned by Nintendo.
    pub licensee: Licensee,
    /// The size of the ROM, in bytes
    pub rom_size: usize,
    /// The size of the RAM
    pub ram_size: RamSize,
    /// The type of cartridge
    pub cartridge_type: CartridgeType,
}

impl<'a> CartridgeHeader<'a> {
    /// Attempts to construct a cartridge header from the raw contents of the cartridge
    pub fn try_new(data: &'a [u8]) -> Result<Self, Error> {
        if data.len() < 0x150 {
            return Err(Error::NoHeader);
        }

        let title = &data[0x134..0x144];
        let title_length = title.iter().take_while(|b| **b != 0).count();
        let title = &title[..title_length];
        let title = core::str::from_utf8(title).map_err(|_| Error::InvalidTitle)?;

        let manufacturer_code = if title.len() > 11 {
            None
        } else {
            Some(&data[0x13f..0x143])
        };
        let cgb_flag = if title.len() > 14 {
            None
        } else {
            Some(data[0x143])
        };

        let licensee = if data[0x14B] == 0x33 {
            Licensee::New([data[0x144], data[0x145]])
        } else {
            Licensee::Old(data[0x14B])
        };

        let ty: CartridgeType = data[0x147].into();

        let rom_size = (32 * 1024) << data[0x148] as usize;
        let ram_size = data[0x149].into();

        Ok(CartridgeHeader {
            entrypoint: &data[0x100..0x104],
            logo: &data[0x104..0x134],
            title,
            manufacturer_code,
            cgb_flag,
            licensee,
            rom_size,
            ram_size,
            cartridge_type: ty,
        })
    }
}
