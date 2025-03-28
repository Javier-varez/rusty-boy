//! Implements the memory mapped interface to OAM

use core::mem::MaybeUninit;

use static_assertions::assert_eq_size;

extern crate alloc;

use alloc::boxed::Box;

use crate::vram::TileIndex;

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::{register_bitfields, registers::InMemoryRegister};

register_bitfields! [
    u8,

    /// Attributes of an 8x8 or 8x16 object
    pub OBJ_ATTRS [
        /// Determines whether the background and window color indexes 1-3 are drawn over this
        /// object
        PRIO OFFSET(7) NUMBITS(1) [
            No = 0,
            BelowBgAndWindow = 1,
        ],

        /// Determines whether the object is drawn normally or flipped around the Y axis
        Y_FLIP OFFSET(6) NUMBITS(1) [
            No = 0,
            Yes = 1,
        ],

        /// Determines whether the object is drawn normally or flipped around the X axis
        X_FLIP OFFSET(5) NUMBITS(1) [
            No = 0,
            Yes = 1,
        ],

        /// Determines which palette must be used to draw the object
        PALETTE_SELECTOR OFFSET(4) NUMBITS(1) [
            Palette0 = 0,
            Palette1 = 1,
        ],
    ],
];

#[repr(C)]
pub struct Object {
    pub(crate) y: u8,
    pub(crate) x: u8,
    pub(crate) tile_idx: TileIndex,
    pub(crate) attrs: InMemoryRegister<u8, OBJ_ATTRS::Register>,
}

const OBJECT_SIZE: usize = 4;
assert_eq_size!([u8; OBJECT_SIZE], Object);

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}

impl Object {
    pub const fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            tile_idx: TileIndex::new(0),
            attrs: InMemoryRegister::new(0),
        }
    }

    pub fn read(&self, offset: usize) -> u8 {
        match offset {
            0 => self.y,
            1 => self.x,
            2 => self.tile_idx.into(),
            3 => self.attrs.get(),
            _ => {
                panic!("Out of range read from OAM object");
            }
        }
    }

    pub fn write(&mut self, offset: usize, value: u8) {
        match offset {
            0 => self.y = value,
            1 => self.x = value,
            2 => self.tile_idx = value.into(),
            3 => self.attrs.set(value),
            _ => {
                panic!("Out of range read from OAM object");
            }
        }
    }
}

/// The number of objects that fit in the OAM
pub const NUM_OBJECTS: usize = 40;

/// The size of the OAM RAM in bytes
pub const OAM_SIZE: usize = OBJECT_SIZE * NUM_OBJECTS;

#[repr(C)]
pub struct Oam {
    objects: Box<[Object; NUM_OBJECTS]>,
}

impl Default for Oam {
    fn default() -> Self {
        Self::new()
    }
}

impl Oam {
    const OAM_BASE: usize = 0xFE00;

    pub fn new() -> Self {
        let mut objects = [const { MaybeUninit::uninit() }; NUM_OBJECTS];

        for elem in objects.iter_mut() {
            elem.write(Object::new());
        }

        Self {
            objects: unsafe {
                Box::new(core::mem::transmute::<
                    [MaybeUninit<Object>; NUM_OBJECTS],
                    [Object; NUM_OBJECTS],
                >(objects))
            },
        }
    }

    const fn cpu_addr_to_object_addr(address: sm83::memory::Address) -> (usize, usize) {
        let address = address as usize - Self::OAM_BASE;
        (address / OBJECT_SIZE, address % OBJECT_SIZE)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Object> {
        self.objects.iter()
    }

    pub fn objects(&self) -> &[Object] {
        &*self.objects
    }

    #[cfg_attr(feature = "profile", inline(never))]
    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        let (object_idx, object_member_offset) = Self::cpu_addr_to_object_addr(address);
        self.objects[object_idx].read(object_member_offset)
    }

    #[cfg_attr(feature = "profile", inline(never))]
    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let (object_idx, object_member_offset) = Self::cpu_addr_to_object_addr(address);
        self.objects[object_idx].write(object_member_offset, value);
    }
}
