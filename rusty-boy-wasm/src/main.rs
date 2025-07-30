mod app;
mod error;
mod game;
mod header;
mod icon;
mod theme;

use app::App;

pub fn main() {
    yew::Renderer::<App>::new().render();
}
