use dioxus::prelude::*;
use dioxus_logger::tracing::info;

#[component]
pub fn Projects() -> Element {
    let projects = use_resource(|| get_project_list());
    rsx! {
        h1 { "Projects Page" }
        match projects() {
            None => rsx! {
            },
            Some(Err(e)) => rsx! {
                p { style: "color: red", "Error: {e}" }
            },
            Some(Ok(projects)) => rsx! {
                {projects.iter().map(|p| rsx!{ p { "{p}" } } )}
            },
        }
    }
}

#[server]
async fn get_project_list() -> Result<Vec<String>, ServerFnError> {
    info!("Sending Projects");
    Ok([
        "project1", "project2", "project3", "project4", "project5", "project6", "project7",
    ]
    .into_iter()
    .map(|s| s.into())
    .collect())
}
