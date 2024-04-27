#![no_std]

pub mod header;
pub mod mappers;

use header::CartridgeHeader;
use mappers::Mapper;

pub struct Cartridge<'a> {
    header: CartridgeHeader<'a>,
    mapper: Mapper<'a>,
}

impl<'a> Cartridge<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, header::Error> {
        let header = CartridgeHeader::new(data)?;
        let mapper = Mapper::new(&header, data);
        Ok(Self { header, mapper })
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }
}

impl<'a> sm83::memory::Memory for Cartridge<'a> {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        self.mapper.read(address)
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        self.mapper.write(address, value)
    }
}
