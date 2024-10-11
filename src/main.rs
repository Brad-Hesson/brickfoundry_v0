#![allow(non_snake_case)]

use std::{
    collections::HashMap,
    convert::Infallible,
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
    sync::{Arc, Mutex},
};

use dioxus::prelude::*;
use dioxus_logger::tracing::{self, info, warn};

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
        use std::path::PathBuf;
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let config = RustlsConfig::from_pem_file(
                PathBuf::new().join("cert/ca_cert.pem"),
                PathBuf::new().join("cert/ca_key.pem"),
            )
            .await
            .unwrap();
            let ldraw_state = LDrawState {
                lib: Arc::new(Mutex::new(PartLibrary::new("dist/complete.zip").unwrap())),
            };
            let app = Router::new()
                .serve_dioxus_application(ServeConfig::builder().build(), || VirtualDom::new(App))
                .await
                .layer(axum::Extension(ldraw_state));
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

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct LDrawState {
    lib: Arc<Mutex<PartLibrary>>,
}

#[cfg(feature = "server")]
#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for LDrawState
where
    S: std::marker::Sync + std::marker::Send,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts.extensions.get::<LDrawState>().cloned().unwrap())
    }
}

#[test]
fn feature() {
    let mut parts = PartLibrary::new("assets/complete.zip").unwrap();
    let mut part = parts.get_part_compressed("8\\4-4cyli.dat").unwrap();
    dbg!(part);
}

#[cfg(feature = "server")]
#[derive(Debug)]
pub struct PartLibrary {
    archive: zip::ZipArchive<BufReader<File>>,
    part_map: HashMap<String, usize>,
}
#[cfg(feature = "server")]
impl PartLibrary {
    pub fn new(archive_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(archive_path)?;
        let reader = std::io::BufReader::new(file);
        let archive = zip::ZipArchive::new(reader)?;
        let mut part_map = HashMap::new();
        for p in archive.file_names() {
            let ind = archive.index_for_name(p).unwrap();
            let part_name = p
                .trim_start_matches("ldraw/")
                .trim_start_matches("parts/")
                .trim_start_matches("p/")
                .trim_start_matches("models/");
            if part_map.insert(part_name.into(), ind).is_some() {
                warn!("Duplicate part name in ldraw lib: `{part_name}`");
            }
        }
        info!("Loaded {} part files", part_map.len());
        Ok(Self { archive, part_map })
    }
    pub fn get_part(&mut self, name: impl AsRef<Path>) -> anyhow::Result<zip::read::ZipFile> {
        let path = name.as_ref().to_str().unwrap().replace("\\", "/");
        let ind = self.part_map.get(&path.to_string()).unwrap();
        let part = self.archive.by_index(*ind)?;
        Ok(part)
    }
}
