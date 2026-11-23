//! Real copyable reference programs: no copied entry script or injected theme.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext, WindowHandle, px, size};
use gpui_rhai::{
    AccessibilityNode, MotionPreference, ScriptView, ScriptViewConfig, ScriptViewHandle,
    ScriptViewHost, ScriptViewState, UiValue,
};

use crate::support::{
    ActiveHost, Step, bounds, click_center, filled_including_rounded_clips, last_error, settle,
    snapshot,
};

#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/artistic_showcase.rs"]
mod artistic_showcase;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/byod_treemap.rs"]
mod byod_treemap;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/code_viewer.rs"]
mod code_viewer;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/diff_viewer.rs"]
mod diff_viewer;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/embedded_hello_world.rs"]
mod embedded_hello_world;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/hello_world.rs"]
mod hello_world;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/mini_timeline.rs"]
mod mini_timeline;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/multi_window.rs"]
mod multi_window;
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/variable_height_chat.rs"]
mod variable_height_chat;

#[allow(dead_code)]
mod host_owned_tree {
    include!("../../../../../crates/gpui-rhai/examples/host_owned_tree.rs");

    pub(super) struct HeldWorker {
        events: Receiver<HostEvent>,
        frames: Sender<WorkerFrame>,
        frame: WorkerFrame,
    }

    impl HeldWorker {
        pub(super) fn publish_next(&mut self) -> bool {
            let Ok(event) = self.events.try_recv() else {
                return false;
            };
            self.frame.worker_event(event);
            self.frames.send(self.frame.clone()).unwrap();
            true
        }
    }

    pub(super) fn with_held_worker(
        cx: &mut gpui::TestAppContext,
    ) -> (gpui::WindowHandle<HostOwnedTree>, HeldWorker) {
        let (send_event, events) = channel();
        let (frames, receive_frame) = channel();
        let window = cx.add_window(move |_, cx| {
            install_view_fonts(cx);
            HostOwnedTree::with_worker(resolved_designs(), send_event, receive_frame, cx)
        });
        (
            window,
            HeldWorker {
                events,
                frames,
                frame: WorkerFrame::default(),
            },
        )
    }

    // Read the real example's Host state without adding test-only SDK APIs.
    pub(super) fn inspect(
        window: &gpui::WindowHandle<HostOwnedTree>,
        cx: &gpui::VisualTestContext,
    ) -> (ThemeVariant, String, String, Vec<gpui_rhai::NodeId>) {
        window
            .read_with(cx, |host, cx| {
                (
                    host.themes[host.theme_index].clone(),
                    host.frame.input.clone(),
                    host.frame.status().to_owned(),
                    host.view
                        .read(cx)
                        .retained()
                        .nodes()
                        .map(|node| node.id())
                        .collect(),
                )
            })
            .unwrap()
    }
}

const DESIGNS: [(&str, &str); 4] = [
    ("Modern", "Light"),
    ("Clean", "Dark"),
    ("Modern", "Dark"),
    ("Clean", "Light"),
];

