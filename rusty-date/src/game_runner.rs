extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};

use alloc::string::ToString;
use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
use ppu::Frame;

use {
    crankstart::{
        file::FileSystem,
        graphics::Graphics,
        system::{MenuItem, System},
    },
    crankstart_sys::{FileOptions, PDButtons, LCD_COLUMNS, LCD_ROWS, LCD_ROWSIZE},
};

use cartridge::Cartridge;
use rusty_boy::RustyBoy;

const DUMMY_BUTTON_CYCLES: usize = 30;

static BUTTON_PRESS: AtomicBool = AtomicBool::new(false);
static SAVE_GAME: AtomicBool = AtomicBool::new(false);
static TERMINATE: AtomicBool = AtomicBool::new(false);

struct MenuItems {
    buttons: MenuItem,
    _save: MenuItem,
    _choose: MenuItem,
}

fn setup_menu_items(system: &System) -> Result<MenuItems, anyhow::Error> {
    Ok(MenuItems {
        buttons: system.add_options_menu_item(
            "buttons",
            vec!["Start".to_string(), "Select".to_string()],
            Box::new(|| BUTTON_PRESS.store(true, Ordering::Relaxed)),
        )?,
        _save: system.add_menu_item(
            "save game",
            Box::new(|| SAVE_GAME.store(true, Ordering::Relaxed)),
        )?,
        _choose: system.add_menu_item(
            "choose game",
            Box::new(|| TERMINATE.store(true, Ordering::Relaxed)),
        )?,
    })
}

fn find_saved_game(fs: &FileSystem, name: &str) -> Result<Vec<u8>, anyhow::Error> {
    fs.mkdir("savegames")?;

    let path = format!("savegames/{name}.save");
    let stat = fs.stat(&path)?;
    let mut data = vec![0; stat.size as usize];
    let file = fs.open(&path, FileOptions::kFileReadData)?;
    file.read(&mut data)?;
    Ok(data)
}

fn save_game(fs: &FileSystem, name: &str, data: &[u8]) -> Result<(), anyhow::Error> {
    fs.mkdir("savegames")?;

    let path = format!("savegames/{name}.save");
    let file = fs.open(&path, FileOptions::kFileWrite)?;
    file.write(&data)?;
    Ok(())
}

fn render_frame(graphics: &Graphics, frame: &Frame) -> Result<(), anyhow::Error> {
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
    Ok(())
}

pub struct GameRunner {
    game: String,
    rusty_boy: RustyBoy,
    select_cycles: usize,
    start_cycles: usize,
    menu_items: MenuItems,
}

impl GameRunner {
    pub fn new(
        fs: &FileSystem,
        graphics: &Graphics,
        system: &System,
        rom: crate::game_selector::Rom,
    ) -> Result<Self, anyhow::Error> {
        let cartridge = Cartridge::new(rom.data).map_err(|e| anyhow::format_err!("{e:?}"))?;
        let mut rusty_boy = RustyBoy::new_with_cartridge(cartridge);
        rusty_boy.configure_cpu_step(sm83::core::Cycles::new(60));

        if let Ok(saved_game) = find_saved_game(fs, &rom.file_name) {
            rusty_boy
                .restore_cartridge_ram(&saved_game)
                .map_err(|e| anyhow::format_err!("{e:?}"))?;
        };

        graphics.clear(crankstart::graphics::LCDColor::Solid(
            crankstart_sys::LCDSolidColor::kColorWhite,
        ))?;

        BUTTON_PRESS.store(false, Ordering::Relaxed);
        SAVE_GAME.store(false, Ordering::Relaxed);
        TERMINATE.store(false, Ordering::Relaxed);
        let menu_items = setup_menu_items(system)?;

        Ok(Self {
            game: rom.file_name,
            rusty_boy,
            select_cycles: 0,
            start_cycles: 0,
            menu_items,
        })
    }

    pub fn update(&mut self) -> Result<bool, anyhow::Error> {
        if TERMINATE.load(Ordering::Relaxed) {
            if self.rusty_boy.supports_battery_backed_ram() {
                if let Some(ram) = self.rusty_boy.get_cartridge_ram() {
                    save_game(&FileSystem::get(), &self.game, ram)?;
                }
            }
            return Ok(true);
        }

        if SAVE_GAME.swap(false, Ordering::Relaxed) && self.rusty_boy.supports_battery_backed_ram()
        {
            if let Some(ram) = self.rusty_boy.get_cartridge_ram() {
                save_game(&FileSystem::get(), &self.game, ram)?;
            }
        }

        if BUTTON_PRESS.swap(false, Ordering::Relaxed) {
            let idx = System::get().get_menu_item_value(&self.menu_items.buttons)?;
            match idx {
                0 => self.start_cycles = DUMMY_BUTTON_CYCLES,
                1 => self.select_cycles = DUMMY_BUTTON_CYCLES,
                _ => {}
            };
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
        render_frame(&graphics, frame)?;

        Ok(false)
    }
}
