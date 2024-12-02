use components::*;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_logger;
use yew::Renderer;

mod components;
mod hooks;
mod ws;

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    Renderer::<App>::new().render();
}
