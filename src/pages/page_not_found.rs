use dioxus::prelude::*;

use crate::pages;

#[component]
pub fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        Link { to: pages::Pages::Home {}, "Go to Homepage" }
        "Invalid link: {route:?}"
    }
}
