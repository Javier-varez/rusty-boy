use wasm_bindgen::JsValue;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("An application specific error happened {0:?}")]
    AppError(Option<String>),

    #[error("A javascript error happened: {0:?}")]
    JsError(JsValue),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Self::JsError(value)
    }
}
