extern crate alloc;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::{boxed::Box, vec, vec::Vec};

use cartridge::Cartridge;
use crankstart::file::FileSystem;
use crankstart::system::MenuItem;
use rusty_boy::RustyBoy;

use crankstart_sys::PDButtons;
use {
    crankstart::{graphics::Graphics, system::System},
    crankstart_sys::{LCD_COLUMNS, LCD_ROWS, LCD_ROWSIZE},
};

const DUMMY_BUTTON_CYCLES: usize = 30;

static PRESS_START: AtomicBool = AtomicBool::new(false);
static PRESS_SELECT: AtomicBool = AtomicBool::new(false);
static TERMINATE: AtomicBool = AtomicBool::new(false);

fn setup_menu_items(system: &System) -> Result<Vec<MenuItem>, anyhow::Error> {
    Ok(vec![
        system.add_menu_item(
            "start button",
            Box::new(|| PRESS_START.store(true, Ordering::Relaxed)),
        )?,
        system.add_menu_item(
            "select button",
            Box::new(|| PRESS_SELECT.store(true, Ordering::Relaxed)),
        )?,
        system.add_menu_item(
            "game selector",
            Box::new(|| TERMINATE.store(true, Ordering::Relaxed)),
        )?,
    ])
}

pub struct GameRunner {
    rusty_boy: RustyBoy,
    select_cycles: usize,
    start_cycles: usize,
    _menu_items: Vec<MenuItem>,
}

impl GameRunner {
    pub fn new(
        fs: &FileSystem,
        graphics: &Graphics,
        system: &System,
        rom: Vec<u8>,
    ) -> Result<Self, anyhow::Error> {
        let file = fs.open("readme.txt", crankstart_sys::FileOptions::kFileWrite)?;
        file.write("Put roms here".as_bytes())?;
        drop(file);

        let cartridge = Cartridge::new(rom).map_err(|e| anyhow::format_err!("{e:?}"))?;
        let mut rusty_boy = RustyBoy::new_with_cartridge(cartridge);
        rusty_boy.configure_cpu_step(sm83::core::Cycles::new(60));
        graphics.clear(crankstart::graphics::LCDColor::Solid(
            crankstart_sys::LCDSolidColor::kColorWhite,
        ))?;

        PRESS_START.store(false, Ordering::Relaxed);
        PRESS_SELECT.store(false, Ordering::Relaxed);
        TERMINATE.store(false, Ordering::Relaxed);
        let menu_items = setup_menu_items(system)?;

        Ok(Self {
            rusty_boy,
            select_cycles: 0,
            start_cycles: 0,
            _menu_items: menu_items,
        })
    }

    pub fn update(&mut self) -> Result<bool, anyhow::Error> {
        if TERMINATE.load(Ordering::Relaxed) {
            return Ok(true);
        }

        if PRESS_SELECT.swap(false, Ordering::Relaxed) {
            self.select_cycles = DUMMY_BUTTON_CYCLES;
        }
        if PRESS_START.swap(false, Ordering::Relaxed) {
            self.start_cycles = DUMMY_BUTTON_CYCLES;
        }

        let mut joypad_state = rusty_boy::joypad::State::new();
        let (current, _, _) = System::get().get_button_state()?;
        if (current & PDButtons::kButtonA).0 != 0 {
            joypad_state.a = true;
        }
        if (current & PDButtons::kButtonB).0 != 0 {
            joypad_state.b = true;
        }
        if (current & PDButtons::kButtonLeft).0 != 0 {
            joypad_state.left = true;
        }
        if (current & PDButtons::kButtonRight).0 != 0 {
            joypad_state.right = true;
        }
        if (current & PDButtons::kButtonUp).0 != 0 {
            joypad_state.up = true;
        }
        if (current & PDButtons::kButtonDown).0 != 0 {
            joypad_state.down = true;
        }
        if self.select_cycles > 0 {
            self.select_cycles -= 1;
            joypad_state.select = true;
        }
        if self.start_cycles > 0 {
            self.start_cycles -= 1;
            joypad_state.start = true;
        }

        self.rusty_boy.update_keys(&joypad_state);

        // We run two frames back to back, only rendering one of them. This trick allows us to get
        // lower framerate, but better overall speed.
        self.rusty_boy.run_until_next_frame(false);
        let frame = self.rusty_boy.run_until_next_frame(true);

        let graphics = Graphics::get();
        let target = graphics.get_frame()?;

        let x_offset = (LCD_COLUMNS as usize - 160) / 2;
        let y_offset = (LCD_ROWS as usize - 144) / 2;

        for (y, line) in frame.iter().enumerate() {
            let target_line_offset = (y_offset + y) * LCD_ROWSIZE as usize;
            let mut byte = 0;
            for (x, pixel) in line.iter().enumerate() {
                match *pixel {
                    ppu::Color::White | ppu::Color::LightGrey => {
                        byte |= 1 << (7 - (x % 8));
                    }
                    _ => {}
                }

                if x % 8 == 7 {
                    target[target_line_offset + (x_offset + x) / 8] = byte;
                    byte = 0;
                }
            }
        }

        graphics.mark_updated_rows((y_offset as i32)..=(y_offset + 144) as i32)?;

        crankstart::system::System::get().draw_fps(0, 0)?;
        Ok(false)
    }
}