#[gpui::test]
fn embedded_host_program_preserves_independent_widgets_through_designs_and_remount(
    cx: &mut TestAppContext,
) {
    use super::examples::embedded_views::{EmbeddedViewsDemo, WINDOW};

    let window = cx.add_window(EmbeddedViewsDemo::new);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(WINDOW.0), px(WINDOW.1)));
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 3);
    let widgets = window
        .read_with(&visual, |host, _| host.live_widgets())
        .unwrap();
    assert_eq!(widgets.len(), 3);
    let increment = bounds(&mut visual, &widgets[0], "button", "Increment");
    click_center(&mut visual, increment);
    settle(&mut visual, Step::Tick, 2);
    for (family, appearance) in DESIGNS {
        for view in &widgets {
            visual
                .update(|_, cx| view.select_theme(family, appearance, cx))
                .unwrap();
        }
        settle(&mut visual, Step::Tick, 2);
        let theme = visual.update(|_, cx| widgets[0].theme_snapshot(cx).unwrap().variant);
        assert!(
            filled_including_rounded_clips(
                &mut visual,
                theme.tokens.color("surface").unwrap().as_rgba_hex()
            )
            .iter()
            .any(|quad| (quad.bounds.width - WINDOW.0).abs() < 0.1
                && (quad.bounds.height - WINDOW.1).abs() < 0.1),
            "native Host chrome must follow {family}/{appearance}"
        );
        for (index, view) in widgets.iter().enumerate() {
            assert_eq!(last_error(&mut visual, view), None);
            assert!(
                snapshot(&mut visual, view)
                    .nodes()
                    .any(|node| node.name == if index == 0 { "1" } else { "0" }),
                "view {index} lost or shared its state"
            );
        }
    }
    let theme = visual.update(|_, cx| widgets[0].theme_snapshot(cx).unwrap().variant);
    let toggle = filled_including_rounded_clips(
        &mut visual,
        theme.tokens.color("surface_hover").unwrap().as_rgba_hex(),
    )
    .into_iter()
    .filter(|quad| quad.bounds.y < 80.0 && quad.bounds.width > 100.0 && quad.bounds.height < 50.0)
    .min_by(|left, right| left.bounds.y.total_cmp(&right.bounds.y))
    .expect("Host unmount action")
    .bounds;
    let toggle_at = gpui::point(
        px(toggle.x + toggle.width / 2.0),
        px(toggle.y + toggle.height / 2.0),
    );
    visual.simulate_click(toggle_at, gpui::Modifiers::none());
    settle(&mut visual, Step::Tick, 2);
    assert_eq!(widgets[2].state(), ScriptViewState::Disposed);
    assert_eq!(
        window
            .read_with(&visual, |host, _| host.live_widgets().len())
            .unwrap(),
        2
    );
    assert!(
        snapshot(&mut visual, &widgets[0])
            .nodes()
            .any(|node| node.name == "1")
    );
    visual.simulate_click(toggle_at, gpui::Modifiers::none());
    settle(&mut visual, Step::Tick, 2);
    let remounted = window
        .read_with(&visual, |host, _| host.live_widgets())
        .unwrap();
    assert_eq!(remounted.len(), 3);
    assert_eq!(remounted[2].state(), ScriptViewState::Active);
    let remounted_theme = visual.update(|_, cx| remounted[2].theme_snapshot(cx).unwrap().variant);
    assert_eq!(
        (
            remounted_theme.family.as_str(),
            remounted_theme.name.as_str()
        ),
        (theme.family.as_str(), theme.name.as_str()),
        "remount must use the Host's current design"
    );
    assert!(
        snapshot(&mut visual, &remounted[2])
            .nodes()
            .any(|node| node.name == "0")
    );
    assert!(
        snapshot(&mut visual, &remounted[0])
            .nodes()
            .any(|node| node.name == "1")
    );
    window
        .update(&mut visual, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
}

fn mounted(
    cx: &mut TestAppContext,
    builder: ScriptView,
    viewport: (f32, f32),
    managed: bool,
) -> (
    WindowHandle<ActiveHost>,
    VisualTestContext,
    ScriptViewHandle,
) {
    cx.update(gpui_rhai::install);
    let prepared = builder
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("main", cx).unwrap();
        let config = ScriptViewConfig::new("reference");
        let view = if managed {
            prepared.mount_window(config, host.clone(), window, cx)
        } else {
            prepared.mount(config, host.clone(), window, cx)
        }
        .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        ActiveHost { host, view }
    });
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(viewport.0), px(viewport.1)));
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(
        last_error(&mut visual, &view),
        None,
        "the real program must render successfully"
    );
    (window, visual, view)
}

fn landmark(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> AccessibilityNode {
    let tree = snapshot(visual, view);
    tree.find_by_role_and_name(role, name)
        .next()
        .unwrap_or_else(|| {
            panic!(
                "missing actual {role} {name:?}; present: {:?}",
                tree.nodes()
                    .take(24)
                    .map(|node| (&node.role, &node.name))
                    .collect::<Vec<_>>()
            )
        })
        .clone()
}

fn check_designs(
    cx: &mut TestAppContext,
    builder: ScriptView,
    viewport: (f32, f32),
    expected: (&str, &str),
    solid_surface: bool,
) {
    let (_, mut visual, view) = mounted(cx, builder, viewport, false);
    let id = landmark(&mut visual, &view, expected.0, expected.1).id;
    for (family, appearance) in DESIGNS {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 2);
        assert_eq!(last_error(&mut visual, &view), None);
        let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
        assert_eq!(
            (theme.family.as_str(), theme.name.as_str()),
            (family, appearance)
        );
        let item = landmark(&mut visual, &view, expected.0, expected.1);
        assert_eq!(
            item.id, id,
            "design switching must retain the actual content"
        );
        let geometry = item.geometry.unwrap().visual;
        assert!(
            geometry.width > 0.0 && geometry.height > 0.0,
            "{family}/{appearance}: {geometry:?}"
        );
        if solid_surface {
            assert!(
                filled_including_rounded_clips(
                    &mut visual,
                    theme.tokens.color("surface").unwrap().as_rgba_hex()
                )
                .iter()
                .any(|quad| quad.visible().width > 100.0 && quad.visible().height > 100.0),
                "{family}/{appearance} must paint, not merely report, the selected surface"
            );
        }
    }
    visual.update(|_, cx| view.dispose(cx)).unwrap();
    settle(&mut visual, Step::Tick, 2);
}

