//! A column stretches a child without a width of its own. The renderer gives
//! such a child a definite `width: 100%` where that is the same box (it saves
//! Taffy a second measuring pass); everywhere else it must lay out exactly like
//! an explicit `.self_stretch()`, in both directions.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;

const THEME: &str = r#"fn theme(){#{family:"Layout",name:"Dark",mode:"dark",tokens:#{}}}"#;

/// Child styles that must stretch identically, by name.
const CASES: &[(&str, &str)] = &[
    ("margin", "style().height(px(30)).margin_x(px(20))"),
    (
        "start-margin",
        "style().height(px(30)).margin_start(px(30))",
    ),
    ("auto-margin", "style().height(px(30)).margin_x(auto())"),
    (
        "padding-border",
        "style().height(px(30)).padding_x(px(16)).border(px(2))",
    ),
    (
        "margin-padding",
        "style().height(px(30)).margin_x(px(12)).padding_x(px(16))",
    ),
    ("max-width", "style().height(px(30)).max_width(px(200))"),
    ("min-width", "style().height(px(30)).min_width(px(400))"),
    ("plain", "style().height(px(30))"),
];

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn script(locale: &str) -> String {
    let mut pairs = Vec::new();
    for (name, child) in CASES {
        for (kind, extra) in [("implicit", ""), ("explicit", ".self_stretch()")] {
            pairs.push(format!(
                r#"column([box([text("x")]).accessibility_role("group").test_id("{kind}-{name}").with_style({child}{extra})]).with_style(style().width(px(300)))"#
            ));
        }
    }
    format!(
        r#"fn init(ctx) {{ ctx.set_locale("{locale}"); }}
fn view(ctx) {{ column([{}]).with_style(style().width(px(600)).gap(px(8))) }}"#,
        pairs.join(",\n")
    )
}

fn mount(cx: &mut TestAppContext, locale: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, script(locale))])),
        THEME,
    )
    .locale_sources([
        (
            "en.rhai".into(),
            include_str!("../../../registry/locales/en.rhai").to_owned(),
        ),
        (
            "ar.rhai".into(),
            include_str!("../../../registry/locales/ar.rhai").to_owned(),
        ),
    ])
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("stretch", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("stretch"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    (window, view)
}

fn geometry(locale: &str, cx: &mut TestAppContext) -> BTreeMap<String, GeometryBounds> {
    let (window, view) = mount(cx, locale);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .filter_map(|node| Some((node.test_id.clone()?, node.geometry?.visual)))
        .collect()
}

#[gpui::test]
fn implicit_stretch_lays_out_like_an_explicit_stretch(cx: &mut TestAppContext) {
    let mut report = Vec::new();
    for locale in ["en", "ar"] {
        let found = geometry(locale, cx);
        for (name, _) in CASES {
            let implicit = found[&format!("implicit-{name}")];
            let explicit = found[&format!("explicit-{name}")];
            // The pairs sit in separate parents at the same x offset.
            let same = (implicit.x - explicit.x).abs() < 0.1
                && (implicit.width - explicit.width).abs() < 0.1;
            report.push(format!(
                "{locale} {name}: implicit x={} w={}, explicit x={} w={}{}",
                implicit.x,
                implicit.width,
                explicit.x,
                explicit.width,
                if same { "" } else { "  <- differs" }
            ));
        }
        // The review's counterexample, stated directly.
        assert!(
            (found["implicit-margin"].width - 260.0).abs() < 0.1,
            "{locale}: margins come out of the stretched width"
        );
    }
    let text = report.join("\n");
    println!("{text}");
    assert!(!text.contains("differs"), "{text}");
}
