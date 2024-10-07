#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing;

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[rustfmt::skip]
enum Route {
    #[layout(NavBar)]
        #[route("/")]
        Home {},
        #[route("/blog/:id")]
        Blog { id: i32 },
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> }
}

fn main() {
    // Init logger
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    tracing::info!("starting app");
    launch(App);
}

fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Blog(id: i32) -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go to counter" }
        "Blog post {id}"
    }
}

#[component]
fn PageNotFound(route: Vec<String> ) -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go to Homepage" }
        "Invalid link: {route:?}"
    }
}

#[component]
fn NavBar() -> Element{
    rsx!{
        div { class: "page-header",
            Link { class: "nav-bar-tab", to: Route::Home {}, "Home" }
            Link { class: "nav-bar-tab", to: Route::Home {}, "About" }
            Link { class: "nav-bar-tab", to: Route::Home {}, "Projects" }
            Link { class: "nav-bar-tab", to: Route::Home {}, "Settings" }
            Link { class: "nav-bar-tab", to: Route::Blog { id: 0 }, "Blog" }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    let mut count = use_signal(|| 0);
    let mut text = use_signal(|| String::from("..."));

    rsx! {
        Link { to: Route::Blog { id: count() }, "Go to blog" }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button {
                onclick: move |_| async move {
                    if let Ok(data) = get_server_data().await {
                        tracing::info!("Client received: {}", data);
                        text.set(data.clone());
                        post_server_data(data).await.unwrap();
                    }
                },
                "Get Server Data"
            }
            p { "Server data: {text}" }
        }
    }
}

#[server(PostServerData)]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    tracing::info!("Server received: {}", data);
    Ok(())
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok("Hello from the server!".to_string())
}
