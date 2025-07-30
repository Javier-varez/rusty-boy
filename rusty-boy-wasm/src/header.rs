use yew::prelude::*;

use crate::icon::ColorThemeIcon;
use crate::theme::Theme;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub toggle_dark_mode: Callback<MouseEvent>,
    pub theme: Theme,
    pub open_file: Callback<InputEvent>,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    html! {
        <div class="container">
            <head class="d-flex flex-wrap justify-content-center py-3 mb-4 border-bottom">
                <div class="d-flex align-items-center mb-3 mb-md-0 me-md-auto link-body-emphasis text-decoration-none">
                    <span class="fs-4">{"Rusty Boy - Wasm Edition"}</span>
                </div>

                <ul class="nav nav-pills">
                    <li class="nav-item">
                        <div class="dropdown">
                            <button class="nav-link dropdown-toggle" type="button" data-bs-toggle="dropdown" aria-expanded="false">
                                {"File"}
                            </button>
                            <ul class="dropdown-menu">
                                <li>
                                    <label class="dropdown-item" for="rom-selector">{"Open file"}</label>
                                    <input style="display: none" type="file" id="rom-selector" oninput={&props.open_file}/>
                                </li>
                                <li><a class="dropdown-item" href="#">{"Close"}</a></li>
                            </ul>
                        </div>
                        // <button class="nav-link">{"Home"}</button>
                    </li>
                    <li class="nav-item"><button class="nav-link">{"Reset"}</button></li>
                    <li class="nav-item"><button class="nav-link">{"Help"}</button></li>
                    <li class="nav-item"><button class="nav-link">{"About"}</button></li>
                    <li class="nav-item"><button onclick={&props.toggle_dark_mode} class="nav-link"><ColorThemeIcon theme={props.theme.opposite()}/></button></li>
                </ul>
            </head>
        </div>
    }
}
