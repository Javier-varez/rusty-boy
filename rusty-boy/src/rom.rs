/// Represents an in-memory ROM file for the Game Boy
pub struct Rom<'a> {
    data: &'a [u8],
}

impl<'a> Rom<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn header(&'a self) -> Result<RomHeader<'a>, Error> {
        if self.data.len() < 0x150 {
            return Err(Error::NoHeader);
        }

        let title = &self.data[0x134..0x144];
        let title_length = title.iter().take_while(|b| **b != 0).count();
        let title = &title[..title_length];
        let title = std::str::from_utf8(title).map_err(|_| Error::InvalidTitle)?;

        let manufacturer_code = if title.len() > 11 {
            None
        } else {
            Some(&self.data[0x13f..0x143])
        };
        let cgb_flag = if title.len() > 14 {
            None
        } else {
            Some(self.data[0x143])
        };

        let licensee = if self.data[0x14B] == 0x33 {
            Licensee::New([self.data[0x144], self.data[0x145]])
        } else {
            Licensee::Old(self.data[0x14B])
        };

        let rom_size = (32 * 1024) << self.data[0x148] as usize;
        let ram_size = self.data[0x149].into();

        Ok(RomHeader {
            entrypoint: &self.data[0x100..0x104],
            logo: &self.data[0x104..0x134],
            title,
            manufacturer_code,
            cgb_flag,
            licensee,
            rom_size,
            ram_size,
        })
    }
}

pub enum Licensee {
    Old(u8),
    New([u8; 2]),
}

pub enum RamSize {
    None,
    KiloBytes8,
    KiloBytes32,
    KiloBytes128,
    KiloBytes64,
    Unknown(u8),
}

impl std::fmt::Display for RamSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc1RamBattery,
    Mbc2,
    Mbc2Battery,
    RomRam,
    RomRamBattery,
    Mmm01,
    Mmm01Ram,
    Mmm01RamBattery,
    Mbc3TimerBattery,
    Mbc3TimerRamBattery,
    Mbc3,
    Mbc3Ram,
    Mbc3RamBattery,
    Mbc5,
    Mbc5Ram,
    Mbc5RamBattery,
    Mbc5Rumble,
    Mbc5RumbleRam,
    Mbc5RumbleRamBattery,
    Mbc6,
    Mbc7SensorRumbleRamBattery,
    PocketCamera,
    BandaiTama5,
    Huc3,
    Huc1RamBattery,
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

#[derive(Debug)]
pub enum Error {
    NoHeader,
    InvalidTitle,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct RomHeader<'a> {
    pub entrypoint: &'a [u8],
    pub logo: &'a [u8],
    pub title: &'a str,
    pub manufacturer_code: Option<&'a [u8]>,
    pub cgb_flag: Option<u8>,
    pub licensee: Licensee,
    pub rom_size: usize,
    pub ram_size: RamSize,
}