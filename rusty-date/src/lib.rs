#![no_std]

mod game_runner;
mod game_selector;

extern crate alloc;

use {
    alloc::{boxed::Box, format},
    crankstart::{
        crankstart_game,
        file::FileSystem,
        graphics::{Font, Graphics},
        system::System,
        Game, Playdate,
    },
    crankstart_sys::PDSystemEvent,
    game_runner::GameRunner,
    game_selector::GameSelector,
};

const SYSTEM_FONT: &str = "/System/Fonts/Asheville-Sans-14-Bold.pft";
const TEXT_HEIGHT: i32 = 16;

enum View {
    GameRunner(game_runner::GameRunner),
    GameSelector(game_selector::GameSelector),
}

struct State {
    view: View,
    font: Font,
}

impl State {
    pub fn new(_playdate: &Playdate) -> Result<Box<Self>, anyhow::Error> {
        let display = crankstart::display::Display::get();
        display.set_refresh_rate(30.0)?;

        let graphics = crankstart::graphics::Graphics::get();
        let font = graphics.load_font(SYSTEM_FONT)?;
        graphics.set_font(&font)?;

        Ok(Box::new(Self {
            view: View::GameSelector(GameSelector::new(&font)?),
            font,
        }))
    }
}

impl Game for State {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), anyhow::Error> {
        let mut selected_rom = None;
        let mut terminate_game = false;
        match &mut self.view {
            View::GameSelector(selector) => {
                selected_rom = selector.update(&self.font)?;
            }
            View::GameRunner(runner) => {
                terminate_game = runner.update()?;
            }
        }

        if let Some(rom) = selected_rom {
            self.view = View::GameRunner(GameRunner::new(
                &FileSystem::get(),
                &Graphics::get(),
                &System::get(),
                rom,
            )?);
        } else if terminate_game {
            self.view = View::GameSelector(GameSelector::new(&self.font)?);
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        _playdate: &mut Playdate,
        event: PDSystemEvent,
    ) -> Result<(), anyhow::Error> {
        System::log_to_console(&format!("{event:?}"));
        if event == PDSystemEvent::kEventTerminate || event == PDSystemEvent::kEventLock {
            match &mut self.view {
                View::GameRunner(runner) => {
                    runner.save_game()?;
                }
                View::GameSelector(_) => {}
            }
        }
        Ok(())
    }
}

crankstart_game!(State);
