use sm83::memory::Memory;

pub struct Joypad {
    buttons: u8,
    dpad: u8,
    sel_buttons: bool,
    sel_dpad: bool,
}

impl Joypad {
    pub const fn new() -> Self {
        Self {
            // buttons are read as 0 when pressed, 1 when not pressed
            buttons: 0xf,
            dpad: 0xf,
            sel_dpad: false,
            sel_buttons: false,
        }
    }
}

impl Memory for Joypad {
    fn read(&mut self, _: sm83::memory::Address) -> u8 {
        if self.sel_dpad && self.sel_buttons {
            self.dpad & self.buttons
        } else if self.sel_dpad {
            self.dpad | 0x20
        } else if self.sel_buttons {
            self.buttons | 0x10
        } else {
            0x3f
        }
    }

    fn write(&mut self, _: sm83::memory::Address, value: u8) {
        self.sel_dpad = value & 0x10 == 0;
        self.sel_buttons = value & 0x20 == 0;
    }
}