macro_rules! design_case {
    ($name:ident, $factory:path, $size:expr, $landmark:expr, $solid:expr) => {
        #[gpui::test]
        fn $name(cx: &mut TestAppContext) {
            check_designs(cx, $factory(), $size, $landmark, $solid);
        }
    };
}

design_case!(
    artistic_program,
    artistic_showcase::view,
    (760.0, 520.0),
    ("group", "Night Signals artistic showcase"),
    false
);
design_case!(
    code_program,
    code_viewer::view,
    (900.0, 620.0),
    ("document", "Rhai component source"),
    true
);
design_case!(
    diff_program,
    diff_viewer::view,
    (1100.0, 680.0),
    ("document", "Server A compared with Server B"),
    true
);
design_case!(
    embedded_hello_program,
    embedded_hello_world::view,
    (560.0, 320.0),
    ("text_field", "Name"),
    true
);
design_case!(
    file_hello_program,
    hello_world::view,
    (560.0, 320.0),
    ("text_field", "Name"),
    true
);
design_case!(
    timeline_program,
    mini_timeline::view,
    (760.0, 380.0),
    ("slider", "Timeline playhead"),
    true
);
design_case!(
    chat_program,
    variable_height_chat::view,
    (700.0, 640.0),
    ("list", "Messages"),
    true
);

#[gpui::test]
fn both_hello_programs_keep_real_edits_focus_and_undo_across_designs(cx: &mut TestAppContext) {
    for factory in [hello_world::view, embedded_hello_world::view] {
        let (window, mut visual, view) = mounted(cx, factory(), (560.0, 320.0), false);
        let input = bounds(&mut visual, &view, "text_field", "Name");
        click_center(&mut visual, input);
        visual.simulate_input("Migration");
        settle(&mut visual, Step::Tick, 2);
        let id = landmark(&mut visual, &view, "text_field", "Name").id;
        let focus = visual.update(|window, cx| window.focused(cx));
        assert!(focus.is_some());
        for (family, appearance) in DESIGNS {
            visual
                .update(|_, cx| view.select_theme(family, appearance, cx))
                .unwrap();
            settle(&mut visual, Step::Tick, 2);
            assert_eq!(last_error(&mut visual, &view), None);
            let input = landmark(&mut visual, &view, "text_field", "Name");
            assert_eq!(input.id, id);
            assert_eq!(input.value, Some(UiValue::String("Migration".into())));
            assert_eq!(visual.update(|window, cx| window.focused(cx)), focus);
        }
        visual.simulate_keystrokes("cmd-z");
        settle(&mut visual, Step::Tick, 2);
        assert_eq!(
            landmark(&mut visual, &view, "text_field", "Name").value,
            Some(UiValue::String(String::new()))
        );
        visual.update(|_, cx| view.dispose(cx)).unwrap();
        window
            .update(&mut visual, |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
    }
}

#[gpui::test]
fn byod_program_preserves_its_own_vocabulary_and_actual_selection(cx: &mut TestAppContext) {
    let (_, mut visual, view) = mounted(cx, byod_treemap::view(), (780.0, 520.0), false);
    assert_eq!(last_error(&mut visual, &view), None);
    let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
    assert_eq!(
        (theme.family.as_str(), theme.name.as_str()),
        ("Treemap", "Night")
    );
    assert!(
        theme.tokens.color("surface").is_none(),
        "do not inject the official token base into BYOD"
    );
    assert!(
        !filled_including_rounded_clips(
            &mut visual,
            theme.tokens.color("ground").unwrap().as_rgba_hex()
        )
        .is_empty()
    );
    let tile = bounds(&mut visual, &view, "button", "assets, 3100 MB");
    click_center(&mut visual, tile);
    settle(&mut visual, Step::Tick, 2);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|node| node.name == "assets · 3100 MB")
    );
    assert_eq!(last_error(&mut visual, &view), None);
    visual.update(|_, cx| view.dispose(cx)).unwrap();
    settle(&mut visual, Step::Tick, 2);
}

