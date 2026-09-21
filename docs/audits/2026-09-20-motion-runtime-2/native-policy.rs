use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
#[gpui::test]
fn none_policy_still_animates_layout(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let manual = ManualRuntimeClock::new(Instant::now());
    let entry = ModuleId::parse("main").unwrap();
    let source = r#"
 fn state_schema(){#{fields:#{x:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
 fn move_card(ctx,payload){ctx.set_state("x",100);}
 fn init(ctx){ctx.register_action("card.move",Fn("move_card"));}
 fn view(ctx){column([text("Card").with_key("card").test_id("card")
    .accessibility_role("button").accessibility_label("Card")
    .layout_motion(1000,"linear")
    .with_style(style().width(px(100)).height(px(30)).margin_left(px(ctx.get_state("x"))))
 ])}
 "#;
    let theme = std::fs::read_to_string(format!(
        "{}/registry/themes/default_dark.rhai",
        std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap()
    ))
    .unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.to_owned())])),
        theme,
    )
    .runtime_clock(manual.clock())
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let take = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("w", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("v"), host.clone(), window, cx)
            .unwrap();
        *take.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let initial = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Card")
            .next()
            .unwrap()
            .geometry
            .unwrap()
    });
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "card.move".into(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
    cx.refresh().unwrap();
    let changed = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Card")
            .next()
            .unwrap()
            .geometry
            .unwrap()
    });
    manual.advance(Duration::from_millis(500));
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "card.move".into(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
    cx.refresh().unwrap();
    let halfway = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Card")
            .next()
            .unwrap()
            .geometry
            .unwrap()
    });
    println!("NONE_LAYOUT initial={initial:?} changed={changed:?} halfway={halfway:?}");
    assert!(
        (changed.layout.x - changed.visual.x).abs() > 90.0,
        "characterization should reveal unwanted layout motion"
    );
    assert!((halfway.layout.x - halfway.visual.x).abs() > 40.0);
    manual.advance(Duration::from_secs(1));
    cx.refresh().unwrap();
}

struct Empty;
impl Render for Empty {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
    }
}
#[gpui::test]
fn actual_motion_gallery_mount(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let repo = std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap();
    let example = std::fs::read_to_string(format!(
        "{repo}/crates/gpui-rhai/examples/motion_gallery.rs"
    ))
    .unwrap();
    let source = example
        .split_once("const MAIN: &str = r#\"")
        .unwrap()
        .1
        .split_once("\"#;")
        .unwrap()
        .0;
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), source.to_owned())]);
    for item in std::fs::read_dir(format!("{repo}/registry/motion")).unwrap() {
        let path = item.unwrap().path();
        if path.extension().is_some_and(|e| e == "rhai") {
            modules.insert(
                ModuleId::parse(format!(
                    "motion/{}",
                    path.file_stem().unwrap().to_str().unwrap()
                ))
                .unwrap(),
                std::fs::read_to_string(path).unwrap(),
            );
        }
    }
    modules.insert(
        ModuleId::parse("components/tabs").unwrap(),
        std::fs::read_to_string(format!("{repo}/registry/components/tabs.rhai")).unwrap(),
    );
    let theme =
        std::fs::read_to_string(format!("{repo}/registry/themes/default_dark.rhai")).unwrap();
    let locale = std::fs::read_to_string(format!("{repo}/registry/locales/en.rhai")).unwrap();
    let prepared = EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), theme)
        .locale_sources([("en.rhai".into(), locale)])
        .prepare()
        .unwrap();
    cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("gallery-test", cx).unwrap();
        let result = prepared.mount(ScriptViewConfig::new("gallery"), host, window, cx);
        let error = result
            .err()
            .expect("current gallery should expose the mount regression");
        println!("ACTUAL_GALLERY_MOUNT_ERROR={error}");
        assert!(error.to_string().contains("size_full"));
        Empty
    });
}
