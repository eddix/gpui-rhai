//! Existing-API, bounded Host bridge for mutable application-owned images.
//!
//! The worker carries typed data only. A non-Send foreground capability owns
//! the AssetRegistry clone; namespace refresh and window repaint are separate.
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, Mutex},
    thread::{self, ThreadId},
    time::Duration,
};

use gpui::{
    App, Context, ImageSource, IntoElement, Render, TestAppContext, VisualTestContext, Window,
    WindowHandle,
};
use gpui_rhai::*;

const COVER: &str = "media/cover";

fn cover_data(color: &str) -> AssetData {
    AssetData {mime_type:"image/svg+xml".into(),bytes:format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8'><rect width='8' height='8' fill='{color}'/></svg>"
    ).into_bytes()}
}

#[derive(Clone)]
struct CoverProvider(Rc<RefCell<AssetData>>);
impl AssetProvider for CoverProvider {
    fn load(&self, name: &str) -> Result<AssetData, String> {
        if name != "cover" {
            return Err("only the registered cover fixture is available".into());
        }
        Ok(self.0.borrow().clone())
    }
}

/// App-owned typed worker result. UiValue is the existing task transport, not
/// permission to access a file or URL. Only this one AssetId is authorized.
struct PreparedCover {
    asset: AssetId,
    revision: i64,
    data: AssetData,
}
impl PreparedCover {
    fn into_message(self) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("asset".into(), UiValue::String(self.asset.as_str().into())),
            ("revision".into(), UiValue::Integer(self.revision)),
            (
                "svg".into(),
                UiValue::String(String::from_utf8(self.data.bytes).unwrap()),
            ),
        ]))
    }
}

struct PrepareCover {
    worker_thread: Arc<Mutex<Option<ThreadId>>>,
}
impl AsyncCapabilityHandler for PrepareCover {
    fn start(&mut self, method: &str, input: UiValue) -> Result<TaskWork, String> {
        if method != "prepare" || input != UiValue::Null {
            return Err("prepare accepts no input".into());
        }
        let worker_thread = Arc::clone(&self.worker_thread);
        // No AssetRegistry, GPUI context, Rc, Dynamic or FnPtr is captured.
        Ok(TaskWork::new(move || {
            *worker_thread.lock().map_err(|error| error.to_string())? =
                Some(thread::current().id());
            Ok(PreparedCover {
                asset: AssetId::parse(COVER).map_err(|error| error.to_string())?,
                revision: 2,
                data: cover_data("blue"),
            }
            .into_message())
        }))
    }
}

struct PublishCover {
    assets: AssetRegistry,
    provider: CoverProvider,
    published: Rc<Cell<i64>>,
    calls: Rc<Cell<usize>>,
    foreground: ThreadId,
}
impl CapabilityHandler for PublishCover {
    fn call(&mut self, method: &str, input: UiValue) -> Result<UiValue, String> {
        assert_eq!(
            thread::current().id(),
            self.foreground,
            "registry access must stay on the foreground"
        );
        if method != "publish" {
            return Err("unknown foreground publication method".into());
        }
        let UiValue::Map(mut fields) = input else {
            return Err("publish expects the typed result map".into());
        };
        if fields.remove("asset") != Some(UiValue::String(COVER.into())) {
            return Err("asset is outside the Host allowlist".into());
        }
        let Some(UiValue::Integer(revision)) = fields.remove("revision") else {
            return Err("missing revision".into());
        };
        let Some(UiValue::String(svg)) = fields.remove("svg") else {
            return Err("missing prepared SVG".into());
        };
        if !fields.is_empty() || svg.as_bytes() != cover_data("blue").bytes {
            return Err("fixture accepts only the bounded prepared cover data".into());
        }
        if revision <= self.published.get() {
            return Err("application rejected a stale publication".into());
        }
        let previous = self.provider.0.replace(AssetData {
            mime_type: "image/svg+xml".into(),
            bytes: svg.into_bytes(),
        });
        let count = match self.assets.refresh_namespace("media") {
            Ok(count) => count,
            Err(error) => {
                self.provider.0.replace(previous);
                return Err(error.to_string());
            }
        };
        if count != 1 {
            return Err("fixture expects one already-cached cover".into());
        }
        self.published.set(revision);
        self.calls.set(self.calls.get() + 1);
        Ok(UiValue::Integer(revision))
    }
}

