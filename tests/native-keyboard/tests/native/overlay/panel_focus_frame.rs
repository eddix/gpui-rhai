//! A container that holds focus itself shows the 2px focus frame over its edge
//! (#118): an overlay panel with focus on the panel, not on a control inside.

use std::collections::BTreeMap;

use gpui::{TestAppContext, VisualTestContext, rgba};
use gpui_rhai::{FocusIndicators, MotionPreference, Rgba8, ScriptViewHandle, ThemeTokenOverrides};

use crate::support::{Mount, Step, bounds, click_center, nodes, painted_quads};

/// A focus color no other part of the theme uses.
const RING: u32 = 0x00ff_00ff;

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    mount_with_indicators(cx, view_body, FocusIndicators::Always)
}

fn mount_with_indicators(
    cx: &mut TestAppContext,
    view_body: &str,
    indicators: FocusIndicators,
) -> (VisualTestContext, ScriptViewHandle) {
    mount_theme(cx, view_body, indicators, None)
}

fn mount_theme(
    cx: &mut TestAppContext,
    view_body: &str,
    indicators: FocusIndicators,
    theme: Option<&str>,
) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"import "components/button" as button;
import "components/dialog" as dialog;
import "components/menu" as menu;
import "components/sheet" as sheet;
import "components/popover" as popover;
import "components/combobox" as combobox;
fn noop(ctx, payload) {{ () }}
fn state_schema() {{ #{{ fields: #{{ open: #{{ schema: #{{ type: "bool" }},
    "default": #{{ type: "bool", value: false }} }} }} }} }}
fn open(ctx, payload) {{ ctx.set_state("open", true); }}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(800)).height(px(600))) }}"#
    );
    let mut mount = Mount::components(main)
        .config(move |config| config.focus_indicators(indicators))
        .with(|view| {
            view.theme_token_overrides(ThemeTokenOverrides {
                colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(RING))]),
                ..Default::default()
            })
        })
        .motion(MotionPreference::None)
        .id("frame");
    if let Some(theme) = theme {
        mount = mount.theme(theme);
    }
    let (mut visual, view) = mount.mount(cx);
    settle(&mut visual);
    (visual, view)
}

#[gpui::test]
fn overlay_focus_decoration_tracks_the_actual_box_in_all_four_presets(cx: &mut TestAppContext) {
    let fixtures = [
        (
            "dialog",
            "Confirm",
            r#"dialog::Dialog(#{ key:"confirm", open:true, title:"Confirm", content:text("Continue?") })"#,
        ),
        (
            "dialog",
            "Inspector",
            r#"sheet::Sheet(#{ key:"inspector", open:true, title:"Inspector", content:text("Details"), initial_focus:"panel" })"#,
        ),
        (
            "dialog",
            "Profile",
            r#"popover::Popover(#{key:"profile",label:"Profile",open:true,trigger:text("Show"),content:text("Details")})"#,
        ),
        (
            "menu",
            "File",
            r#"menu::Menu(#{key:"file",label:"File",trigger:text("File"),open:true,active_value:"open",items:[#{kind:"item",value:"open",label:"Open"}],on_active_change:Fn("noop"),on_action:Fn("noop")})"#,
        ),
        (
            "listbox",
            "Regions",
            r#"combobox::Combobox(#{key:"regions",label:"Regions",open:true,query:"",selected:[],options:[#{value:"a",label:"Alpha"}],on_change:Fn("noop"),on_open_change:Fn("noop")})"#,
        ),
    ];
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for (role, label, body) in fixtures {
            let (mut visual, view) = mount_theme(cx, body, FocusIndicators::Always, Some(theme));
            // Nonmodal popovers/listboxes intentionally do not steal focus on
            // open. Reach their panel through the normal Tab chain instead.
            if matches!(label, "Profile" | "Regions") {
                visual.update(|window, cx| view.focus(window, cx)).unwrap();
                // A listbox's trigger precedes its panel; a plain popover has
                // no interactive trigger. Exercise the actual focus chain.
                for _ in 0..3 {
                    visual.simulate_keystrokes("tab");
                    settle(&mut visual);
                    if !frames(&mut visual).is_empty() {
                        break;
                    }
                }
            }
            let panel = bounds(&mut visual, &view, role, label);
            let quads = frames(&mut visual);
            assert!(
                !quads.is_empty(),
                "{name}/{label}: focused panel must be decorated"
            );
            assert!(
                quads
                    .iter()
                    .all(|(stroke, width, height)| (*stroke - 2.0).abs() < 0.01
                        && (f64::from(*width) - panel.width).abs() < 0.5
                        && (f64::from(*height) - panel.height).abs() < 0.5),
                "{name}/{label}: {quads:?} vs {panel:?}"
            );
        }
    }
}

#[gpui::test]
fn a_panel_focus_decorator_obeys_the_hosts_never_policy(cx: &mut TestAppContext) {
    for body in [
        r#"dialog::Dialog(#{ key:"confirm", open:true, title:"Confirm", content:text("Continue?") })"#,
        r#"sheet::Sheet(#{ key:"inspector", open:true, title:"Inspector", content:text("Details"), initial_focus:"panel" })"#,
    ] {
        let (mut visual, _) = mount_with_indicators(cx, body, FocusIndicators::Never);
        assert!(frames(&mut visual).is_empty());
        assert!(visual.update(|window, cx| window.focused(cx).is_some()));
    }
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Frame, 2);
}

