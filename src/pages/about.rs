use crate::components::lego_render::LegoRender;
use dioxus::prelude::*;

#[component]
pub fn About() -> Element {
    let mut count = use_signal_sync(|| 0);
    rsx! {
        h1 { "About Page" }
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        div {
            LegoRender {
                id: "lego-canvas",
                style: "width: 100%; outline: none; height: 600px; border: 1px black solid"
            }
        }
    }
}
