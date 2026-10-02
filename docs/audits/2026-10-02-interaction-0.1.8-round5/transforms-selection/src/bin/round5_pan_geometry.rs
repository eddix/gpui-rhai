//! Record the real GPUI CPU paint scene; no synthetic raster or product modifications.
use gpui::{
    AppContext, Context, DevicePixels, HeadlessAppContext, HeadlessAtlas, InputEvent, IntoElement,
    Modifiers, PlatformAtlas, PlatformHeadlessRenderer, Render, Scene, ScrollDelta,
    ScrollWheelEvent, Size, TouchPhase, Window, point, px, size,
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

fn redraw(app: &mut HeadlessAppContext, window: gpui::AnyWindowHandle) -> Result<(), String> {
    for _ in 0..4 {
        app.run_until_parked();
        app.update_window(window, |_, window, cx| {
            window.refresh();
            let _ = window.draw(cx);
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn center(
    app: &mut HeadlessAppContext,
    window: gpui::AnyWindowHandle,
    paths: &Paths,
) -> Result<(f32, f32), String> {
    app.capture_screenshot(window).map_err(|e| e.to_string())?;
    let scale = app
        .update_window(window, |_, window, _| window.scale_factor())
        .map_err(|e| e.to_string())?;
    let p = paths.borrow();
    assert_eq!(p.len(), 1);
    let (x, y, w, h) = p[0];
    Ok(((x + w / 2.) / scale, (y + h / 2.) / scale))
}
fn main() -> Result<(), String> {
    let paths: Paths = Rc::default();
    let recorded = paths.clone();
    let mut app = HeadlessAppContext::with_platform(
        gpui_platform::current_platform(true).text_system(),
        Arc::new(()),
        move || {
            Ok(Some(Box::new(SceneRecorder {
                paths: recorded.clone(),
                atlas: Arc::new(HeadlessAtlas::default()),
            })))
        },
    );
    app.update(gpui_rhai::install);
    let mut drift = Vec::new();
    for (left, right, top, bottom) in [(20, 20, 20, 20), (40, 0, 20, 0)] {
        let script = format!(
            r#"
import "components/pan_zoom" as pan;
fn view(ctx){{pan::PanZoom(#{{key:"p",label:"Canvas viewport",transform:#{{x:0.0,y:0.0,scale:1.0}},wheel_zoom:"always",part_styles:#{{content:style().padding_left(px({left})).padding_right(px({right})).padding_top(px({top})).padding_bottom(px({bottom}))}},content:canvas(canvas_scene([canvas_rect("anchor-marker",80.0,40.0,10.0,10.0,rgba(0xff0088ff))]))}}).with_style(style().width(px(300)).height(px(220)))}}
"#
        );
        let entry = ModuleId::parse("main").unwrap();
        let prepared = EmbeddedScriptView::new(
            entry.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([
                (entry, script),
                (
                    ModuleId::parse("components/pan_zoom").unwrap(),
                    std::fs::read_to_string(format!("{ROOT}/registry/components/pan_zoom.rhai"))
                        .unwrap(),
                ),
            ])),
            std::fs::read_to_string(format!("{ROOT}/registry/themes/default_dark.rhai")).unwrap(),
        )
        .motion_preference(MotionPreference::None)
        .prepare()
        .map_err(|e| e.to_string())?;
        let id = format!("pan-paint-{left}-{right}-{top}-{bottom}");
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
            .map_err(|e| e.to_string())?;
        let handle = (*window).into();
        redraw(&mut app, handle)?;
        let before = center(&mut app, handle, &paths)?;
        app.update_window(handle, |_, window, cx| {
            window.dispatch_event(
                ScrollWheelEvent {
                    position: point(px(before.0), px(before.1)),
                    delta: ScrollDelta::Pixels(point(px(0.), px(-400.0_f32 * 2.0_f32.ln()))),
                    touch_phase: TouchPhase::Started,
                    modifiers: Modifiers::default(),
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
        })
        .map_err(|e| e.to_string())?;
        redraw(&mut app, handle)?;
        let after = center(&mut app, handle, &paths)?;
        let d = (after.0 - before.0, after.1 - before.1);
        println!(
            "pan/zoom actual paint: inset=({left},{right},{top},{bottom}), anchor={before:?}, after2x={after:?}, drift={d:?}"
        );
        drift.push(d);
    }
    assert!(
        drift[0].0.abs() < 1.0 && drift[0].1.abs() < 1.0,
        "symmetric control must keep anchor"
    );
    assert!(
        drift[1].0.abs() < 1.0 && drift[1].1.abs() < 1.0,
        "asymmetric content must keep same pointer anchor"
    );
    Ok(())
}