/// Quads painted with a visible (left) border in the focus color: (border width, width,
/// height) in logical pixels.
fn frames(visual: &mut VisualTestContext) -> Vec<(f32, f32, f32)> {
    let color: gpui::Hsla = rgba(RING).into();
    painted_quads(visual)
        .iter()
        .filter(|quad| quad.border_widths[3] > 0.0 && quad.border_color == color)
        .map(|quad| (quad.border_widths[3], quad.bounds.width, quad.bounds.height))
        .collect()
}

fn panel_width(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    nodes(visual, view)
        .into_iter()
        .find(|node| node.role == role)
        .and_then(|node| node.geometry)
        .map(|geometry| geometry.visual.width as f32)
        .unwrap_or_else(|| panic!("no {role} node"))
}

#[gpui::test]
fn a_dialog_panel_holding_focus_shows_the_frame(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"dialog::Dialog(#{ key: "confirm", open: true, title: "Confirm",
            content: text("Continue?"),
            actions: [button::Button(#{ key: "ok", text: "OK", on_click: Fn("noop") })] })"#,
    );
    let width = panel_width(&mut visual, &view, "dialog");
    let found = frames(&mut visual);
    println!("panel {width}, frames {found:?}");
    assert!(
        found
            .iter()
            .any(|(border, w, _)| (*border - 2.0).abs() < 0.005 && (*w - width).abs() < 0.25),
        "the panel holds focus: a 2px frame the panel's width, got {found:?}"
    );
    // One decorator paints after the panel and its children; it must not require a
    // second, layout-affecting focus border to repair overpainting.
    assert!(
        found
            .iter()
            .all(|(border, _, _)| (*border - 2.0).abs() < 0.005)
    );

    // Tab moves focus to the button: the button shows its own frame, the panel none.
    visual.simulate_keystrokes("tab");
    settle(&mut visual);
    let found = frames(&mut visual);
    println!("after tab {found:?}");
    assert!(
        !found.iter().any(|(_, w, _)| (*w - width).abs() < 0.25),
        "focus inside the panel: no panel frame, got {found:?}"
    );
    assert!(!found.is_empty(), "the focused button shows its frame");
}

#[gpui::test]
fn a_sheet_panel_holding_focus_shows_the_frame(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"sheet::Sheet(#{ key: "inspector", open: true, side: "end", title: "Inspector",
            content: text("Details"), initial_focus: "panel" })"#,
    );
    let width = panel_width(&mut visual, &view, "dialog");
    let found = frames(&mut visual);
    println!("panel {width}, frames {found:?}");
    assert!(
        found
            .iter()
            .any(|(border, w, _)| (*border - 2.0).abs() < 0.005 && (*w - width).abs() < 0.25),
        "the sheet holds focus: a 2px frame the panel's width, got {found:?}"
    );
}

#[gpui::test]
fn a_dialog_opened_by_a_click_shows_the_frame_without_another_input(cx: &mut TestAppContext) {
    // The panel takes focus while its first frame is laid out, when GPUI ignores the
    // refresh `focus()` asks for; the overlay must ask for the next frame itself.
    let (mut visual, view) = mount(
        cx,
        r#"button::Button(#{ key: "open", text: "Open", on_click: Fn("open") }),
        dialog::Dialog(#{ key: "confirm", open: ctx.get_state("open"), title: "Confirm",
            content: text("Continue?"), on_open_change: Fn("noop") })"#,
    );
    let open = bounds(&mut visual, &view, "button", "Open");
    click_center(&mut visual, open);
    // No forced frames: only what the window asked for.
    visual.run_until_parked();
    let width = panel_width(&mut visual, &view, "dialog");
    let found = frames(&mut visual);
    println!("panel {width}, frames {found:?}");
    assert!(
        found
            .iter()
            .any(|(border, w, _)| (*border - 2.0).abs() < 0.005 && (*w - width).abs() < 0.25),
        "the frame shows on the frame after the dialog opened: {found:?}"
    );
}

#[gpui::test]
fn a_dialog_is_as_wide_as_it_asks_and_at_most_ninety_percent_of_the_window(
    cx: &mut TestAppContext,
) {
    // The panel asks for 420px; the overlay caps it at 90% of the backdrop. Before, the
    // cap resolved against the panel's own wrapper and every dialog was 378px.
    let (mut visual, view) = mount(
        cx,
        r#"dialog::Dialog(#{ key: "confirm", open: true, title: "Confirm",
            content: text("Continue?") })"#,
    );
    let width = panel_width(&mut visual, &view, "dialog");
    assert!((width - 420.0).abs() < 0.5, "dialog width {width}");
    let (mut visual, view) = mount(
        cx,
        r#"dialog::Dialog(#{ key: "wide", open: true, title: "Wide",
            content: text("Continue?"), part_styles: #{ panel: style().width(px(4000)) } })"#,
    );
    let width = panel_width(&mut visual, &view, "dialog");
    let viewport = visual.update(|window, _| f32::from(window.viewport_size().width));
    assert!(
        (width - viewport * 0.9).abs() < 1.0,
        "a dialog wider than the window: {width} in a {viewport} window"
    );
}