#[gpui::test]
fn multi_window_program_opens_a_real_peer_and_keeps_state_through_designs(cx: &mut TestAppContext) {
    let (window, mut visual, view) = mounted(cx, multi_window::view(), (820.0, 460.0), true);
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(
        cx.windows().len(),
        2,
        "real init must open the settings window"
    );
    let increment = bounds(&mut visual, &view, "button", "Increment shared counter");
    click_center(&mut visual, increment);
    settle(&mut visual, Step::Tick, 2);
    for (family, appearance) in DESIGNS {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 2);
        assert_eq!(last_error(&mut visual, &view), None);
        assert!(
            snapshot(&mut visual, &view)
                .nodes()
                .any(|node| node.name == "Shared count: 1")
        );
        for label in [
            "Increment shared counter",
            "Edit this window's draft",
            "Light",
            "Dark",
            "English",
            "العربية",
            "Open settings window",
            "Focus settings window",
        ] {
            let rect = bounds(&mut visual, &view, "button", label);
            assert!(
                rect.x >= 0.0
                    && rect.y >= 0.0
                    && rect.x + rect.width <= 820.1
                    && rect.y + rect.height <= 460.1,
                "{family}/{appearance}: {label} escaped the window: {rect:?}"
            );
        }
    }
    for appearance in ["Dark", "Light"] {
        let target = bounds(&mut visual, &view, "button", appearance);
        click_center(&mut visual, target);
        settle(&mut visual, Step::Tick, 2);
        let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
        assert_eq!(
            (theme.family.as_str(), theme.name.as_str()),
            ("Clean", appearance)
        );
    }
    assert!(!visual.simulate_close());
    settle(&mut visual, Step::Tick, 2);
    let cancel = bounds(&mut visual, &view, "button", "Cancel");
    click_center(&mut visual, cancel);
    settle(&mut visual, Step::Tick, 2);
    assert_eq!(cx.windows().len(), 2);
    assert!(!visual.simulate_close());
    settle(&mut visual, Step::Tick, 2);
    let close = bounds(&mut visual, &view, "button", "Close");
    click_center(&mut visual, close);
    cx.run_until_parked();
    assert_eq!(view.state(), ScriptViewState::Disposed);
    assert!(!cx.windows().contains(&*window));
}

