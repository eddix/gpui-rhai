//! Table's header is unfilled like its rows (#129): on a raised layer (Dialog, Sheet and
//! Popover are surface_raised) it took `surface` and became a band across the table.

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
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID {
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

const SCRIPT: &str = r#"import "components/table" as table;
fn view(ctx){box([
    table::Table(#{key:"rows",label:"Rows",row_key:"id",height:120,
        columns:[#{key:"name",title:"Name",width:#{kind:"flex",value:1.0}}],rows:[#{id:"1",name:"one"},#{id:"2",name:"two"}]})
]).with_style(style().width(px(320)).height(px(200)).background(theme_color("surface_raised")))}"#;

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

#[gpui::test]
fn a_table_on_a_raised_layer_has_no_band_of_another_surface(cx: &mut TestAppContext) {
    let (mut visual, _view) = mount(cx, SCRIPT.to_owned());
    // default_dark: surface 0x151412, surface_raised 0x1d1c19.
    let raised = filled(&mut visual, 0x1d1c_19ff);
    assert!(!raised.is_empty(), "the raised layer was painted");
    let surface = filled(&mut visual, 0x1514_12ff);
    assert!(
        surface.is_empty(),
        "nothing on the raised layer should be painted with `surface`, got {surface:?}"
    );
}
