#![no_std]

extern crate alloc;

use cartridge::Cartridge;
use crankstart::log_to_console;
use crankstart_sys::PDButtons;
use rusty_boy::RustyBoy;
use {
    alloc::{boxed::Box, vec},
    anyhow::Error,
    crankstart::{crankstart_game, graphics::Graphics, system::System, Game, Playdate},
    crankstart_sys::{LCD_COLUMNS, LCD_ROWS, LCD_ROWSIZE},
};

struct State<'a> {
    rusty_boy: RustyBoy<'a>,
}

impl<'a> State<'a> {
    pub fn new(_playdate: &Playdate) -> Result<Box<Self>, Error> {
        crankstart::display::Display::get().set_refresh_rate(60.0)?;
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
            rusty_boy: RustyBoy::new_with_cartridge(cartridge),
        }))
    }
}

impl<'a> Game for State<'a> {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let mut state = rusty_boy::joypad::State::new();

        let (current, _, _) = System::get().get_button_state()?;
        if (current & PDButtons::kButtonA).0 != 0 {
            state.a = true;
        }
        if (current & PDButtons::kButtonB).0 != 0 {
            state.b = true;
        }
        if (current & PDButtons::kButtonLeft).0 != 0 {
            state.left = true;
        }
        if (current & PDButtons::kButtonRight).0 != 0 {
            state.right = true;
        }
        if (current & PDButtons::kButtonUp).0 != 0 {
            state.up = true;
        }
        if (current & PDButtons::kButtonDown).0 != 0 {
            state.down = true;
        }
        let crank = System::get().is_crank_docked()?;
        if crank {
            state.start = true;
        }

        self.rusty_boy.update_keys(&state);

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

        System::get().draw_fps(0, 0)?;
        Ok(())
    }
}

crankstart_game!(State);