#[gpui::test]
fn diff_program_uses_the_official_control_and_keeps_its_viewer(cx: &mut TestAppContext) {
    let (_, mut visual, view) = mounted(cx, diff_viewer::view(), (1100.0, 680.0), false);
    let document_id = landmark(
        &mut visual,
        &view,
        "document",
        "Server A compared with Server B",
    )
    .id;
    for (family, appearance) in DESIGNS {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 2);
        let unified = bounds(&mut visual, &view, "button", "Unified");
        click_center(&mut visual, unified);
        settle(&mut visual, Step::Tick, 2);
        assert_eq!(
            landmark(&mut visual, &view, "button", "Unified").pressed,
            Some(true)
        );
        assert_eq!(
            landmark(&mut visual, &view, "button", "Split").pressed,
            Some(false)
        );
        visual.simulate_keystrokes("right space");
        settle(&mut visual, Step::Tick, 2);
        assert_eq!(
            landmark(&mut visual, &view, "button", "Split").pressed,
            Some(true)
        );
        assert_eq!(
            landmark(
                &mut visual,
                &view,
                "document",
                "Server A compared with Server B"
            )
            .id,
            document_id
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
    visual.update(|_, cx| view.dispose(cx)).unwrap();
    settle(&mut visual, Step::Tick, 2);
}

#[gpui::test]
fn rust_owned_program_switches_real_themes_without_losing_worker_state_or_undo(
    cx: &mut TestAppContext,
) {
    use std::time::{Duration, Instant};
    // The real example uses an OS worker/channel and a foreground poll Task.
    cx.executor().allow_parking();
    let window = cx.add_window(|_, cx| host_owned_tree::view(cx));
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(568.0), px(440.0)));
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 2);
    let (theme, _, _, _) = host_owned_tree::inspect(&window, &visual);
    let input = filled_including_rounded_clips(
        &mut visual,
        theme.tokens.color("surface.field").unwrap().as_rgba_hex(),
    )
    .into_iter()
    .find(|quad| {
        quad.bounds.width > 400.0 && quad.bounds.height >= 24.0 && quad.bounds.height <= 60.0
    })
    .expect("actual Host-owned native input fill")
    .bounds;
    visual.simulate_click(
        gpui::point(
            px(input.x + input.width / 2.0),
            px(input.y + input.height / 2.0),
        ),
        gpui::Modifiers::none(),
    );
    visual.simulate_input("Migration");
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        settle(&mut visual, Step::Tick, 1);
        let (_, text, status, _) = host_owned_tree::inspect(&window, &visual);
        if text == "Migration" && status == "Input event handled on the Host worker" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the actual worker did not commit: {text:?}, {status:?}"
        );
        std::thread::yield_now();
    }
    let (_, _, _, ids) = host_owned_tree::inspect(&window, &visual);
    for expected in [
        ("Modern", "Dark"),
        ("Modern", "Light"),
        ("Clean", "Dark"),
        ("Clean", "Light"),
    ] {
        let (theme, text, _, current_ids) = host_owned_tree::inspect(&window, &visual);
        assert_eq!((theme.family.as_str(), theme.name.as_str()), expected);
        assert_eq!(text, "Migration");
        assert_eq!(current_ids, ids);
        assert!(
            !filled_including_rounded_clips(
                &mut visual,
                theme.tokens.color("surface").unwrap().as_rgba_hex()
            )
            .is_empty()
        );
        let next = filled_including_rounded_clips(
            &mut visual,
            theme.tokens.color("surface_hover").unwrap().as_rgba_hex(),
        )
        .into_iter()
        .filter(|quad| quad.bounds.width > 400.0)
        .min_by(|left, right| left.bounds.y.total_cmp(&right.bounds.y))
        .expect("actual Next design action")
        .bounds;
        visual.simulate_click(
            gpui::point(
                px(next.x + next.width / 2.0),
                px(next.y + next.height / 2.0),
            ),
            gpui::Modifiers::none(),
        );
        settle(&mut visual, Step::Tick, 2);
    }
    // Four changes return to the initial metrics; refocus before native undo.
    visual.simulate_click(
        gpui::point(
            px(input.x + input.width / 2.0),
            px(input.y + input.height / 2.0),
        ),
        gpui::Modifiers::none(),
    );
    visual.simulate_keystrokes("cmd-z");
    settle(&mut visual, Step::Tick, 2);
    assert_eq!(host_owned_tree::inspect(&window, &visual).1, "");
    window
        .update(&mut visual, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn rust_owned_program_never_replaces_a_newer_native_edit_with_a_delayed_worker_frame(
    cx: &mut TestAppContext,
) {
    let (window, mut worker) = host_owned_tree::with_held_worker(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(568.0), px(440.0)));
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 2);
    let (theme, _, _, identities) = host_owned_tree::inspect(&window, &visual);
    let input = filled_including_rounded_clips(
        &mut visual,
        theme.tokens.color("surface.field").unwrap().as_rgba_hex(),
    )
    .into_iter()
    .find(|quad| {
        quad.bounds.width > 400.0 && quad.bounds.height >= 24.0 && quad.bounds.height <= 60.0
    })
    .expect("actual Host-owned native input fill")
    .bounds;
    visual.simulate_click(
        gpui::point(
            px(input.x + input.width / 2.0),
            px(input.y + input.height / 2.0),
        ),
        gpui::Modifiers::none(),
    );
    visual.simulate_input("First");
    visual.simulate_input(" later");
    assert_eq!(host_owned_tree::inspect(&window, &visual).1, "First later");
    assert!(worker.publish_next());
    settle(&mut visual, Step::Tick, 3);
    let (_, value, status, nodes) = host_owned_tree::inspect(&window, &visual);
    assert_eq!(
        value, "First later",
        "the worker had only acknowledged the first edit"
    );
    assert_eq!(status, "Input sent to the Host worker");
    assert_eq!(nodes, identities);
    let mut remaining = 0;
    while worker.publish_next() {
        remaining += 1;
    }
    assert!(
        remaining > 0,
        "the worker must acknowledge the rest of the edits"
    );
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(
        host_owned_tree::inspect(&window, &visual).2,
        "Input event handled on the Host worker"
    );

    // A delayed clear must not erase text restored by native undo either.
    visual.simulate_keystrokes("cmd-a backspace");
    assert_eq!(host_owned_tree::inspect(&window, &visual).1, "");
    visual.simulate_keystrokes("cmd-z");
    assert_eq!(host_owned_tree::inspect(&window, &visual).1, "First later");
    assert!(worker.publish_next(), "worker processes the former clear");
    settle(&mut visual, Step::Tick, 3);
    let (_, value, status, nodes) = host_owned_tree::inspect(&window, &visual);
    assert_eq!(value, "First later", "a delayed clear cannot undo the undo");
    assert_eq!(status, "Input sent to the Host worker");
    assert_eq!(nodes, identities);
    assert!(worker.publish_next(), "worker processes the latest undo");
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(host_owned_tree::inspect(&window, &visual).1, "First later");
    assert_eq!(
        host_owned_tree::inspect(&window, &visual).2,
        "Input event handled on the Host worker"
    );
}
