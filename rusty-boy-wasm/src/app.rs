use wasm_bindgen::JsValue;
use yew::prelude::*;

use crate::game::Game;
use crate::icon::ColorThemeIcon;
use crate::loader::Loader;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Theme {
    Light,
    Dark,
}

impl TryFrom<String> for Theme {
    type Error = Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match &value as &str {
            "light" => Ok(Theme::Light),
            "dark" => Ok(Theme::Dark),
            theme => Err(Error::AppError(Some(format!("Unknown theme: {theme}")))),
        }
    }
}

impl From<Theme> for &str {
    fn from(value: Theme) -> Self {
        match value {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

impl Theme {
    pub fn opposite(&self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("An application specific error happened {0:?}")]
    AppError(Option<String>),

    #[error("A javascript error happened: {0:?}")]
    JsError(JsValue),
}

type Result<T> = std::result::Result<T, Error>;

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Self::JsError(value)
    }
}

fn get_stored_theme() -> Result<Option<Theme>> {
    let local_storage = web_sys::window()
        .ok_or_else(|| Error::AppError(Some("window is not valid".to_string())))?
        .local_storage()?
        .ok_or_else(|| Error::AppError(Some("local storage is not valid".to_string())))?;

    let Some(val) = local_storage.get_item("theme")? else {
        web_sys::console::log_1(&"No valid theme stored".into());
        return Ok(None);
    };

    web_sys::console::log_1(&format!("Theme is {val}").into());

    Ok(Some(val.try_into()?))
}

fn store_theme(theme: Theme) -> Result<()> {
    let local_storage = web_sys::window()
        .ok_or_else(|| Error::AppError(Some("window is not valid".to_string())))?
        .local_storage()?
        .ok_or_else(|| Error::AppError(Some("local storage is not valid".to_string())))?;

    local_storage.set_item("theme", theme.into())?;
    web_sys::console::log_1(&format!("Saving theme {theme:?}").into());
    Ok(())
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub game_data: UseStateHandle<Option<Vec<u8>>>,
}

#[function_component(App)]
pub(super) fn app() -> Html {
    let file_data: UseStateHandle<Option<Vec<u8>>> = use_state(|| None);
    let theme = use_state(|| {
        get_stored_theme()
            .expect("Unable to get theme")
            .unwrap_or(Theme::Light)
    });

    let toggle_dark_mode = {
        let theme = theme.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let new_theme = theme.opposite();
            theme.set(new_theme);
            store_theme(new_theme).expect("Unable to save theme");
        })
    };

    html! {
        <body data-bs-theme={<Theme as Into<&str>>::into(*theme)} style="height: 100%">
            <div class="container">
                <head class="d-flex flex-wrap justify-content-center py-3 mb-4 border-bottom">
                    <div class="d-flex align-items-center mb-3 mb-md-0 me-md-auto link-body-emphasis text-decoration-none">
                        <span class="fs-4">{"Rusty Boy - Wasm Edition"}</span>
                    </div>

                    <ul class="nav nav-pills">
                        <li class="nav-item"><button aria-current="page" class="nav-link">{"Home"}</button></li>
                        <li class="nav-item"><button class="nav-link">{"Reset"}</button></li>
                        <li class="nav-item"><button class="nav-link">{"Help"}</button></li>
                        <li class="nav-item"><button class="nav-link">{"About"}</button></li>
                        <li class="nav-item"><button onclick={toggle_dark_mode} class="nav-link"><ColorThemeIcon theme={theme.opposite()}/></button></li>
                    </ul>
                </head>
            </div>

            <div class="container">
                if file_data.is_none() {
                    <Loader game_data={file_data}>
                    </Loader>
                } else {
                    <Game game_data={file_data}>
                    </Game>
                }
            </div>
        </body>
    }
}
