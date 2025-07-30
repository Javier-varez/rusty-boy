use crate::error::{Error, Result};

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

pub fn get_stored_theme() -> Result<Option<Theme>> {
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

pub fn store_theme(theme: Theme) -> Result<()> {
    let local_storage = web_sys::window()
        .ok_or_else(|| Error::AppError(Some("window is not valid".to_string())))?
        .local_storage()?
        .ok_or_else(|| Error::AppError(Some("local storage is not valid".to_string())))?;

    local_storage.set_item("theme", theme.into())?;
    web_sys::console::log_1(&format!("Saving theme {theme:?}").into());
    Ok(())
}
