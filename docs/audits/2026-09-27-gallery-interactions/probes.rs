//! Characterization probes: assertions describe observed defects, not release acceptance.
use gpui::prelude::*;
use gpui::{
    Context, IntoElement, Modifiers, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent,
    TestAppContext, VisualTestContext, Window, div, point, px, rgba,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

struct ScrollHost {
    handle: ScrollHandle,
    horizontal: bool,
    restricted: bool,
}
impl Render for ScrollHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut scroller = div()
            .id("axis-probe")
            .w(px(200.))
            .h(px(80.))
            .track_scroll(&self.handle);
        if self.horizontal {
            scroller = scroller
                .overflow_x_scroll()
                .child(div().w(px(1000.)).h(px(40.)).bg(rgba(0xff0000ff)));
        } else {
            scroller = scroller
                .overflow_y_scroll()
                .child(div().w(px(100.)).h(px(1000.)).bg(rgba(0xff0000ff)));
        }
        scroller.style().restrict_scroll_to_axis = Some(self.restricted);
        div().size_full().child(scroller)
    }
}

#[gpui::test]
fn upstream_single_axis_remaps_the_other_axis_unless_restricted(cx: &mut TestAppContext) {
    for horizontal in [true, false] {
        for restricted in [false, true] {
            let handle = ScrollHandle::new();
            let cloned = handle.clone();
            let window = cx.add_window(move |_, _| ScrollHost {
                handle: cloned,
                horizontal,
                restricted,
            });
            cx.refresh().unwrap();
            let mut visual = VisualTestContext::from_window(*window, cx);
            let position = point(px(40.), px(30.));
            visual.simulate_mouse_move(position, None, Modifiers::none());
            visual.simulate_event(ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(if horizontal {
                    point(px(0.), px(-45.))
                } else {
                    point(px(-45.), px(0.))
                }),
                ..Default::default()
            });
            visual.run_until_parked();
            visual.update(|window, _| window.refresh());
            visual.run_until_parked();
            let offset = handle.offset();
            println!("AXIS horizontal={horizontal} restricted={restricted} offset={offset:?}");
            let moved = if horizontal { offset.x } else { offset.y };
            assert_eq!(moved, if restricted { px(0.) } else { px(-45.) });
        }
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

fn mount(cx: &mut TestAppContext, source: &str) -> (gpui::WindowHandle<Host>, ScriptViewHandle) {
    let root = std::path::Path::new(env!("GPUI_RHAI_AUDIT_ROOT"));
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, source.to_owned()),
            (
                ModuleId::parse("components/input").unwrap(),
                std::fs::read_to_string(root.join("registry/components/input.rhai")).unwrap(),
            ),
        ])),
        std::fs::read_to_string(root.join("registry/themes/default_dark.rhai")).unwrap(),
    )
    .theme_token_overrides(ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("border".to_owned(), Rgba8::from_rgba_hex(0xff0000ff)),
            ("focus_ring".to_owned(), Rgba8::from_rgba_hex(0x00ff00ff)),
        ]),
        ..Default::default()
    })
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let cloned = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("audit", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("audit"), host.clone(), window, cx)
            .unwrap();
        *cloned.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}

fn border_counts(visual: &mut VisualTestContext) -> (usize, usize) {
    visual.update(|window, _| {
        let quads = window.painted_quads();
        let red = rgba(0xff0000ff).into();
        let green = rgba(0x00ff00ff).into();
        let visible: Vec<_> = quads
            .iter()
            .filter(|q| q.border_widths.left.0 > 0.)
            .collect();
        (
            visible.iter().filter(|q| q.border_color == red).count(),
            visible.iter().filter(|q| q.border_color == green).count(),
        )
    })
}

#[gpui::test]
fn native_input_accepts_text_but_does_not_paint_declared_focus_border(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(
        cx,
        r#"
import "components/input" as input;
fn state_schema() { #{ fields: #{ value: #{ schema: #{ type: "string" },
    "default": #{ type: "string", value: "initial" } } } } }
fn changed(ctx, value) { ctx.set_state("value", value); }
fn view(ctx) { input::Input(#{key:"field",label:"Audit input",value:ctx.get_state("value"),
    on_change:Fn("changed")}).with_style(style().width(px(220))) }
"#,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text_field", "Audit input")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let before = border_counts(&mut visual);
    visual.simulate_click(
        point(
            px((bounds.x + 30.) as f32),
            px((bounds.y + bounds.height / 2.) as f32),
        ),
        Modifiers::none(),
    );
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("focused and editable");
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let after = border_counts(&mut visual);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let input = tree
        .find_by_role_and_name("text_field", "Audit input")
        .next()
        .unwrap();
    println!(
        "INPUT before(red,green)={before:?} after={after:?} value={:?}",
        input.value
    );
    assert!(format!("{:?}", input.value).contains("focused and editable"));
    assert!(before.0 > 0 && after.0 > 0, "idle border still painted");
    assert_eq!(
        after.1, 0,
        "characterization: focus_ring is not painted despite native text input"
    );
}

