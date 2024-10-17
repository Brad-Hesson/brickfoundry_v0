#![allow(non_snake_case)]

use std::{
    collections::HashMap,
    convert::Infallible,
    fs::File,
    io::{BufReader, Read, Seek},
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::Context;
#[cfg(feature = "server")]
use axum::{extract::Host, handler::HandlerWithoutStateExt as _, response::Redirect, BoxError};
use dioxus::prelude::*;
use dioxus_logger::tracing::{self, info, warn};
use http::{StatusCode, Uri};

mod components;
mod pages;
mod server;

const USE_TLS: bool = false;

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
        use std::path::PathBuf;
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let config = RustlsConfig::from_pem_file(
                PathBuf::new().join("cert/ca_cert.pem"),
                PathBuf::new().join("cert/ca_key.pem"),
            )
            .await
            .unwrap();

            let ports = Ports {
                http: 8080,
                https: 3000,
            };

            let ldraw_state = AppState {
                ldraw_part_library: Arc::new(server::ldraw::PartLibrary::new("dist/complete.zip").unwrap()),
            };
            let app = Router::new()
                .serve_dioxus_application(ServeConfig::builder().build(), || VirtualDom::new(App))
                .await
                .layer(axum::Extension(ldraw_state));
            if USE_TLS {
                tokio::spawn(redirect_http_to_https(ports));
                let addr = std::net::SocketAddr::from(([0, 0, 0, 0], ports.https));
                axum_server::bind_rustls(addr, config)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            } else {
                let addr = std::net::SocketAddr::from(([0, 0, 0, 0], ports.http));
                axum_server::bind(addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            }
        });
    }
}

fn App() -> Element {
    rsx! {
        Router::<pages::Pages> {}
    }
}

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct AppState {
    ldraw_part_library: Arc<server::ldraw::PartLibrary>,
}

#[cfg(feature = "server")]
#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AppState
where
    S: std::marker::Sync + std::marker::Send,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts.extensions.get::<AppState>().cloned().unwrap())
    }
}

#[cfg(feature = "server")]
#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

#[cfg(feature = "server")]
async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], ports.http));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}
