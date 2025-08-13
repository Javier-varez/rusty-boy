use std::cell::RefCell;
use std::rc::Rc;

use base64::Engine;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::CanvasRenderingContext2d;
use web_sys::ImageData;
use yew::Component;
use yew::prelude::*;

use crate::joypad::Joypad;
use cartridge::Cartridge;
use rusty_boy::RustyBoy;

use crate::app::AppState;

pub struct Game {
    canvas: NodeRef,
    rusty_boy: RustyBoy,
    game_name: String,
    interval_handle: i32,
    _tick_closure: Closure<dyn Fn()>,
    _joypad: Joypad,
}

pub enum GameMessage {
    Init,
    Tick,
    JoypadEvent(rusty_boy::joypad::State),
}

const ALPHA: u8 = 255;
fn to_color_array(color: ppu::Color) -> [u8; 4] {
    match color {
        ppu::Color::Black => [15, 56, 15, ALPHA],
        ppu::Color::DarkGrey => [48, 98, 48, ALPHA],
        ppu::Color::LightGrey => [139, 172, 15, ALPHA],
        ppu::Color::White => [155, 188, 15, ALPHA],
    }
}

#[derive(Properties, PartialEq)]
pub struct GameProps {
    pub app_state: Rc<RefCell<AppState>>,
}

impl Component for Game {
    type Message = GameMessage;
    type Properties = GameProps;

    fn create(ctx: &Context<Self>) -> Self {
        web_sys::console::log_1(&"Game created!".into());
        let data = {
            let AppState::GameSelected { data } = &*ctx.props().app_state.borrow() else {
                panic!("Invalid state to construct a game!");
            };
            data.clone()
        };

        let cartridge = Cartridge::try_new(data)
            .map_err(|e| anyhow::format_err!("Invalid cartridge: {}", e))
            .unwrap();
        let title = cartridge.header().title.to_string();
        let rusty_boy = RustyBoy::new_with_cartridge(cartridge);
        web_sys::console::log_1(&format!("Loaded game: {title}").into());

        ctx.link().send_message(GameMessage::Init);

        let tick_closure = {
            let cb = ctx.link().callback(|_| GameMessage::Tick);
            Closure::<dyn Fn()>::wrap(Box::new(move || cb.emit(())))
        };

        let interval_handle = {
            let window = web_sys::window().unwrap();
            window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    tick_closure.as_ref().unchecked_ref(),
                    1000 / 60,
                )
                .unwrap()
        };

        *ctx.props().app_state.borrow_mut() = AppState::Running;
        Self {
            canvas: NodeRef::default(),
            rusty_boy,
            game_name: title,
            interval_handle,
            _tick_closure: tick_closure,
            _joypad: Joypad::new(ctx.link().callback(GameMessage::JoypadEvent)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            GameMessage::Init => {
                let Some(canvas): Option<web_sys::HtmlCanvasElement> = self.canvas.cast() else {
                    return true;
                };
                canvas.set_width(ppu::DISPLAY_WIDTH as u32);
                canvas.set_height(ppu::DISPLAY_HEIGHT as u32);
            }
            GameMessage::JoypadEvent(joypad_state) => {
                self.rusty_boy.update_keys(&joypad_state);
            }
            GameMessage::Tick => {
                if matches!(*ctx.props().app_state.borrow(), AppState::LoadingState) {
                    // TODO(ja):
                    self.load_state().unwrap();
                    web_sys::console::log_1(&"load success".into());
                }
                if matches!(*ctx.props().app_state.borrow(), AppState::SavingState) {
                    // TODO(ja):
                    self.save_state().unwrap();
                    web_sys::console::log_1(&"save success".into());
                }
                *ctx.props().app_state.borrow_mut() = AppState::Running;

                let frame = self.rusty_boy.run_until_next_frame(true);

                let Some(canvas): Option<web_sys::HtmlCanvasElement> = self.canvas.cast() else {
                    return true;
                };

                let ctx = canvas.get_context("2d").unwrap().unwrap();
                let ctx: &CanvasRenderingContext2d = ctx.dyn_ref().unwrap();

                let v: Vec<u8> = frame
                    .iter()
                    .flatten()
                    .flat_map(|pix| to_color_array(*pix))
                    .collect();
                let pixels = wasm_bindgen::Clamped(&v[..]);

                let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                    pixels,
                    ppu::DISPLAY_WIDTH as u32,
                    ppu::DISPLAY_HEIGHT as u32,
                )
                .unwrap();

                ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
            }
        }

        false
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        web_sys::console::log_1(&"redraw".into());
        html! {
            <div class="d-flex justify-content-center card">
                <div class="card-header">
                    {&self.game_name}
                </div>
                <div class="card-body">
                    <canvas ref={self.canvas.clone()} class="game-canvas d-flex justify-content-center "/>
                </div>
            </div>
        }
    }
}

impl Game {
    pub fn load_state(&mut self) -> crate::error::Result<()> {
        let local_storage = web_sys::window()
            .ok_or_else(|| crate::error::Error::AppError(Some("window is not valid".to_string())))?
            .local_storage()?
            .ok_or_else(|| {
                crate::error::Error::AppError(Some("local storage is not valid".to_string()))
            })?;

        let game_name = &self.game_name;
        let Some(savestate) = local_storage.get_item(&format!("game_savestate:{game_name}"))?
        else {
            web_sys::console::log_1(&"No save states available".into());
            return Ok(());
        };

        web_sys::console::log_1(&"Loading save state".into());

        if !self.rusty_boy.supports_battery_backed_ram() {
            return Ok(());
        }

        let engine = base64::engine::general_purpose::URL_SAFE;
        let ram = engine.decode(savestate).map_err(|e| {
            crate::error::Error::AppError(Some(format!("savestate is invalid base64 data: {e:?}")))
        })?;

        self.rusty_boy.reset();
        self.rusty_boy
            .restore_cartridge_ram(&ram[..])
            .map_err(|e| {
                crate::error::Error::AppError(Some(format!("Error restoring ram: {e:?}")))
            })?;

        Ok(())
    }

    pub fn save_state(&mut self) -> crate::error::Result<()> {
        let local_storage = web_sys::window()
            .ok_or_else(|| crate::error::Error::AppError(Some("window is not valid".to_string())))?
            .local_storage()?
            .ok_or_else(|| {
                crate::error::Error::AppError(Some("local storage is not valid".to_string()))
            })?;

        if !self.rusty_boy.supports_battery_backed_ram() {
            return Ok(());
        }

        let Some(ram) = self.rusty_boy.get_cartridge_ram() else {
            return Ok(());
        };

        let engine = base64::engine::general_purpose::URL_SAFE;
        let savedata = engine.encode(ram);

        let game_name = &self.game_name;
        web_sys::console::log_1(&format!("Saving state for game {game_name}").into());
        local_storage.set_item(&format!("game_savestate:{game_name}"), &savedata)?;

        Ok(())
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let window = web_sys::window().expect("Unable to get window");
        window.clear_interval_with_handle(self.interval_handle);
    }
}
