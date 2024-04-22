use std::io::Read;
use std::path::Path;

use sm83::memory::{Address, Memory};

pub struct FileRom {
    data: Vec<u8>,
}

impl Memory for FileRom {
    fn read(&mut self, address: Address) -> u8 {
        self.data[address as usize]
    }

    fn write(&mut self, address: Address, value: u8) {
        panic!(
            "Attempted to write rom at address {}, with value {}",
            address, value
        )
    }
}

impl FileRom {
    pub fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        let mut file = std::fs::File::open(filepath)?;

        let mut data = vec![];
        file.read_to_end(&mut data)?;

        Ok(Self { data })
    }

    pub fn rom(&self) -> &[u8] {
        &self.data
    }
}
