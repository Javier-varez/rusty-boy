pub type Address = u16;

pub trait Memory {
    fn read(&mut self, address: Address) -> u8;
    fn write(&mut self, address: Address, value: u8);
}
