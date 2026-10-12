//! Chart has no fill and no frame (#131): its root and its native plot were both painted
//! with `surface`, so on a raised layer (Dialog, Sheet and Popover are surface_raised, as
//! are docked tool panels) every chart was a darker framed block.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, rgba};
use gpui_rhai::*;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), source)]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_CHART_SOURCES_BY_ID.iter())
    {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .motion_preference(MotionPreference::None)
    .asset_sources(
        gpui_rhai_registry::BUNDLED_ASSET_SOURCES
            .iter()
            .map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }),
    )
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("table", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("table"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

/// A line chart on a `surface_raised` box; `root` is appended to the chart's own style.
fn script(root: &str) -> String {
    format!(
        r#"import "charts/chart" as chart;
fn view(ctx){{box([
    chart::Chart(#{{key:"c",key_dimension:"id",data:[#{{id:"a",x:0,y:0}},#{{id:"b",x:1,y:1}}],
        spec:#{{title:"Control",series:[#{{key:"s",kind:"line",encode:#{{x:"x",y:"y"}}}}]}}}})
        .with_style(style().width(px(300)).height(px(160)){root})
]).with_style(style().width(px(320)).height(px(200)).background(theme_color("surface_raised")))}}"#
    )
}

/// Painted quads filled with `color`, as (width, height) in scaled pixels.
fn filled(visual: &mut VisualTestContext, color: u32) -> Vec<(f32, f32)> {
    let fill: gpui::Background = rgba(color).into();
    visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.background == fill)
            .map(|quad| (quad.bounds.size.width.0, quad.bounds.size.height.0))
            .collect()
    })
}

/// Painted quads with a border in `color`.
fn framed(visual: &mut VisualTestContext, color: u32) -> usize {
    let border: gpui::Hsla = rgba(color).into();
    visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.border_color == border && quad.border_widths.top.0 > 0.0)
            .count()
    })
}

// default_dark: surface 0x151412, surface_raised 0x1d1c19, border 0x3a3833.
const SURFACE: u32 = 0x1514_12ff;
const SURFACE_RAISED: u32 = 0x1d1c_19ff;
const BORDER: u32 = 0x3a38_33ff;

#[gpui::test]
fn a_chart_on_a_raised_layer_has_no_block_of_another_surface(cx: &mut TestAppContext) {
    let (mut visual, _view) = mount(cx, script(""));
    let raised = filled(&mut visual, SURFACE_RAISED);
    assert!(!raised.is_empty(), "the raised layer was painted");
    let surface = filled(&mut visual, SURFACE);
    assert!(
        surface.is_empty(),
        "nothing on the raised layer should be painted with `surface`, got {surface:?}"
    );
    assert_eq!(framed(&mut visual, BORDER), 0, "the chart draws no frame");
}

#[gpui::test]
fn a_root_style_fills_and_frames_the_chart_once(cx: &mut TestAppContext) {
    let (mut visual, _view) = mount(
        cx,
        script(
            r#".background(theme_color("surface")).border(px(1)).border_color(theme_color("border"))"#,
        ),
    );
    // The root paints the fill and the frame (GPUI paints a border as its own quads); the
    // native plot inside it adds no fill.
    assert_eq!(filled(&mut visual, SURFACE), vec![(600.0, 320.0)]);
    assert!(
        framed(&mut visual, BORDER) > 0,
        "the root frame was painted"
    );
}
