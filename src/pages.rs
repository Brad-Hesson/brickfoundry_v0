use dioxus::prelude::*;

use about::About;
use home::Home;
use page_not_found::PageNotFound;
use projects::Projects;

mod about;
mod home;
mod page_not_found;
mod projects;

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[rustfmt::skip]
pub enum Pages {
    #[layout(NavBar)]
        #[route("/")]
        Home,
        #[route("/about")]
        About,
        #[route("/projects")]
        Projects,
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> }
}

#[component]
fn NavBar() -> Element {
    rsx! {
        div { class: "nav-bar",
            Link { class: "nav-bar-tab", to: Pages::Home, "Home" }
            Link { class: "nav-bar-tab", to: Pages::About, "About" }
            Link { class: "nav-bar-tab", to: Pages::Projects, "Projects" }
            Link { class: "nav-bar-tab", to: Pages::Home, "Settings" }
        }
        Outlet::<Pages> {}
    }
}
