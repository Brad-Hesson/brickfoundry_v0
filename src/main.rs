#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing;

mod pages;

fn main() {
    // Init logger
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    tracing::info!("starting app");
    launch(App);
}

fn App() -> Element {
    rsx! {
        Router::<pages::Pages> {}
    }
}
