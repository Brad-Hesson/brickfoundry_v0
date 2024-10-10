#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing;

mod components;
mod pages;

fn main() {
    // Init logger
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    tracing::info!("starting app");

    #[cfg(feature = "web")]
    dioxus::launch(App);

    #[cfg(feature = "server")]
    {
        use axum::Router;
        use axum_server::tls_rustls::RustlsConfig;
        use rcgen::generate_simple_self_signed;
        use std::path::PathBuf;

        let subject_alt_names = vec!["localhost".to_string()];

        let cert = generate_simple_self_signed(subject_alt_names).unwrap();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let config = RustlsConfig::from_pem_file(
                PathBuf::new().join("cert/cert.pem"),
                PathBuf::new().join("cert/key.pem"),
            )
            .await
            .unwrap();

            let app = Router::new()
                .serve_dioxus_application(ServeConfig::builder().build(), || VirtualDom::new(App))
                .await;
            let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));
            axum_server::bind_rustls(addr, config)
                .serve(app.into_make_service())
                .await
                .unwrap();
        });
    }
}

fn App() -> Element {
    rsx! {
        Router::<pages::Pages> {}
    }
}
