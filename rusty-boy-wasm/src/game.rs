use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::CanvasRenderingContext2d;
use web_sys::ImageData;
use yew::Component;
use yew::prelude::*;

use cartridge::Cartridge;
use rusty_boy::RustyBoy;

use crate::app::AppState;

pub struct Game {
    canvas: NodeRef,
    rusty_boy: RustyBoy,
    joypad_state: rusty_boy::joypad::State,
    _tick_closure: Closure<dyn Fn()>,
    keyboard_closure: Closure<dyn Fn(KeyboardEvent)>,
    game_name: String,
    interval_handle: i32,
}

pub enum GameMessage {
    Init,
    Tick,
    KeyboardEvent(KeyboardEvent),
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

        let keyboard_closure = {
            let cb = ctx
                .link()
                .callback(|e: KeyboardEvent| GameMessage::KeyboardEvent(e));
            Closure::<dyn Fn(KeyboardEvent)>::wrap(Box::new(move |e| cb.emit(e)))
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
            joypad_state: Default::default(),
            _tick_closure: tick_closure,
            keyboard_closure,
            game_name: title,
            interval_handle,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            GameMessage::Init => {
                let Some(canvas): Option<web_sys::HtmlCanvasElement> = self.canvas.cast() else {
                    return true;
                };
                canvas.set_width(ppu::DISPLAY_WIDTH as u32);
                canvas.set_height(ppu::DISPLAY_HEIGHT as u32);

                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                document
                    .add_event_listener_with_callback(
                        "keydown",
                        self.keyboard_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                document
                    .add_event_listener_with_callback(
                        "keyup",
                        self.keyboard_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
            }
            GameMessage::KeyboardEvent(e) => {
                let down = e.type_() == "keydown";
                match &e.key() as &str {
                    "a" => self.joypad_state.a = down,
                    "b" => self.joypad_state.b = down,
                    "Enter" => self.joypad_state.start = down,
                    " " => self.joypad_state.select = down,
                    "ArrowDown" => self.joypad_state.down = down,
                    "ArrowUp" => self.joypad_state.up = down,
                    "ArrowLeft" => self.joypad_state.left = down,
                    "ArrowRight" => self.joypad_state.right = down,
                    _ => {}
                }
                self.rusty_boy.update_keys(&self.joypad_state);
            }
            GameMessage::Tick => {
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

impl Drop for Game {
    fn drop(&mut self) {
        let window = web_sys::window().expect("Unable to get window");
        window.clear_interval_with_handle(self.interval_handle);
        let document = window.document().expect("Unable to get document");
        document
            .remove_event_listener_with_callback(
                "keydown",
                self.keyboard_closure.as_ref().unchecked_ref(),
            )
            .unwrap();
        document
            .remove_event_listener_with_callback(
                "keyup",
                self.keyboard_closure.as_ref().unchecked_ref(),
            )
            .unwrap();
    }
}
