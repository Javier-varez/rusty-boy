//! Memory-mapped access via the CPU bus, with a 16-bit address space and 8-bit memory accesses

/// The Address type of the SM83 CPU, which corresponds to the 16 bits of its address space
pub type Address = u16;

/// Trait that provides a memory access interface for the CPU.
pub trait Memory {
    /// Reads the value at the given memory address
    fn read(&self, address: Address) -> u8;

    /// Writes the value at the given memory address with the given value
    fn write(&mut self, address: Address, value: u8);
}
