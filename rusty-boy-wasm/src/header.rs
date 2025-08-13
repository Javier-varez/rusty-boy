use yew::prelude::*;

use crate::icon::ColorThemeIcon;
use crate::modal::Modal;
use crate::theme::Theme;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub toggle_dark_mode: Callback<MouseEvent>,
    pub theme: Theme,
    pub open_file: Callback<InputEvent>,
    pub close_file: Option<Callback<MouseEvent>>,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    html! {
        <>
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
                                    <li>
                                        <button class="dropdown-item" onclick={props.close_file.clone()} disabled={props.close_file.is_none()}>
                                            {"Close"}
                                        </button>
                                    </li>
                                </ul>
                            </div>
                        </li>
                        <li class="nav-item">
                            <button class="nav-link" data-bs-toggle="modal" data-bs-target="#helpModal">{"Help"}</button>
                        </li>
                        <li class="nav-item">
                            <button class="nav-link" data-bs-toggle="modal" data-bs-target="#aboutModal">{"About"}</button>
                        </li>
                        <li class="nav-item"><button onclick={&props.toggle_dark_mode} class="nav-link"><ColorThemeIcon theme={props.theme.opposite()}/></button></li>
                    </ul>
                </head>
            </div>

            <Modal id={"helpModal"} title="Keybindings">
                <table class="keyboard-shortcut-help">
                    <tr>
                        <th class="keyboard-shortcut-key">{"A"}</th>
                        <td class="">{"Game Boy A button"}</td>
                    </tr>
                    <tr>
                        <th class="keyboard-shortcut-key">{"B"}</th>
                        <td class="">{"Game Boy B button"}</td>
                    </tr>
                    <tr>
                        <th class="keyboard-shortcut-key">{"↑"}</th>
                        <td class="">{"Game Boy Up keypad"}</td>
                    </tr>
                    <tr>
                        <th class="keyboard-shortcut-key">{"↓"}</th>
                        <td class="">{"Game Boy Down keypad"}</td>
                    </tr>
                    <tr>
                        <th class="keyboard-shortcut-key">{"←"}</th>
                        <td class="">{"Game Boy Left keypad"}</td>
                    </tr>
                    <tr>
                        <th class="keyboard-shortcut-key">{"→"}</th>
                        <td class="">{"Game Boy Right keypad"}</td>
                    </tr>
                    <tr>
                        <th class="keyboard-shortcut-key">{"Return"}</th>
                        <td class="">{"Game Boy start button"}</td>
                    </tr>
                    <tr>
                        <th class="keyboard-shortcut-key">{"Space"}</th>
                        <td class="">{"Game Boy select button"}</td>
                    </tr>
                </table>
            </Modal>

            <Modal id={"aboutModal"} title="About">
                <p> {"Built by "} <a href="https://github.com/javier-varez">{"javier-varez"}</a> </p>
                <p> <a href="https://allthingsembedded.com">{"AllThingsEmbedded"}</a> </p>
            </Modal>
        </>
    }
}
