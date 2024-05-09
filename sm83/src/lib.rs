//! Implementation of an SM83 CPU emulator, used in the Game Boy.
#![no_std]

pub mod core;
pub mod decoder;
pub mod interrupts;
pub mod memory;
