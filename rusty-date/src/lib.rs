#![no_std]

extern crate alloc;

use cartridge::Cartridge;
use crankstart::log_to_console;
use crankstart_sys::PDButtons;
use rusty_boy::RustyBoy;
use {
    alloc::{boxed::Box, vec},
    anyhow::Error,
    crankstart::{
        crankstart_game,
        geometry::{ScreenPoint, ScreenVector},
        graphics::{Graphics, LCDColor, LCDSolidColor},
        system::System,
        Game, Playdate,
    },
    crankstart_sys::{LCD_COLUMNS, LCD_ROWS},
    euclid::{point2, vec2},
};

struct State<'a> {
    rusty_boy: Box<RustyBoy<'a>>,
    location: ScreenPoint,
    velocity: ScreenVector,
}

impl<'a> State<'a> {
    pub fn new(_playdate: &Playdate) -> Result<Box<Self>, Error> {
        crankstart::display::Display::get().set_refresh_rate(20.0)?;
        let fs = crankstart::file::FileSystem::get();
        let file = fs.open("readme.txt", crankstart_sys::FileOptions::kFileWrite)?;
        file.write("Put roms here".as_bytes())?;
        drop(file);

        let file = fs.open("pokemon.gb", crankstart_sys::FileOptions::kFileReadData)?;
        file.seek(0, crankstart::file::Whence::End)?;
        let size = file.tell()? as usize;
        log_to_console!("file size is {size} bytes");
        file.seek(0, crankstart::file::Whence::Set)?;
        let rom = Box::leak(Box::new(vec![0u8; size]));
        file.read(rom)?;
        log_to_console!("File is in ram");
        let cartridge = Cartridge::new(rom).map_err(|e| anyhow::format_err!("{e:?}"))?;

        Ok(Box::new(Self {
            location: point2(INITIAL_X, INITIAL_Y),
            velocity: vec2(1, 2),
            rusty_boy: Box::new(RustyBoy::new_with_cartridge(cartridge)),
        }))
    }
}

impl<'a> Game for State<'a> {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let graphics = Graphics::get();
        graphics.clear_context()?;
        graphics.clear(LCDColor::Solid(LCDSolidColor::kColorWhite))?;
        graphics.draw_text("Hello World Rust", self.location)?;

        self.location += self.velocity;

        if self.location.x < 0 || self.location.x > LCD_COLUMNS as i32 - TEXT_WIDTH {
            self.velocity.x = -self.velocity.x;
        }

        if self.location.y < 0 || self.location.y > LCD_ROWS as i32 - TEXT_HEIGHT {
            self.velocity.y = -self.velocity.y;
        }

        let (_, pushed, _) = System::get().get_button_state()?;
        if (pushed & PDButtons::kButtonA).0 != 0 {
            log_to_console!("Button A pushed");
        }

        self.rusty_boy.run_until_next_frame();

        System::get().draw_fps(0, 0)?;
        Ok(())
    }
}

const INITIAL_X: i32 = (400 - TEXT_WIDTH) / 2;
const INITIAL_Y: i32 = (240 - TEXT_HEIGHT) / 2;

const TEXT_WIDTH: i32 = 86;
const TEXT_HEIGHT: i32 = 16;

crankstart_game!(State);
