pub struct RomOnly<'a> {
    data: &'a [u8],
}

impl<'a> RomOnly<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        if (address as usize) < self.data.len() {
            self.data[address as usize]
        } else {
            0
        }
    }

    // Writes are ignored
    pub fn write(&mut self, _: sm83::memory::Address, _: u8) {}
}
