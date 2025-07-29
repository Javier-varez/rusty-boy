mod app;
mod game;
mod icon;
mod loader;

use app::App;

pub fn main() {
    yew::Renderer::<App>::new().render();
}
