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
    pub fn update_buttons(&mut self, state: &State) {
        let to_bit = |val: bool, bit: usize| -> u8 {
            if val {
                !(1 << bit) & 0xf
            } else {
                0x0f
            }
        };

        self.buttons = to_bit(state.a, 0)
            & to_bit(state.b, 1)
            & to_bit(state.select, 2)
            & to_bit(state.start, 3);

        self.dpad = to_bit(state.right, 0)
            & to_bit(state.left, 1)
            & to_bit(state.up, 2)
            & to_bit(state.down, 3);
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

pub struct State {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
}

impl State {
    pub fn new() -> Self {
        State {
            left: false,
            right: false,
            up: false,
            down: false,
            a: false,
            b: false,
            start: false,
            select: false,
        }
    }
}
