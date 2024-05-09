extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};

use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
use crankstart::geometry::ScreenRect;
use crankstart::graphics::LCDColor;
use crankstart_sys::LCDSolidColor;
use euclid::{Point2D, Size2D};
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

static TERMINATE: AtomicBool = AtomicBool::new(false);
static SELECT_BUTTON: AtomicBool = AtomicBool::new(false);
static START_BUTTON: AtomicBool = AtomicBool::new(false);

struct MenuItems {
    _choose: MenuItem,
    _select_button: MenuItem,
    _start_button: MenuItem,
}

fn setup_menu_items(system: &System) -> Result<MenuItems, anyhow::Error> {
    Ok(MenuItems {
        _choose: system.add_menu_item(
            "choose game",
            Box::new(|| TERMINATE.store(true, Ordering::Relaxed)),
        )?,
        _select_button: system.add_menu_item(
            "select",
            Box::new(|| SELECT_BUTTON.store(true, Ordering::Relaxed)),
        )?,
        _start_button: system.add_menu_item(
            "start",
            Box::new(|| START_BUTTON.store(true, Ordering::Relaxed)),
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

    const TARGET_WIDTH: usize =
        ((LCD_ROWS as f64) * (ppu::DISPLAY_WIDTH as f64) / (ppu::DISPLAY_HEIGHT as f64)) as usize;
    const TARGET_HEIGHT: usize = LCD_ROWS as usize;

    let x_offset = (LCD_COLUMNS as usize - TARGET_WIDTH) / 2;
    let y_offset = (LCD_ROWS as usize - TARGET_HEIGHT) / 2;

    graphics.fill_rect(
        ScreenRect {
            origin: Point2D::new(x_offset as i32, y_offset as i32),
            size: Size2D::new(TARGET_WIDTH as i32, TARGET_HEIGHT as i32),
        },
        LCDColor::Solid(LCDSolidColor::kColorBlack),
    )?;

    for y in y_offset..(y_offset + TARGET_HEIGHT) {
        let ppu_y = ((y - y_offset) * ppu::DISPLAY_HEIGHT + (TARGET_HEIGHT / 2)) / TARGET_HEIGHT;
        let ppu_line = &frame[ppu_y];
        for x in x_offset..(x_offset + TARGET_WIDTH) {
            let ppu_x = ((x - x_offset) * ppu::DISPLAY_WIDTH + (TARGET_WIDTH / 2)) / TARGET_WIDTH;
            let pixel = ppu_line[ppu_x];

            let on = match pixel {
                ppu::Color::White => 1,
                ppu::Color::LightGrey if ((y & 1) != 0) || ((x & 1) != 0) => 1,
                ppu::Color::LightGrey => 0,
                ppu::Color::DarkGrey if ((y & 1) != 0) || ((x & 1) != 0) => 0,
                ppu::Color::DarkGrey => 1,
                ppu::Color::Black => 0,
            };

            let target_offset = (y * LCD_ROWSIZE as usize) + x / 8;
            let target_bit = 7 - (x % 8);

            target[target_offset] |= on << target_bit;
        }
    }

    graphics.mark_updated_rows((y_offset as i32)..=(y_offset + TARGET_HEIGHT) as i32)?;
    Ok(())
}

pub struct GameRunner {
    game: String,
    rusty_boy: RustyBoy,
    select_cycles: usize,
    start_cycles: usize,
    _menu_items: MenuItems,
}

impl GameRunner {
    pub fn new(
        fs: &FileSystem,
        graphics: &Graphics,
        system: &System,
        rom: crate::game_selector::Rom,
    ) -> Result<Self, anyhow::Error> {
        let cartridge = Cartridge::try_new(rom.data).map_err(|e| anyhow::format_err!("{e:?}"))?;
        let mut rusty_boy = RustyBoy::new_with_cartridge(cartridge);
        rusty_boy.configure_cpu_step(sm83::core::Cycles::new(60));

        if let Ok(saved_game) = find_saved_game(fs, &rom.file_name) {
            rusty_boy
                .restore_cartridge_ram(&saved_game)
                .map_err(|e| anyhow::format_err!("{e:?}"))?;
        };

        graphics.clear(crankstart::graphics::LCDColor::Solid(
            crankstart_sys::LCDSolidColor::kColorBlack,
        ))?;

        SELECT_BUTTON.store(false, Ordering::Relaxed);
        START_BUTTON.store(false, Ordering::Relaxed);
        TERMINATE.store(false, Ordering::Relaxed);
        let menu_items = setup_menu_items(system)?;

        Ok(Self {
            game: rom.file_name,
            rusty_boy,
            select_cycles: 0,
            start_cycles: 0,
            _menu_items: menu_items,
        })
    }

    pub fn save_game(&mut self) -> Result<(), anyhow::Error> {
        if self.rusty_boy.supports_battery_backed_ram() {
            if let Some(ram) = self.rusty_boy.get_cartridge_ram() {
                System::log_to_console("Game was saved");
                save_game(&FileSystem::get(), &self.game, ram)?;
            }
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<bool, anyhow::Error> {
        if TERMINATE.load(Ordering::Relaxed) {
            self.save_game()?;
            return Ok(true);
        }

        if SELECT_BUTTON.swap(false, Ordering::Relaxed) {
            self.select_cycles = DUMMY_BUTTON_CYCLES;
        }
        if START_BUTTON.swap(false, Ordering::Relaxed) {
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
        render_frame(&graphics, frame)?;

        Ok(false)
    }
}