#[gpui::test]
fn rhai_scroll_container_inherits_cross_axis_remapping(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(
        cx,
        r#"
fn view(ctx) { box([
    text("Scroll marker").accessibility_label("Scroll marker")
        .with_style(style().width(px(1000)).height(px(40)))
]).with_style(style().width(px(200)).height(px(80)).overflow_x_scroll()) }
"#,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    let marker_x = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            let tree = view.accessibility_snapshot(cx).unwrap();
            tree.find_by_role_and_name("text", "Scroll marker")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
                .x
        })
    };
    let before = marker_x(&mut visual);
    let position = point(px(40.), px(30.));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.), px(-45.))),
        ..Default::default()
    });
    visual.run_until_parked();
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let after = marker_x(&mut visual);
    println!("RHAI_SCROLL vertical_delta=-45 marker_x_before={before} marker_x_after={after}");
    assert!((after - before + 45.).abs() < 0.01);
}

fn documents(node: &UiNode, result: &mut BTreeMap<String, (u64, String)>) {
    match node.kind() {
        UiNodeKind::Custom { primitive } => {
            for (_, value) in primitive.props.iter() {
                if let PrimitiveValue::Document(document) = value {
                    result.insert(
                        document.identity().to_owned(),
                        (document.revision(), document.text().to_owned()),
                    );
                }
            }
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                documents(child, result);
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            documents(trigger, result);
            documents(content, result);
        }
        UiNodeKind::Layer { content, .. } => documents(content, result),
        _ => {}
    }
}

fn visible_texts(node: &UiNode, result: &mut Vec<String>) {
    match node.kind() {
        UiNodeKind::Text { text, .. } => result.push(text.to_string()),
        UiNodeKind::RichText { spans, .. } => result.push(spans.iter().map(|s| s.text()).collect()),
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                visible_texts(child, result);
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            visible_texts(trigger, result);
            visible_texts(content, result);
        }
        UiNodeKind::Layer { content, .. } => visible_texts(content, result),
        _ => {}
    }
}

#[gpui::test]
fn workbench_reports_success_without_updating_configuration_documents(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let launch = gpui_rhai_cli::gallery::GalleryLaunch {
        story: "apps/operations".to_owned(),
        case: "config-diff".to_owned(),
        ..Default::default()
    };
    let prepared = gpui_rhai_cli::gallery::prepare(&launch).unwrap();
    let captured = Rc::new(RefCell::new(None));
    let cloned = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("workbench-audit", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("workbench-audit"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *cloned.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let snapshot = |visual: &mut VisualTestContext| {
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let mut found = BTreeMap::new();
        documents(&root, &mut found);
        found
    };
    let before = snapshot(&mut visual);
    assert_eq!(before.len(), 2);
    // Change deployment data through actual native text input, not a fabricated callback.
    let input_bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text_field", "Release channel")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    visual.simulate_click(
        point(
            px((input_bounds.x + 30.) as f32),
            px((input_bounds.y + input_bounds.height / 2.) as f32),
        ),
        Modifiers::none(),
    );
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("audit-canary");
    visual.run_until_parked();
    let value = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text_field", "Release channel")
            .next()
            .unwrap()
            .value
            .clone()
    });
    assert_eq!(value, Some(UiValue::String("audit-canary".to_owned())));
    for id in ["stage-deploy", "confirm-deploy"] {
        visual
            .update(|window, cx| {
                view.automate(
                    AutomationCommand::Dispatch {
                        locator: AutomationLocator::TestId { id: id.to_owned() },
                        event: "click".to_owned(),
                        payload: None,
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        visual.run_until_parked();
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
    loop {
        visual.run_until_parked();
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let mut texts = Vec::new();
        visible_texts(&root, &mut texts);
        if texts
            .iter()
            .any(|s| s == "Configuration verified and activated.")
        {
            assert!(texts.iter().any(|s| s.contains("audit-canary")));
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "deployment failed to settle"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::RoleName {
                        role: "button".to_owned(),
                        name: "Configurations".to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
    let after = snapshot(&mut visual);
    println!("WORKBENCH successful_channel=audit-canary before={before:?} after={after:?}");
    assert_eq!(
        before, after,
        "characterization: success never changed the Host documents"
    );
    assert!(
        after
            .values()
            .all(|(_, text)| !text.contains("audit-canary"))
    );
}

struct ClickHost {
    keyboard: bool,
    clicks: Rc<std::cell::Cell<usize>>,
}
impl Render for ClickHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let mut button = div()
            .id("gallery-style-control")
            .role(gpui::Role::Button)
            .aria_label("Reset current story")
            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
            .child("Reset story");
        if self.keyboard {
            button = button.tab_index(0);
        }
        div().size_full().child(button)
    }
}

#[gpui::test]
fn gallery_style_role_and_click_control_has_no_keyboard_focus_without_explicit_tab_stop(
    cx: &mut TestAppContext,
) {
    for keyboard in [false, true] {
        let clicks = Rc::new(std::cell::Cell::new(0));
        let cloned = clicks.clone();
        let window = cx.add_window(move |_, _| ClickHost {
            keyboard,
            clicks: cloned,
        });
        cx.refresh().unwrap();
        let mut visual = VisualTestContext::from_window(*window, cx);
        let focused = visual.update(|window, cx| {
            window.focus_next(cx);
            window.focused(cx).is_some()
        });
        visual.run_until_parked();
        visual.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        visual.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        println!(
            "GALLERY_CONTROL tab_stop={keyboard} focused={focused} keyboard_clicks={}",
            clicks.get()
        );
        assert_eq!(focused, keyboard);
        assert_eq!(clicks.get(), usize::from(keyboard));
    }
}
