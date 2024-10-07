use dioxus::prelude::*;

#[component]
pub fn About() -> Element {
    rsx! {
        h1 { "About Page" }
    }
}