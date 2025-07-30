use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{FileReader, HtmlInputElement};
use yew::prelude::*;

use crate::game::Game;
use crate::header::Header;
use crate::theme::{Theme, get_stored_theme, store_theme};

#[derive(PartialEq, Eq)]
pub enum AppState {
    Idle,
    GameSelected { data: Vec<u8> },
    Running,
}

#[derive(Debug)]
pub enum AppMessage {
    ToggleTheme,
    OpenEvent(HtmlInputElement),
    CloseEvent,
    FileLoaded(Vec<u8>),
}

pub(super) struct App {
    app_state: Rc<RefCell<AppState>>,
    file_reader: FileReader,
    theme: Theme,
    _on_load_file: Closure<dyn Fn()>,
}

impl Component for App {
    type Message = AppMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let app_state = Rc::new(RefCell::new(AppState::Idle));
        let theme = get_stored_theme()
            .expect("Unable to get theme")
            .unwrap_or(Theme::Light);

        let file_reader = FileReader::new().expect("Unable to create file reader");

        let on_load_file = {
            let file_reader = file_reader.clone();
            let cb = ctx
                .link()
                .callback(|data: Vec<u8>| AppMessage::FileLoaded(data));

            Closure::<dyn Fn()>::wrap(Box::new(move || {
                let data = file_reader.result().unwrap();
                let data: &web_sys::js_sys::ArrayBuffer = data.unchecked_ref();
                let data = web_sys::js_sys::Uint8Array::new(data);
                let data = data.to_vec();
                cb.emit(data);
            }))
        };

        file_reader.set_onload(Some(on_load_file.as_ref().unchecked_ref()));

        Self {
            app_state,
            file_reader,
            theme,
            _on_load_file: on_load_file,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMessage::ToggleTheme => {
                let new_theme = self.theme.opposite();
                self.theme = new_theme;
                store_theme(new_theme).expect("Unable to save theme");
            }
            AppMessage::OpenEvent(input_ref) => {
                if let Some(files) = input_ref.files() {
                    *self.app_state.borrow_mut() = AppState::Idle;

                    let file = files.item(0);
                    self.file_reader
                        .read_as_array_buffer(file.as_ref().unwrap())
                        .unwrap();
                }
            }
            AppMessage::FileLoaded(data) => {
                *self.app_state.borrow_mut() = AppState::GameSelected { data };
            }
            AppMessage::CloseEvent => {
                *self.app_state.borrow_mut() = AppState::Idle;
            }
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let is_idle = *self.app_state.borrow() == AppState::Idle;

        let toggle_dark_mode = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            AppMessage::ToggleTheme
        });

        let emit_open_event = ctx.link().callback(|e: InputEvent| {
            e.prevent_default();
            let target = e.target().unwrap();
            let input_ref = target.unchecked_ref::<web_sys::HtmlInputElement>();
            AppMessage::OpenEvent(input_ref.clone())
        });

        let emit_close_event =
            (!is_idle).then_some(ctx.link().callback(|_| AppMessage::CloseEvent));

        html! {
            <body data-bs-theme={<Theme as Into<&str>>::into(self.theme)} style="height: 100%">
                <Header toggle_dark_mode={toggle_dark_mode} theme={self.theme} open_file={emit_open_event} close_file={emit_close_event}/>

                <div class="container">
                    if !is_idle {
                        <Game app_state={self.app_state.clone()}>
                        </Game>
                    }
                </div>
            </body>
        }
    }
}
