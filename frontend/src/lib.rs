use components::*;
use wasm_bindgen::prelude::wasm_bindgen;

mod components;
mod hooks;
mod state;
mod ws;

#[wasm_bindgen(start)]
pub fn run_app() {
    #[cfg(debug_assertions)]
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
