use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::js_sys;
use yew::prelude::*;

use crate::app::Props;

#[function_component(Loader)]
pub fn loader(props: &Props) -> Html {
    let input_ref = use_node_ref();

    let action = {
        let file_reader = web_sys::FileReader::new().unwrap();
        let onload_file = {
            let file_reader = file_reader.clone();
            let game_data = props.game_data.clone();
            Closure::<dyn Fn()>::wrap(Box::new(move || {
                let data = file_reader.result().unwrap();
                let data: &js_sys::ArrayBuffer = data.unchecked_ref();
                let data = js_sys::Uint8Array::new(data);
                let data = data.to_vec();
                game_data.set(Some(data));
            }))
        };

        let input_ref = input_ref.clone();

        Callback::from(move |e: InputEvent| {
            e.prevent_default();

            let input_ref = input_ref
                .cast::<web_sys::HtmlInputElement>()
                .expect("div_ref not attached to div element");

            if let Some(files) = input_ref.files() {
                let file = files.item(0);
                file_reader.set_onload(Some(onload_file.as_ref().unchecked_ref()));
                file_reader
                    .read_as_array_buffer(file.as_ref().unwrap())
                    .unwrap();
            }
        })
    };

    html! {
        <>
        <div class="mx-5">
            <div class="input-group mb-3">
                <input type="file" id="rom" name="rom" ref={input_ref} class="form-control" oninput={action}/>
            </div>

            <div>
            </div>
        </div>
        </>
    }
}
