//! The Gallery's title bar at narrow widths: the controls never run past the window's edge
//! and the title stays whole. A wide window keeps every control; narrower, the lower-priority
//! ones give way (the command palette still reaches them).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, width: f32) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch::default())
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("gallery", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("gallery"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(gpui::px(width), gpui::px(860.0)));
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    (visual, view)
}

/// (role, name, left, right) of every node on the title bar's line, the bar included.
fn title_bar_nodes(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<(String, String, f64, f64)> {
    visual.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        let bar = tree
            .find_by_role_and_name("toolbar", "gpui-rhai Gallery title bar")
            .next()
            .and_then(|node| node.geometry)
            .expect("the Gallery has a title bar")
            .visual;
        tree.nodes()
            .filter_map(|node| {
                let visual = node.geometry.as_ref()?.visual;
                (visual.y >= bar.y - 0.5 && visual.y + visual.height <= bar.y + bar.height + 0.5)
                    .then(|| {
                        (
                            node.role.clone(),
                            node.name.clone(),
                            visual.x,
                            visual.x + visual.width,
                        )
                    })
            })
            .collect()
    })
}

fn find<'a>(
    nodes: &'a [(String, String, f64, f64)],
    role: &str,
    name: &str,
) -> Option<&'a (String, String, f64, f64)> {
    nodes
        .iter()
        .find(|(node_role, node_name, ..)| node_role == role && node_name == name)
}

/// The controls on the bar, from the start edge.
fn controls(nodes: &[(String, String, f64, f64)]) -> Vec<&str> {
    let mut controls = nodes
        .iter()
        .filter(|(role, name, ..)| {
            ["group", "combobox"].contains(&role.as_str())
                || (role == "button" && name == "Commands")
        })
        .collect::<Vec<_>>();
    controls.sort_by(|left, right| left.2.total_cmp(&right.2));
    controls
        .into_iter()
        .map(|(_, name, ..)| name.as_str())
        .collect()
}

/// At each width the title and the page title are drawn whole, the controls follow them
/// without overlap and none runs past the bar's end inset. A wide window keeps every control;
/// narrower, the Density and Corners toggles give way, and in a compact window the theme
/// picker too.
#[gpui::test]
fn the_title_bar_fits_narrow_windows(cx: &mut TestAppContext) {
    let width_of = |nodes: &[(String, String, f64, f64)], name: &str| {
        find(nodes, "text", name).map(|(_, _, left, right)| right - left)
    };
    // The title and page title as a wide window draws them.
    let mut whole = None;
    for (width, expected) in [
        (
            1280.0,
            &["Density", "Corners", "Theme", "Locale", "Commands"][..],
        ),
        (852.0, &["Theme", "Locale", "Commands"][..]),
        (560.0, &["Locale", "Commands"][..]),
    ] {
        let (mut visual, view) = mount(cx, width);
        let nodes = title_bar_nodes(&mut visual, &view);
        println!("{width}: {nodes:?}");
        assert_eq!(
            controls(&nodes),
            expected,
            "{width}: the controls on the bar"
        );
        let titles = (width_of(&nodes, "GPUI RHAI"), width_of(&nodes, "Button"));
        let (title, page) = *whole.get_or_insert(titles);
        assert!(
            title.is_some_and(|width| width > 0.0) && page.is_some_and(|width| width > 0.0),
            "{width}: the wide window draws both titles: {titles:?}"
        );
        assert_eq!(
            titles,
            (title, page),
            "{width}: the title and page title are drawn whole"
        );
        let page_end = find(&nodes, "text", "Button").unwrap().3;
        let first = find(&nodes, "group", expected[0])
            .or_else(|| find(&nodes, "combobox", expected[0]))
            .unwrap();
        assert!(
            first.2 >= page_end,
            "{width}: the controls start after the page title: {first:?}, page title ends at \
             {page_end}"
        );
        let end = f64::from(width) - 12.0;
        for (role, name, left, right) in &nodes {
            assert!(
                *left >= -0.5 && *right <= end + 0.5 || role == "toolbar",
                "{width}: {role} {name:?} runs to {right}, past the bar's end inset {end}"
            );
        }
    }
}
