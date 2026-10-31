//! The Gallery's title bar at narrow widths: the controls never run past the window's edge
//! and the title stays whole. A wide window keeps every control; narrower, the lower-priority
//! ones give way (the command palette still reaches them). On macOS the Host keeps the
//! window buttons in the middle of the bar, whose height follows the density.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::mount_prepared;
use crate::support::{Host, Step, last_error, settle};

fn mount(cx: &mut TestAppContext, width: f32) -> (VisualTestContext, ScriptViewHandle) {
    mount_launch(cx, width, &AcceptanceLaunch::default())
}

fn mount_launch(
    cx: &mut TestAppContext,
    width: f32,
    launch: &AcceptanceLaunch,
) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(launch)
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let (window, view) =
        mount_prepared(cx, prepared, ("gallery", "gallery"), false, |host, view| {
            Host { host, view }
        });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(gpui::px(width), gpui::px(860.0)));
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
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
/// narrower, the Density toggle gives way, and in a compact window the theme picker too.
#[gpui::test]
fn the_title_bar_fits_narrow_windows(cx: &mut TestAppContext) {
    let width_of = |nodes: &[(String, String, f64, f64)], name: &str| {
        find(nodes, "text", name).map(|(_, _, left, right)| right - left)
    };
    // The title and page title as a wide window draws them.
    let mut whole = None;
    for (width, expected) in [
        (1280.0, &["Density", "Theme", "Locale", "Commands"][..]),
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

#[gpui::test]
fn the_window_buttons_sit_in_the_middle_of_the_bar_in_either_density(cx: &mut TestAppContext) {
    for (density, height, y) in [("comfortable", 36.0, 11.0), ("compact", 32.0, 9.0)] {
        let launch = AcceptanceLaunch {
            density: density.to_owned(),
            ..AcceptanceLaunch::default()
        };
        let (mut visual, view) = mount_launch(cx, 1280.0, &launch);
        let (bar, position) = visual.update(|_, cx| {
            let tree = view.accessibility_snapshot(cx).unwrap();
            let bar = tree
                .find_by_role_and_name("toolbar", "gpui-rhai Gallery title bar")
                .next()
                .and_then(|node| node.geometry)
                .expect("the Gallery has a title bar")
                .visual;
            (bar, acceptance::titlebar_traffic_lights(&view, cx).unwrap())
        });
        assert!(
            (bar.height - height).abs() < 0.01,
            "{density}: the bar is {} high",
            bar.height
        );
        // The comfortable bar keeps the position the window opens with; the compact one
        // moves the buttons up by half the 4pt the bar loses.
        assert_eq!(
            position,
            Some(gpui::point(gpui::px(12.0), gpui::px(y))),
            "{density}"
        );
    }
}
