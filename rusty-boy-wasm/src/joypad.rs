use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;
use yew::prelude::*;

pub struct Joypad {
    keyboard_closure: Closure<dyn Fn(KeyboardEvent)>,
    _joypad_state: Rc<RefCell<rusty_boy::joypad::State>>,
}

impl Joypad {
    pub fn new(callback: Callback<rusty_boy::joypad::State>) -> Self {
        let joypad_state = Rc::new(RefCell::new(rusty_boy::joypad::State::new()));

        let keyboard_closure = {
            let joypad_state = joypad_state.clone();
            Closure::<dyn Fn(KeyboardEvent)>::wrap(Box::new(move |e| {
                let mut joypad_state = joypad_state.borrow_mut();
                let down = e.type_() == "keydown";
                match &e.key() as &str {
                    "a" => joypad_state.a = down,
                    "b" => joypad_state.b = down,
                    "Enter" => joypad_state.start = down,
                    " " => joypad_state.select = down,
                    "ArrowDown" => joypad_state.down = down,
                    "ArrowUp" => joypad_state.up = down,
                    "ArrowLeft" => joypad_state.left = down,
                    "ArrowRight" => joypad_state.right = down,
                    _ => {}
                };

                let joypad_state = joypad_state.clone();
                callback.emit(joypad_state);
            }))
        };

        let window = web_sys::window().expect("Unable to get window");
        let document = window.document().expect("Unable to get document");
        document
            .add_event_listener_with_callback("keydown", keyboard_closure.as_ref().unchecked_ref())
            .unwrap();
        document
            .add_event_listener_with_callback("keyup", keyboard_closure.as_ref().unchecked_ref())
            .unwrap();

        Self {
            keyboard_closure,
            _joypad_state: joypad_state,
        }
    }
}

impl Drop for Joypad {
    fn drop(&mut self) {
        let window = web_sys::window().expect("Unable to get window");
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
