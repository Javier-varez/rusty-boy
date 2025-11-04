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
    file.write(data)?;
    Ok(())
}

fn render_frame(graphics: &Graphics, frame: &Frame) -> Result<(), anyhow::Error> {
    let system = System::get();
    system.reset_elapsed_time()?;

    let target = graphics.get_frame()?;

    const TARGET_WIDTH: usize =
        ((LCD_ROWS as f64) * (ppu::DISPLAY_WIDTH as f64) / (ppu::DISPLAY_HEIGHT as f64)) as usize;
    const TARGET_HEIGHT: usize = LCD_ROWS as usize;

    const X_OFFSET: usize = (LCD_COLUMNS as usize - TARGET_WIDTH) / 2;

    graphics.fill_rect(
        ScreenRect {
            origin: Point2D::new(X_OFFSET as i32, 0),
            size: Size2D::new(TARGET_WIDTH as i32, TARGET_HEIGHT as i32),
        },
        LCDColor::Solid(LCDSolidColor::kColorBlack),
    )?;

    let ppu_x_offsets: heapless::Vec<(usize, u8), TARGET_WIDTH> = (0..TARGET_WIDTH)
        .map(|offset| {
            (
                (offset * ppu::DISPLAY_WIDTH + (TARGET_WIDTH / 2)) / TARGET_WIDTH,
                (offset & 1) as u8,
            )
        })
        .collect();

    const INITIAL_X_OFFSET_INNER: usize = X_OFFSET % 8;
    const PIXELS_IN_BYTE: usize = 8;

    target
        .chunks_exact_mut(LCD_ROWSIZE as usize)
        .enumerate()
        .for_each(|(y, line)| {
            let ppu_y = (y * ppu::DISPLAY_HEIGHT + (TARGET_HEIGHT / 2)) / TARGET_HEIGHT;
            let odd_y = (y & 1) as u8;
            let ppu_line = unsafe { frame.get_unchecked(ppu_y) };

            let mut x_offset_inner = INITIAL_X_OFFSET_INNER;
            let mut ppu_x_offsets_iter = ppu_x_offsets.iter();

            line.iter_mut()
                .skip(X_OFFSET / 8)
                .take((TARGET_WIDTH + INITIAL_X_OFFSET_INNER) / PIXELS_IN_BYTE)
                .for_each(|b| {
                    (x_offset_inner..PIXELS_IN_BYTE).for_each(|bit_idx| {
                        let (ppu_x, odd_x) =
                            unsafe { ppu_x_offsets_iter.next().unwrap_unchecked() };
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
                    x_offset_inner = 0;
                });
        });

    graphics.mark_updated_rows(0..=TARGET_HEIGHT as i32)?;

    let time = system.get_elapsed_time()?;
    System::log_to_console(&format!("Playdate render: {time} s"));

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

        System::get().draw_fps(0, 0)?;

        Ok(false)
    }
}
