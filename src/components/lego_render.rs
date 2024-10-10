use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info};
use ldraw::color::ColorCatalog;
use tokio::io::AsyncReadExt;
use web_sys::{
    wasm_bindgen::{JsCast as _, JsValue},
    HtmlCanvasElement,
};

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
    use futures_util::TryFutureExt;
    use ldraw::{
        color::ColorCatalog,
        document::MultipartDocument,
        error::ResolutionError,
        library::{CacheCollectionStrategy, FileLocation, LibraryLoader, PartCache},
        parser::{parse_color_definitions, parse_multipart_document},
        resolvers::http::HttpLoader,
        PartAlias,
    };
    use reqwest::Url;
    use web_sys::HtmlCanvasElement;
    use winit::{
        event_loop::EventLoop, platform::web::WindowBuilderExtWebSys as _, window::WindowBuilder,
    };

    use super::load_asset;

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
            info!("Loaded: {_alias}");
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
        info!("Subparts {sp:?}");
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
            let bytes = load_asset("ldraw/LDConfig.ldr".into())
                .await
                .map_err(|_| ResolutionError::FileNotFound)?;
            let colors = parse_color_definitions(&mut bytes.as_slice()).await?;
            Ok(colors)
        }

        async fn load_ref(
            &self,
            _alias: PartAlias,
            _local: bool,
            colors: &ColorCatalog,
        ) -> Result<(FileLocation, MultipartDocument), ResolutionError> {
            let path = PathBuf::new().join("ldraw/parts").join(_alias.normalized);
            let bytes = load_asset(path)
                .await
                .map_err(|_| ResolutionError::FileNotFound)?;
            let multi_doc = parse_multipart_document(&mut bytes.as_slice(), colors).await?;
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