fn patch_schema() -> ValueSchema {
    ValueSchema::Object {
        allow_unknown: false,
        fields: BTreeMap::from([
            (
                "asset".into(),
                ObjectField::required(ValueSchema::enumeration([COVER])),
            ),
            (
                "revision".into(),
                ObjectField::required(ValueSchema::bounded_integer(Some(1), Some(8))),
            ),
            ("svg".into(), ObjectField::required(ValueSchema::string())),
        ]),
    }
}
fn descriptor(
    id: &str,
    method: &str,
    input: ValueSchema,
    output: ValueSchema,
) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: CapabilityId::parse(id).unwrap(),
        version: semver::Version::new(1, 0, 0),
        methods: BTreeMap::from([(method.into(), CapabilityMethod { input, output })]),
    }
}

#[derive(Clone)]
struct Extension {
    assets: AssetRegistry,
    provider: CoverProvider,
    published: Rc<Cell<i64>>,
    calls: Rc<Cell<usize>>,
    worker_thread: Arc<Mutex<Option<ThreadId>>>,
}
impl ScriptViewExtension for Extension {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        // Explicitly sharing a registry is a Host choice, not an SDK promise
        // that independently prepared views automatically share asset caches.
        runtime.assets = self.assets.clone();
        runtime
            .assets
            .load_image(&AssetId::parse(COVER).unwrap())
            .map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register_async(
                descriptor(
                    "app.cover_prepare",
                    "prepare",
                    ValueSchema::Null,
                    patch_schema(),
                ),
                PrepareCover {
                    worker_thread: Arc::clone(&self.worker_thread),
                },
            )
            .map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register(
                descriptor(
                    "app.cover_publish",
                    "publish",
                    patch_schema(),
                    ValueSchema::integer(),
                ),
                PublishCover {
                    assets: runtime.assets.clone(),
                    provider: self.provider.clone(),
                    published: self.published.clone(),
                    calls: self.calls.clone(),
                    foreground: thread::current().id(),
                },
            )
            .map_err(|error| error.to_string())
    }
}

const SOURCE: &str = r#"
fn state_schema(){#{fields:#{revision:#{schema:#{type:"integer"},"default":#{type:"integer",value:1}},error:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
fn request(ctx,payload){ctx.start_task("app.cover_prepare","prepare",(),Fn("loaded"),Fn("failed"));}
fn loaded(ctx,result){let revision=ctx.call_capability("app.cover_publish","publish",result);ctx.set_state("revision",revision);}
fn failed(ctx,error){ctx.set_state("error",`${error}`);}
fn view(ctx){column([
    text("Prepare cover").test_id("prepare").accessibility_role("button").on_click(Fn("request")),
    image_source(asset("media/cover")).with_key("cover").with_style(style().width(px(32)).height(px(32))).accessibility_role("image").accessibility_label("Cover"),
    text(`${ctx.get_state("revision")}`).accessibility_role("status"),
    text(ctx.get_state("error")).test_id("error")
])}
"#;

struct Host {
    domain: ScriptViewHost,
    view: ScriptViewHandle,
    frames: Rc<Cell<usize>>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.frames.set(self.frames.get() + 1);
        self.domain.container(self.view.element().unwrap())
    }
}
fn mount(
    cx: &mut TestAppContext,
    extension: Extension,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle, Rc<Cell<usize>>) {
    let entry = ModuleId::parse("main").unwrap();
    let manifest = AppManifest::new(entry.clone())
        .with_capability("app.cover_prepare", "*")
        .unwrap()
        .with_capability("app.cover_publish", "*")
        .unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, SOURCE.into())])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .manifest(manifest)
    .extension(extension)
    .prepare()
    .unwrap();
    let capture = Rc::new(RefCell::new(None));
    let captured = capture.clone();
    let frames = Rc::new(Cell::new(0));
    let frame_count = frames.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let domain = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), domain.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host {
            domain,
            view,
            frames: frame_count,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view, frames)
}
fn extension() -> Extension {
    let assets = AssetRegistry::new();
    let provider = CoverProvider(Rc::new(RefCell::new(cover_data("red"))));
    assets.register("media", provider.clone()).unwrap();
    Extension {
        assets,
        provider,
        published: Rc::new(Cell::new(1)),
        calls: Rc::new(Cell::new(0)),
        worker_thread: Arc::new(Mutex::new(None)),
    }
}
fn content(assets: &AssetRegistry, handle: &ImageHandle) -> Arc<gpui::Image> {
    match assets.image_source(handle.opaque()).unwrap() {
        ImageSource::Image(image) => image,
        _ => panic!("fixture expects a prepared non-currentColor image"),
    }
}
fn decoded_pixels(cx: &mut TestAppContext, image: Arc<gpui::Image>) -> Vec<u8> {
    cx.update(|app| {
        image
            .to_image_data(app.svg_renderer())
            .unwrap()
            .as_bytes(0)
            .unwrap()
            .to_vec()
    })
}

