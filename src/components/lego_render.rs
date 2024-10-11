use std::{
    io::{BufReader, Read},
    path::PathBuf,
};

use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info};
use futures_util::stream;
use server_fn::codec::{ByteStream, Streaming};
#[cfg(feature = "server")]
use server_fn::response::http;
#[cfg(feature = "server")]
use tokio::io::AsyncReadExt as _;
use web_sys::{
    wasm_bindgen::{JsCast as _, JsValue},
    HtmlCanvasElement,
};

#[cfg(feature = "server")]
use crate::LDrawState;

#[derive(Props, PartialEq, Clone)]
pub struct LegoRenderProps {
    #[props(extends = GlobalAttributes)]
    global_attributes: Vec<Attribute>,
    children: Element,
}

#[component]
pub fn LegoRender(props: LegoRenderProps) -> Element {
    let on_mounted = use_element(move |element| {
        info!("Canvas mounted");
        let tag_name = format!("{:?}", element.value_of().dyn_into::<JsValue>().unwrap());
        let Ok(canvas_elem) = element.dyn_into::<HtmlCanvasElement>() else {
            error!("LegoRender: expected `JsValue(HtmlCanvasElement)` but got `{tag_name}`");
            return;
        };
        #[cfg(feature = "web")]
        use_future(move || {
            to_owned![canvas_elem];
            async {
                if let Err(e) = renderer::run(canvas_elem).await {
                    error!("LegoRender Error: {e:#}");
                }
            }
        });
    });
    rsx! {
        canvas { onmounted: on_mounted, ..props.global_attributes, {props.children} }
    }
}

fn use_element(mut f: impl FnMut(web_sys::Element) + 'static) -> impl FnMut(Event<MountedData>) {
    let mut mounted_data: Signal<Option<Event<MountedData>>> = use_signal(|| None);
    use_effect(move || {
        if let Some(mounted_data) = mounted_data() {
            f(mounted_data
                .downcast::<web_sys::Element>()
                .unwrap()
                .to_owned())
        }
    });
    move |event| mounted_data.set(Some(event))
}

#[cfg(feature = "web")]
mod renderer {
    use std::{
        cell::RefCell,
        path::PathBuf,
        rc::Rc,
        sync::{Arc, RwLock},
    };

    use dioxus_logger::tracing::{error, info};
    use futures_util::{stream, StreamExt, TryStreamExt};
    use ldraw::{
        color::ColorCatalog,
        document::MultipartDocument,
        error::ResolutionError,
        library::{CacheCollectionStrategy, FileLocation, LibraryLoader, PartCache},
        parser::{parse_color_definitions, parse_multipart_document},
        PartAlias,
    };
    use web_sys::HtmlCanvasElement;
    use winit::{
        event_loop::EventLoop, platform::web::WindowBuilderExtWebSys as _, window::WindowBuilder,
    };

    use super::load_asset;
    use super::load_ldr;

    pub async fn run(canvas_elem: HtmlCanvasElement) -> anyhow::Result<()> {
        info!("run called");
        use winit::{
            event::{Event, WindowEvent},
            platform::web::EventLoopExtWebSys,
        };
        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_canvas(Some(canvas_elem))
            .build(&event_loop)?;

        let loader = Rc::new(DioxusLoader);
        let colors = Rc::new(loader.load_colors().await?);
        info!("colors loaded");

        let mut app =
            viewer_common::App::new(Arc::new(window), loader.clone(), colors.clone(), true)
                .await
                .unwrap();
        let cache = Arc::new(RwLock::new(PartCache::new()));
        let (_file_loc, document) = loader
            .load_ref(
                PartAlias {
                    normalized: "car.ldr".into(),
                    original: "".into(),
                },
                true,
                &colors,
            )
            .await?;
        app.set_document(cache.clone(), &document, &|_alias, _err| {
            if let Err(e) = _err {
                error!("Error loading `{_alias}`: {e:?}");
            }
        })
        .await?;
        cache
            .write()
            .unwrap()
            .collect(CacheCollectionStrategy::Parts);
        let sp = app.get_subparts();
        let app = Rc::new(RefCell::new(app));

        let perf = web_sys::window().unwrap().performance().unwrap();
        let start_time = perf.now();
        info!("Event loop starting");
        event_loop.spawn(move |event, window_target| match event {
            Event::AboutToWait => {
                app.borrow().request_redraw();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Occluded(true) => window_target.exit(),
                WindowEvent::RedrawRequested => {
                    let mut app_ = app.borrow_mut();
                    app_.animate(((perf.now() - start_time) / 1000.0) as f32);
                    app_.render().unwrap();
                }
                WindowEvent::Resized(size) => {
                    app.borrow_mut().resize(size);
                }
                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Pressed,
                    button: winit::event::MouseButton::Right,
                    ..
                } => {
                    app.borrow_mut()
                        .advance(((perf.now() - start_time) / 1000.0) as f32);
                }
                event => {
                    if let Ok(mut app) = app.try_borrow_mut() {
                        app.handle_window_event(event, ((perf.now() - start_time) / 1000.0) as f32);
                    }
                }
            },
            _ => {}
        });

        Ok(())
    }

    struct DioxusLoader;

    #[async_trait::async_trait(?Send)]
    impl LibraryLoader for DioxusLoader {
        async fn load_colors(&self) -> Result<ColorCatalog, ResolutionError> {
            let mut byte_stream = load_ldr("LDConfig.ldr".into())
                .await
                .map_err(|_| ResolutionError::FileNotFound)?
                .into_inner();
            let mut buf = Vec::new();
            while let Some(frame) = byte_stream.try_next().await.unwrap() {
                buf.extend(frame);
            }
            let colors = parse_color_definitions(&mut buf.as_slice()).await?;
            Ok(colors)
        }

        async fn load_ref(
            &self,
            _alias: PartAlias,
            _local: bool,
            colors: &ColorCatalog,
        ) -> Result<(FileLocation, MultipartDocument), ResolutionError> {
            let mut byte_stream = load_ldr(_alias.normalized)
                .await
                .map_err(|_| ResolutionError::FileNotFound)?
                .into_inner();
            let mut buf = Vec::new();
            while let Some(frame) = byte_stream.try_next().await.unwrap() {
                buf.extend(frame);
            }
            dbg!(buf.len());
            let multi_doc = parse_multipart_document(&mut buf.as_slice(), colors).await?;
            Ok((FileLocation::Local, multi_doc))
        }
    }
}

#[server]
async fn load_asset(asset: PathBuf) -> Result<Vec<u8>, ServerFnError> {
    let mut path = std::env::current_dir()?;
    path.push("dist");
    path.push(&asset);
    let mut file = tokio::fs::File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;
    Ok(buf)
}

#[server(output = Streaming)]
async fn load_ldr(name: String) -> Result<ByteStream, ServerFnError> {
    let state: LDrawState = extract().await.unwrap();
    let mut buf = Vec::new();
    let mut lock = state.lib.lock()?;
    let file = lock.get_part(&name).unwrap();
    let mut reader = BufReader::new(file);
    reader.read_to_end(&mut buf)?;
    Ok(ByteStream::new(stream::once(async { Ok(buf) })))
}
