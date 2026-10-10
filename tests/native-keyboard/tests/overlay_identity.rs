//! An overlay's identity is (declaring component instance, key) (#109): three
//! instances of one component, each with a Select keyed the same, open and
//! close their own Select. Host lookups by key report the ambiguity and can
//! name the instance.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
use gpui_rhai::*;
use gpui_rhai_registry::{BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME};

const FILTER: &str = r#"
import "components/select" as select;
define_component(#{
    metadata: #{ id: "test/filter", "export": "Filter", version: "0.0.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/select"], capabilities: #{}, tokens: [], environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        },
        state: #{ fields: #{ open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: false } } } },
        events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_Filter")
});
fn Filter(props) { render_component("test/filter", props) }
fn opened(ctx, open) { ctx.set_state("open", open); }
fn render_Filter(ctx, props) {
    column([
        text(`${props.label} open=${ctx.get_state("open")}`).accessibility_role("group")
            .test_id(`state-${props.label}`),
        select::Select(#{ key: "region-filter", label: props.label, value: "",
            options: [#{ value: "", label: "All" }, #{ value: "x", label: "X" }],
            open: ctx.get_state("open"), query: "", on_open_change: Fn("opened") }),
    ]).with_style(style().width(px(200)).height(px(120)))
}
"#;

const MAIN: &str = r#"
import "test/filter" as filter;
fn view(ctx) {
    row([
        filter::Filter(#{ key: "sg", label: "sg" }),
        filter::Filter(#{ key: "eu", label: "eu" }),
        filter::Filter(#{ key: "us", label: "us" }),
    ]).with_style(style().width(px(760)).height(px(500)).gap(px(40)))
}
"#;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHost, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    modules.insert(ModuleId::parse("test/filter").unwrap(), FILTER.to_owned());
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), MAIN.to_owned());
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .locale_sources([(
                "en.rhai".to_owned(),
                include_str!("../../../registry/locales/en.rhai").to_owned(),
            )])
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("filters", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("filters"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some((host.clone(), view.clone()));
        Host { host, view }
    });
    cx.run_until_parked();
    let (host, view) = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, host, view)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..2 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn open_states(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let mut states = tree
        .nodes()
        .filter(|node| {
            node.test_id
                .as_deref()
                .is_some_and(|id| id.starts_with("state-"))
        })
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    states.sort();
    states.join(", ")
}

fn click(visual: &mut VisualTestContext, x: f64, y: f64) {
    #[allow(clippy::cast_possible_truncation)]
    let position = point(px(x as f32), px(y as f32));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    settle(visual);
}

fn click_trigger(visual: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let bounds = tree
        .nodes()
        .find(|node| node.role == "combobox" && node.name == label)
        .and_then(|node| node.geometry)
        .map(|geometry| geometry.visual)
        .unwrap_or_else(|| panic!("no combobox {label}"));
    click(
        visual,
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
}

#[gpui::test]
fn each_instance_closes_its_own_overlay(cx: &mut TestAppContext) {
    let (mut visual, _, view) = mount(cx);
    assert_eq!(
        open_states(&mut visual, &view),
        "eu open=false, sg open=false, us open=false"
    );
    // A second press on the trigger closes the first instance's Select, not the
    // last-rendered one's.
    click_trigger(&mut visual, &view, "sg");
    assert_eq!(
        open_states(&mut visual, &view),
        "eu open=false, sg open=true, us open=false"
    );
    click_trigger(&mut visual, &view, "sg");
    assert_eq!(
        open_states(&mut visual, &view),
        "eu open=false, sg open=false, us open=false"
    );
    // An outside press closes the middle instance.
    click_trigger(&mut visual, &view, "eu");
    assert_eq!(
        open_states(&mut visual, &view),
        "eu open=true, sg open=false, us open=false"
    );
    click(&mut visual, 700.0, 480.0);
    assert_eq!(
        open_states(&mut visual, &view),
        "eu open=false, sg open=false, us open=false"
    );
}

#[gpui::test]
fn host_lookups_by_key_name_the_instance_when_ambiguous(cx: &mut TestAppContext) {
    let (mut visual, host, view) = mount(cx);
    click_trigger(&mut visual, &view, "eu");
    // Select opens a Combobox overlay keyed `<key>-combobox`.
    let error = host
        .overlay_placement("filters", "region-filter-combobox")
        .unwrap_err();
    println!("{error}");
    let OverlayLookupError::Ambiguous { instances, .. } = &error;
    assert_eq!(instances.len(), 3, "{error}");
    let eu = instances
        .iter()
        .find(|path| path.contains("/Filter[eu]/"))
        .expect("the eu instance");
    let placement = host.overlay_placement_in("filters", eu, "region-filter-combobox");
    assert!(placement.is_some(), "the open eu Select has a placement");
    let sg = instances
        .iter()
        .find(|path| path.contains("/Filter[sg]/"))
        .unwrap();
    assert!(
        host.overlay_placement_in("filters", sg, "region-filter-combobox")
            .is_none(),
        "the closed sg Select has none"
    );
}

const FILES: &str = r#"
import "components/menu" as menu;
define_component(#{
    metadata: #{ id: "test/files", "export": "Files", version: "0.0.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/menu"], capabilities: #{}, tokens: [], environment: [] },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_Files")
});
fn Files(props) { render_component("test/files", props) }
fn changed(ctx, value) { () }
fn render_Files(ctx, props) {
    let child = menu::Menu(#{ key: "file-more", label: "More actions", parent_overlay: "file",
        trigger: text("More"), open: true, active_value: "export", placement: "right",
        items: [#{ kind: "item", value: "export", label: "Export" }],
        on_action: Fn("changed"), on_active_change: Fn("changed"), on_open_change: Fn("changed") });
    column([menu::Menu(#{ key: "file", label: `File ${props.key}`, trigger: text("File"),
        open: true, active_value: "more",
        items: [#{ kind: "submenu", value: "more", label: "More", submenu: child }],
        on_action: Fn("changed"), on_active_change: Fn("changed"), on_open_change: Fn("changed") })])
}
"#;

#[gpui::test]
fn a_submenu_parent_resolves_to_the_enclosing_menu_of_its_instance(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    modules.insert(ModuleId::parse("test/files").unwrap(), FILES.to_owned());
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(
        entry.clone(),
        r#"import "test/files" as files;
fn view(ctx) { row([files::Files(#{ key: "a" }), files::Files(#{ key: "b" })])
    .with_style(style().gap(px(300))) }"#
            .to_owned(),
    );
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .locale_sources([(
                "en.rhai".to_owned(),
                include_str!("../../../registry/locales/en.rhai").to_owned(),
            )])
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("files", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("files"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some((host.clone(), view.clone()));
        Host { host, view }
    });
    cx.run_until_parked();
    let (host, view) = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    // Each submenu registers under its own instance's parent menu: an unresolved
    // parent would leave it without a placement.
    let OverlayLookupError::Ambiguous { instances, .. } =
        host.overlay_placement("files", "file-more").unwrap_err();
    assert_eq!(instances.len(), 2, "{instances:?}");
    for instance in &instances {
        assert!(
            host.overlay_placement_in("files", instance, "file-more")
                .is_some(),
            "{instance} is placed"
        );
    }
}