fn assert_pixels(pixels: &[u8], bgra: [u8; 4]) {
    assert_eq!(
        pixels.len(),
        8 * 8 * 4,
        "fixture must decode all 8x8 pixels"
    );
    assert!(pixels.chunks_exact(4).all(|pixel| pixel == bgra));
}
fn assert_cover_geometry(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    let geometry = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("image", "Cover")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    assert!((geometry.width - 32.0).abs() < 0.01 && (geometry.height - 32.0).abs() < 0.01);
    assert!(v.update(|_, cx| view.last_error(cx).unwrap()).is_none());
}
fn publish_from_worker(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    calls: &Rc<Cell<usize>>,
) {
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::TestId {
                    id: "prepare".into(),
                },
                event: "click".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    for _ in 0..200 {
        v.run_until_parked();
        v.executor().advance_clock(Duration::from_millis(16));
        if calls.get() == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        calls.get(),
        1,
        "worker must deliver through the registered foreground capability"
    );
    v.run_until_parked();
    assert_eq!(
        v.update(|_, cx| view
            .accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("status", "2")
            .count()),
        1
    );
}

#[gpui::test]
fn task_result_publishes_changed_pixels_with_stable_asset_and_opaque_handle(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let foreground = thread::current().id();
    let extension = extension();
    let (window, view, _) = mount(cx, extension.clone(), "asset-refresh-task");
    let id = AssetId::parse(COVER).unwrap();
    let handle = extension.assets.cached_image(&id).unwrap();
    let before = content(&extension.assets, &handle);
    assert_pixels(&decoded_pixels(cx, before.clone()), [0, 0, 255, 255]);
    let mut v = VisualTestContext::from_window(*window, cx);
    assert_cover_geometry(&mut v, &view);
    publish_from_worker(&mut v, &view, &extension.calls);
    v.update(|window, _| window.refresh());
    v.run_until_parked();
    assert_cover_geometry(&mut v, &view);
    assert_eq!(extension.assets.cached_image(&id).unwrap(), handle);
    let after = content(&extension.assets, &handle);
    assert_ne!(before.id(), after.id());
    assert_pixels(&decoded_pixels(cx, after), [255, 0, 0, 255]);
    assert_ne!(extension.worker_thread.lock().unwrap().unwrap(), foreground);
    println!(
        "stable AssetId/opaque handle; new GPUI content id; decoded BGRA red -> blue; mounted 32px image geometry; TaskWork is off foreground"
    );
}

#[gpui::test]
fn shared_registry_requires_host_scheduled_foreground_window_refresh(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let extension = extension();
    let (first, first_view, first_frames) = mount(cx, extension.clone(), "asset-refresh-first");
    let (second, second_view, second_frames) = mount(cx, extension.clone(), "asset-refresh-second");
    let id = AssetId::parse(COVER).unwrap();
    let handle = extension.assets.cached_image(&id).unwrap();
    let old = content(&extension.assets, &handle).id();
    let mut first_v = VisualTestContext::from_window(*first, cx);
    publish_from_worker(&mut first_v, &first_view, &extension.calls);
    let first_count = first_frames.get();
    let second_count = second_frames.get();
    // A synchronous capability has no App argument. The embedding Host pairs
    // its publication with this foreground operation for shared-window redraw.
    cx.update(App::refresh_windows);
    cx.run_until_parked();
    assert!(
        first_frames.get() > first_count && second_frames.get() > second_count,
        "both actual Host roots must redraw"
    );
    assert_cover_geometry(&mut first_v, &first_view);
    let mut second_v = VisualTestContext::from_window(*second, cx);
    assert_cover_geometry(&mut second_v, &second_view);
    assert_eq!(
        second_v.update(|_, cx| second_view
            .accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("status", "1")
            .count()),
        1,
        "asset sharing does not share application state"
    );
    assert_eq!(extension.assets.cached_image(&id).unwrap(), handle);
    assert_ne!(content(&extension.assets, &handle).id(), old);
    assert_pixels(
        &decoded_pixels(cx, content(&extension.assets, &handle)),
        [255, 0, 0, 255],
    );
    println!(
        "explicit shared registry, two Host roots repainted via App::refresh_windows; peer app revision stayed isolated"
    );
}
