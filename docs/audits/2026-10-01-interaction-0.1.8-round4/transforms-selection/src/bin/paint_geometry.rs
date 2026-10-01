//! Record the real GPUI CPU paint scene; no synthetic raster or product modifications.
use gpui::{
    AppContext, Context, DevicePixels, HeadlessAppContext, HeadlessAtlas, IntoElement,
    PlatformAtlas, PlatformHeadlessRenderer, Render, Scene, Size, Window, point, px, size,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../..");
type Paths = Rc<RefCell<Vec<(f32, f32, f32, f32)>>>;
struct SceneRecorder {
    paths: Paths,
    atlas: Arc<HeadlessAtlas>,
}
impl SceneRecorder {
    fn record(&self, scene: &Scene) {
        *self.paths.borrow_mut() = scene
            .paths
            .iter()
            .map(|p| {
                (
                    p.bounds.origin.x.0,
                    p.bounds.origin.y.0,
                    p.bounds.size.width.0,
                    p.bounds.size.height.0,
                )
            })
            .collect();
    }
}
impl PlatformHeadlessRenderer for SceneRecorder {
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        _: Size<DevicePixels>,
    ) -> anyhow::Result<image::RgbaImage> {
        self.record(scene);
        Ok(image::RgbaImage::new(1, 1))
    }
    fn render_scene(&mut self, scene: &Scene, _: Size<DevicePixels>) -> anyhow::Result<()> {
        self.record(scene);
        Ok(())
    }
    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.atlas.clone()
    }
}
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
fn main() -> Result<(), String> {
    let paths: Paths = Rc::default();
    let recorded = paths.clone();
    let text_system = gpui_platform::current_platform(true).text_system();
    let mut app = HeadlessAppContext::with_platform(text_system, Arc::new(()), move || {
        Ok(Some(Box::new(SceneRecorder {
            paths: recorded.clone(),
            atlas: Arc::new(HeadlessAtlas::default()),
        })))
    });
    app.update(gpui_rhai::install);
    let mut centers = Vec::new();
    for (padding, border, angle) in [(0, 0, 0.0), (0, 0, 90.0), (20, 10, 0.0), (20, 10, 90.0)] {
        let script = format!(
            r#"
import "components/rotatable" as rot;
fn view(ctx){{rot::Rotatable(#{{key:"r",label:"Rotation",angle:{angle},pivot:#{{x:100.0,y:20.0}},part_styles:#{{content:style().width(px(200)).height(px(100)).padding(px({padding})).border(px({border}))}},content:canvas(canvas_scene([canvas_rect("pivot-marker",95.0,15.0,10.0,10.0,rgba(0xff0088ff))])).accessibility_role("image").accessibility_label("Canvas")}}).with_style(style().width(px(300)).height(px(300)))}}
"#
        );
        let entry = ModuleId::parse("main").map_err(|error| error.to_string())?;
        let prepared = EmbeddedScriptView::new(
            entry.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([
                (entry, script),
                (
                    ModuleId::parse("components/rotatable").map_err(|error| error.to_string())?,
                    std::fs::read_to_string(format!("{ROOT}/registry/components/rotatable.rhai"))
                        .map_err(|error| error.to_string())?,
                ),
            ])),
            std::fs::read_to_string(format!("{ROOT}/registry/themes/default_dark.rhai"))
                .map_err(|error| error.to_string())?,
        )
        .motion_preference(MotionPreference::None)
        .prepare()
        .map_err(|error| error.to_string())?;
        let id = format!("paint-{padding}-{angle}");
        let window = app
            .open_window(size(px(400.), px(400.)), move |window, cx| {
                cx.new(move |cx| {
                    let host = ScriptViewHost::new(&id, cx).unwrap();
                    let view = prepared
                        .mount(ScriptViewConfig::new(&id), host.clone(), window, cx)
                        .unwrap();
                    Host { host, view }
                })
            })
            .map_err(|error| error.to_string())?;
        for _ in 0..4 {
            app.run_until_parked();
            app.update_window((*window).into(), |_, window, cx| {
                window.refresh();
                window.draw(cx);
            })
            .map_err(|error| error.to_string())?;
        }
        app.capture_screenshot((*window).into())
            .map_err(|error| error.to_string())?;
        let scale = app
            .update_window((*window).into(), |_, window, _| window.scale_factor())
            .map_err(|error| error.to_string())?;
        let current = paths.borrow().clone();
        assert_eq!(current.len(), 1, "expected one actual Canvas path");
        let (x, y, w, h) = current[0];
        let center = ((x + w / 2.) / scale, (y + h / 2.) / scale);
        centers.push(center);
        println!(
            "actual GPUI paint path: padding={padding}, border={border}, angle={angle}, scaled_bounds={current:?}, logical_center={center:?}"
        );
    }
    assert_eq!(
        centers[0], centers[1],
        "positive control: unpadded Canvas pivot should stay fixed"
    );
    assert_eq!(
        centers[2], centers[3],
        "padded/bordered Canvas pivot moved in actual GPUI paint scene"
    );
    Ok(())
}
