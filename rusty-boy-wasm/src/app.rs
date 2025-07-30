use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::FileReader;
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

pub enum AppMessage {}

pub(super) struct App {
    app_state: Rc<RefCell<AppState>>,
    theme: Rc<RefCell<Theme>>,
    on_load_file: Rc<RefCell<Closure<dyn Fn()>>>,
    file_reader: Rc<RefCell<FileReader>>,
}

impl Component for App {
    type Message = AppMessage;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let app_state = Rc::new(RefCell::new(AppState::Idle));
        let theme = Rc::new(RefCell::new(
            get_stored_theme()
                .expect("Unable to get theme")
                .unwrap_or(Theme::Light),
        ));

        let file_reader = Rc::new(RefCell::new(
            FileReader::new().expect("Unable to create file reader"),
        ));

        let on_load_file = Rc::new(RefCell::new({
            let file_reader = file_reader.clone();
            let app_state = app_state.clone();
            Closure::<dyn Fn()>::wrap(Box::new(move || {
                let data = file_reader.borrow_mut().result().unwrap();
                let data: &web_sys::js_sys::ArrayBuffer = data.unchecked_ref();
                let data = web_sys::js_sys::Uint8Array::new(data);
                let data = data.to_vec();
                *app_state.borrow_mut() = AppState::GameSelected { data };
            }))
        }));

        // let app_state = app_state.clone();
        // Callback::from(move |e: InputEvent| {
        //     e.prevent_default();

        //     let target = e.target().unwrap();
        //     let input_ref = target.unchecked_ref::<web_sys::HtmlInputElement>();

        //     if let Some(files) = input_ref.files() {
        //         app_state.set(crate::app::AppState::Idle);
        //         let file = files.item(0);
        //         file_reader.set_onload(Some(onload_file.as_ref().unchecked_ref()));
        //         file_reader
        //             .read_as_array_buffer(file.as_ref().unwrap())
        //             .unwrap();
        //     }
        // })

        Self {
            app_state,
            theme,
            on_load_file,

            file_reader,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let is_idle = *self.app_state.borrow() == AppState::Idle;

        let toggle_dark_mode = {
            let theme = self.theme.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                let new_theme = theme.borrow().opposite();
                *theme.borrow_mut() = new_theme;
                store_theme(new_theme).expect("Unable to save theme");
            })
        };

        let open_file = {
            let app_state = self.app_state.clone();
            let file_reader = self.file_reader.clone();
            let on_load_file = self.on_load_file.clone();
            Callback::from(move |e: InputEvent| {
                e.prevent_default();

                let target = e.target().unwrap();
                let input_ref = target.unchecked_ref::<web_sys::HtmlInputElement>();

                if let Some(files) = input_ref.files() {
                    *app_state.borrow_mut() = AppState::Idle;
                    let file = files.item(0);
                    let file_reader = file_reader.borrow_mut();
                    file_reader.set_onload(Some(on_load_file.borrow().as_ref().unchecked_ref()));
                    file_reader
                        .read_as_array_buffer(file.as_ref().unwrap())
                        .unwrap();
                }
            })
        };

        html! {
            <body data-bs-theme={<Theme as Into<&str>>::into(*self.theme.borrow())} style="height: 100%">
                <Header toggle_dark_mode={toggle_dark_mode} theme={*self.theme.borrow()} open_file={open_file} />

                <div class="container">
                    if is_idle {
                        // <Loader game_data={file_data}>
                        // </Loader>
                    } else {
                        <Game app_state={self.app_state.clone()}>
                        </Game>
                    }
                </div>
            </body>
        }
    }
}

// #[function_component(App)]
// pub(super) fn app() -> Html {
//     let app_state = use_state(|| AppState::Idle);

//     let theme = use_state(|| {
//         get_stored_theme()
//             .expect("Unable to get theme")
//             .unwrap_or(Theme::Light)
//     });

//     let toggle_dark_mode = {
//         let theme = theme.clone();
//         Callback::from(move |e: MouseEvent| {
//             e.prevent_default();
//             let new_theme = theme.opposite();
//             theme.set(new_theme);
//             store_theme(new_theme).expect("Unable to save theme");
//         })
//     };

//     let open_file = {
//         let file_reader = web_sys::FileReader::new().unwrap();
//         let onload_file = use_state(|| {
//             let file_reader = file_reader.clone();
//             let app_state = app_state.clone();
//             Closure::<dyn Fn()>::wrap(Box::new(move || {
//                 let data = file_reader.result().unwrap();
//                 let data: &web_sys::js_sys::ArrayBuffer = data.unchecked_ref();
//                 let data = web_sys::js_sys::Uint8Array::new(data);
//                 let data = data.to_vec();
//                 app_state.set(AppState::GameSelected { data });
//             }))
//         });

//         let app_state = app_state.clone();
//         Callback::from(move |e: InputEvent| {
//             e.prevent_default();

//             let target = e.target().unwrap();
//             let input_ref = target.unchecked_ref::<web_sys::HtmlInputElement>();

//             if let Some(files) = input_ref.files() {
//                 app_state.set(crate::app::AppState::Idle);
//                 let file = files.item(0);
//                 file_reader.set_onload(Some(onload_file.as_ref().unchecked_ref()));
//                 file_reader
//                     .read_as_array_buffer(file.as_ref().unwrap())
//                     .unwrap();
//             }
//         })
//     };

//     html! {
//         <body data-bs-theme={<Theme as Into<&str>>::into(*theme)} style="height: 100%">
//             <Header toggle_dark_mode={toggle_dark_mode} theme={*theme} open_file={open_file} />

//             <div class="container">
//                 if *app_state == AppState::Idle {
//                     // <Loader game_data={file_data}>
//                     // </Loader>
//                 } else {
//                     <Game app_state={app_state}>
//                     </Game>
//                 }
//             </div>
//         </body>
//     }
// }
