extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};

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
    file.write(data)?;
    Ok(())
}

const BYTE_ALIGNMENT: usize = !7;

const TARGET_WIDTH: usize = {
    let size =
        ((LCD_ROWS as f64) * (ppu::DISPLAY_WIDTH as f64) / (ppu::DISPLAY_HEIGHT as f64)) as usize;
    // The width will be 2 pixels shorter than it should be, but this should make rendering nicer,
    // as we don't need to account for initial offset within a byte
    size & BYTE_ALIGNMENT
};

const TARGET_HEIGHT: usize = LCD_ROWS as usize;

const X_OFFSET: usize = ((LCD_COLUMNS as usize - TARGET_WIDTH) / 2) & BYTE_ALIGNMENT;

const PPU_X_OFFSETS: [(usize, u8); TARGET_WIDTH] = {
    let mut res = [(0, 0); TARGET_WIDTH];
    let mut offset = 0;
    while offset < TARGET_WIDTH {
        res[offset] = (
            (offset * ppu::DISPLAY_WIDTH + (TARGET_WIDTH / 2)) / TARGET_WIDTH,
            (offset & 1) as u8,
        );
        offset += 1;
    }
    res
};

const PPU_Y_OFFSETS: [(usize, u8); TARGET_HEIGHT] = {
    let mut res = [(0, 0); TARGET_HEIGHT];
    let mut offset = 0;
    while offset < TARGET_HEIGHT {
        res[offset] = (
            (offset * ppu::DISPLAY_HEIGHT + (TARGET_HEIGHT / 2)) / TARGET_HEIGHT,
            (offset & 1) as u8,
        );
        offset += 1;
    }
    res
};

fn render_frame(graphics: &Graphics, source_frame: &Frame) {
    let system = System::get();
    system.reset_elapsed_time().unwrap();

    const PIXELS_IN_BYTE: usize = 8;

    let target_framebuffer = graphics.get_frame().unwrap();
    target_framebuffer
        .chunks_exact_mut(LCD_ROWSIZE as usize)
        .zip(PPU_Y_OFFSETS)
        .for_each(|(line, (ppu_y, odd_y))| {
            // SAFETY: The PPU_Y bounds are guaranteed, as they are checked at compile time
            let ppu_line = unsafe { source_frame.get_unchecked(ppu_y) };

            let mut ppu_x_offsets_iter = PPU_X_OFFSETS.iter();

            line.iter_mut()
                .skip(X_OFFSET / PIXELS_IN_BYTE)
                .take(TARGET_WIDTH / PIXELS_IN_BYTE)
                .for_each(|b| {
                    *b = 0;

                    (0..PIXELS_IN_BYTE).for_each(|bit_idx| {
                        let (ppu_x, odd_x) =
                            // SAFETY: The PPU_X_OFFSETS array contains exactly one item for each pixel
                            // in a line. The line is located at an 8-bit-aligned boundary, and its
                            // size is a multiple of 8 bytes. These preconditions are validated right
                            // below at compile time.
                            unsafe { ppu_x_offsets_iter.next().unwrap_unchecked() };

                        const _: () = const {
                            assert!((TARGET_WIDTH % PIXELS_IN_BYTE) == 0);
                            assert!((X_OFFSET % PIXELS_IN_BYTE) == 0);
                        };

                        // SAFETY: The PPU_X bounds are guaranteed, as they are checked at compile time
                        let pixel = unsafe { ppu_line.get_unchecked(*ppu_x) };

                        let odd_pixel = odd_y | odd_x;
                        let on = match pixel {
                            ppu::Color::White => 1,
                            ppu::Color::LightGrey => odd_pixel,
                            ppu::Color::DarkGrey => 1 - odd_pixel,
                            ppu::Color::Black => 0,
                        };

                        let target_bit = PIXELS_IN_BYTE - 1 - bit_idx;
                        *b |= on << target_bit;
                    });
                });
        });

    graphics
        .mark_updated_rows(0..=TARGET_HEIGHT as i32)
        .unwrap();

    let time = system.get_elapsed_time().unwrap();
    System::log_to_console(&format!("Rendering took: {time}"));
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

    pub fn update(&mut self) -> bool {
        if TERMINATE.load(Ordering::Relaxed) {
            self.save_game().unwrap();
            return true;
        }

        if SELECT_BUTTON.swap(false, Ordering::Relaxed) {
            self.select_cycles = DUMMY_BUTTON_CYCLES;
        }
        if START_BUTTON.swap(false, Ordering::Relaxed) {
            self.start_cycles = DUMMY_BUTTON_CYCLES;
        }

        let mut joypad_state = rusty_boy::joypad::State::new();
        let (current, _, _) = System::get().get_button_state().unwrap();
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
        render_frame(&graphics, frame);

        System::get().draw_fps(0, 0).unwrap();

        false
    }
}
