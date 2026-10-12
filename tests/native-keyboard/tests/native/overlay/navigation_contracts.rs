//! Menu, ContextMenu, Command, AlertDialog, AppShell and AnimatedTabs keep the contracts
//! their schemas declare: declared parts reach the node they name, a row whose action is
//! disabled is disabled for the keys too, every activated menu row closes the menu, a
//! ContextMenu opens from the keyboard, an AlertDialog reports `cancel` when dismissed, a
//! disabled Command takes no input, F6 moves on from the region that holds focus, and a
//! callback reaches its caller through a nested component.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
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

/// The marker color for part styles and, through `focus_ring`, for focus frames.
const MARK: u32 = 0x00ff_00ff;

fn mount(cx: &mut TestAppContext, body: &str) -> (VisualTestContext, ScriptViewHandle) {
    try_mount(cx, body).unwrap()
}

/// Mounts `body` as the module's tail after a `log` state field, a `note(ctx, line)` helper
/// and a disabled `demo.run` action; a failed first render is the error.
fn try_mount(
    cx: &mut TestAppContext,
    body: &str,
) -> Result<(VisualTestContext, ScriptViewHandle), String> {
    cx.update(gpui_rhai::install);
    let source = format!(
        r#"import "components/menu" as menu;
import "components/context_menu" as context_menu;
import "components/command" as command;
import "components/alert_dialog" as alert_dialog;
import "components/button" as button;
import "components/tab_bar" as tab_bar;
import "patterns/app_shell" as app_shell;
import "motion/animated_tabs" as animated_tabs;
fn state_schema() {{ #{{ fields: #{{
    log: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "" }} }},
    tab: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "one" }} }} }} }} }}
fn init(ctx) {{
    ctx.register_action("demo.run", Fn("ran"));
    ctx.set_action_enabled("demo.run", false);
}}
fn note(ctx, line) {{ ctx.set_state("log", `${{ctx.get_state("log")}} ${{line}}`); }}
fn ran(ctx, payload) {{ note(ctx, "ran"); }}
fn acted(ctx, value) {{ note(ctx, `action:${{value}}`); }}
fn moved(ctx, value) {{ note(ctx, `active:${{value}}`); }}
fn opened(ctx, open) {{ note(ctx, `open:${{open}}`); }}
fn noop(ctx, value) {{ () }}
{body}"#
    );
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), source)]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        .chain(gpui_rhai_registry::BUNDLED_MOTION_SOURCES_BY_ID)
    {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(MARK))]),
        ..Default::default()
    };
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .theme_token_overrides(overrides)
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
    let fallback = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(BTreeMap::from([(
            ModuleId::parse("main").unwrap(),
            "fn view(ctx) { column([]) }".to_owned(),
        )])),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let failure = Rc::new(RefCell::new(None));
    let fail = failure.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("contracts", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("contracts"), host.clone(), window, cx)
            .unwrap_or_else(|error| {
                *fail.borrow_mut() = Some(error.to_string());
                fallback
                    .mount(ScriptViewConfig::new("empty"), host.clone(), window, cx)
                    .unwrap()
            });
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    if let Some(error) = failure.borrow().clone() {
        return Err(error);
    }
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    Ok((visual, view))
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn last_error(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Option<String> {
    visual.update(|_, cx| view.last_error(cx).unwrap())
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.test_id.as_deref() == Some("log"))
            .map(|node| node.name.trim().to_owned())
            .unwrap_or_default()
    })
}

fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .and_then(|node| node.geometry)
            .unwrap_or_else(|| panic!("no {role} {name:?}"))
            .visual
    })
}

fn click(visual: &mut VisualTestContext, bounds: GeometryBounds) {
    #[allow(clippy::cast_possible_truncation)]
    let at = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_mouse_move(at, None, Modifiers::none());
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(visual);
}

fn keys(visual: &mut VisualTestContext, keys: &str) {
    visual.simulate_keystrokes(keys);
    settle(visual);
}

/// Whether some painted quad is filled with `color`.
fn painted(visual: &mut VisualTestContext, color: u32) -> bool {
    let fill: gpui::Hsla = gpui::rgba(color).into();
    visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .any(|quad| quad.background == fill.into())
    })
}

/// The widest quad framed in the marker color, as `(x, width)` in logical pixels.
fn widest_frame(visual: &mut VisualTestContext) -> Option<(f64, f64)> {
    let color: gpui::Hsla = gpui::rgba(MARK).into();
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.border_widths.left.0 > 0.0 && quad.border_color == color)
            .map(|quad| {
                (
                    f64::from(quad.bounds.origin.x.0 / scale),
                    f64::from(quad.bounds.size.width.0 / scale),
                )
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
    })
}

const MENU_ITEMS: &str = r#"[
    #{ kind: "item", value: "run", label: "Run", action: "demo.run" },
    #{ kind: "item", value: "other", label: "Other" }
]"#;

#[gpui::test]
fn context_menu_parts_reach_the_menu_rows_and_panel(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &format!(
            r#"fn view(ctx) {{ column([
    context_menu::ContextMenu(#{{ key: "rows", label: "Row actions", trigger: text("Area"),
        open: true, active_value: "", items: {MENU_ITEMS}, on_open_change: Fn("opened"),
        part_styles: #{{ item: style().background(rgba(0x00ff00ff)),
            content: style().background(rgba(0x0000ffff)) }} }}),
]).with_style(style().width(px(600)).height(px(400))) }}"#
        ),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        painted(&mut visual, MARK),
        "part_styles.item reaches the menu rows"
    );
    assert!(
        painted(&mut visual, 0x0000_ffff),
        "part_styles.content reaches the menu panel"
    );
}

#[gpui::test]
fn menu_keys_skip_and_do_not_activate_a_row_whose_action_is_disabled(cx: &mut TestAppContext) {
    let script = |active: &str| {
        format!(
            r#"fn view(ctx) {{ column([
    menu::Menu(#{{ key: "m", label: "Actions", trigger: text("Actions"), open: true,
        active_value: "{active}", items: {MENU_ITEMS}, on_action: Fn("acted"),
        on_active_change: Fn("moved"), on_open_change: Fn("opened") }}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(600)).height(px(400))) }}"#
        )
    };
    // The row draws disabled, so Down from the other row wraps back to it.
    let (mut visual, view) = mount(cx, &script("other"));
    keys(&mut visual, "down");
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "active:other");
    // Enter on it dispatches nothing, rather than failing on the disabled action.
    let (mut visual, view) = mount(cx, &script("run"));
    keys(&mut visual, "enter");
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "");
}

fn command_script(extra: &str, items: &str) -> String {
    format!(
        r#"fn view(ctx) {{ column([
    command::Command(#{{ key: "palette", label: "Commands", query: "", active_value: "run",
        autofocus: true, items: {items}, on_query_change: Fn("noop"),
        on_active_change: Fn("moved"), on_action: Fn("acted") {extra} }}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(600)).height(px(400))) }}"#
    )
}

#[gpui::test]
fn command_keys_pass_over_a_row_whose_action_is_disabled(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &command_script(
            "",
            r#"[#{ value: "run", label: "Run", action: "demo.run" }, #{ value: "b", label: "B" }]"#,
        ),
    );
    // The highlight starts on the first enabled command, and Enter runs that one.
    keys(&mut visual, "enter");
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "action:b");
}

#[gpui::test]
fn command_item_accepts_an_explicit_unit_shortcut(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &command_script("", r#"[#{ value: "a", label: "A", shortcut: () }]"#),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    bounds(&mut visual, &view, "option", "A");
}

#[gpui::test]
fn a_disabled_command_ignores_row_clicks(cx: &mut TestAppContext) {
    let items = r#"[#{ value: "a", label: "A" }, #{ value: "b", label: "B" }]"#;
    // The control: an enabled command runs the clicked row.
    let (mut visual, view) = mount(cx, &command_script("", items));
    let row = bounds(&mut visual, &view, "option", "B");
    click(&mut visual, row);
    assert_eq!(log(&mut visual, &view), "active:b action:b");
    let (mut visual, view) = mount(cx, &command_script(", disabled: true", items));
    let row = bounds(&mut visual, &view, "option", "B");
    click(&mut visual, row);
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "");
}

#[gpui::test]
fn alert_dialog_actions_part_reaches_the_button_row(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"fn view(ctx) { column([
    alert_dialog::AlertDialog(#{ key: "confirm", open: true, title: "Delete?",
        on_open_change: Fn("opened"),
        part_styles: #{ actions: style().background(rgba(0x00ff00ff)) } }),
]).with_style(style().width(px(600)).height(px(400))) }"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        painted(&mut visual, MARK),
        "part_styles.actions reaches the button row"
    );
}

#[gpui::test]
fn app_shell_status_takes_as_many_fields_as_status_bar(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"fn fields() { let result = []; for index in 0..16 { result.push(text(`${index}`)); } result }
fn view(ctx) {
    app_shell::AppShell(#{ key: "shell", label: "Tool", main: text("Main"),
        status: #{ start: fields(), center: fields(), end: fields() } })
}"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn f6_moves_on_from_the_region_that_holds_focus(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"fn pane(name) {
    column([button::Button(#{ key: name, text: name, on_click: Fn("noop") })])
}
fn view(ctx) {
    app_shell::AppShell(#{ key: "shell", label: "Tool", sidebar: pane("Sidebar"),
        main: pane("Main"), inspector: pane("Inspector"), sidebar_width: px(200),
        inspector_width: px(240) })
}"#,
    );
    let sidebar = bounds(&mut visual, &view, "button", "Sidebar");
    let inspector = bounds(&mut visual, &view, "button", "Inspector");
    let framing = |visual: &mut VisualTestContext, target: GeometryBounds| {
        visual.update(|window, _| window.refresh());
        settle(visual);
        let center = target.x + target.width / 2.0;
        widest_frame(visual).is_some_and(|(x, width)| x <= center && center <= x + width)
    };
    // A press in the inspector focuses it without F6; the shell still stores the sidebar.
    click(&mut visual, inspector);
    assert!(
        framing(&mut visual, inspector),
        "the press focuses the inspector"
    );
    keys(&mut visual, "f6");
    assert!(
        framing(&mut visual, sidebar),
        "F6 in the inspector wraps to the sidebar: frame {:?}",
        widest_frame(&mut visual)
    );
    keys(&mut visual, "shift-f6");
    assert!(
        framing(&mut visual, inspector),
        "Shift+F6 in the sidebar wraps to the inspector: frame {:?}",
        widest_frame(&mut visual)
    );
}

#[gpui::test]
fn animated_tabs_change_reaches_the_caller_through_tabs(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"fn chose(ctx, value) { ctx.set_state("tab", value); note(ctx, `change:${value}`); }
fn view(ctx) { column([
    animated_tabs::AnimatedTabs(#{ key: "views", label: "Views", value: ctx.get_state("tab"),
        tabs: [#{ value: "one", label: "One", content: text("First") },
            #{ value: "two", label: "Two", content: text("Second") }],
        on_change: Fn("chose") }),
    text(ctx.get_state("log")).test_id("log"),
]) }"#,
    );
    let two = bounds(&mut visual, &view, "tab", "Two");
    click(&mut visual, two);
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "change:two");
    bounds(&mut visual, &view, "text", "Second");
}

#[gpui::test]
fn tab_bar_rejects_two_tabs_with_one_value(cx: &mut TestAppContext) {
    let error = try_mount(
        cx,
        r#"fn view(ctx) {
    tab_bar::TabBar(#{ key: "files", label: "Files", value: "a",
        tabs: [#{ value: "a", label: "A" }, #{ value: "a", label: "B" }] })
}"#,
    )
    .err();
    assert!(
        error
            .as_deref()
            .is_some_and(|error| error.contains("duplicate key `a`")),
        "{error:?}"
    );
}

fn menu_script(active: &str) -> String {
    format!(
        r#"fn view(ctx) {{ column([
    menu::Menu(#{{ key: "m", label: "Actions", trigger: text("Actions"), open: true,
        active_value: "{active}", items: {MENU_ITEMS}, on_action: Fn("acted"),
        on_active_change: Fn("moved"), on_open_change: Fn("opened") }}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(600)).height(px(400))) }}"#
    )
}

#[gpui::test]
fn a_menu_row_without_an_action_closes_the_menu_after_reporting_it(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &menu_script(""));
    let other = bounds(&mut visual, &view, "menuitem", "Other");
    click(&mut visual, other);
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "action:other open:false");
    // Enter on the highlighted row does the same.
    let (mut visual, view) = mount(cx, &menu_script("other"));
    keys(&mut visual, "enter");
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "action:other open:false");
}

#[gpui::test]
fn enter_on_a_submenu_row_reports_it_and_keeps_the_menu_open(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"fn view(ctx) {
    let child = menu::Menu(#{ key: "more", label: "More actions", parent_overlay: "m",
        trigger: text("More"), open: false, active_value: "", placement: "right",
        items: [#{ kind: "item", value: "export", label: "Export" }], on_action: Fn("acted") });
    column([
        menu::Menu(#{ key: "m", label: "Actions", trigger: text("Actions"), open: true,
            active_value: "more", on_action: Fn("acted"), on_open_change: Fn("opened"),
            items: [#{ kind: "item", value: "other", label: "Other" },
                #{ kind: "submenu", value: "more", label: "More", submenu: child }] }),
        text(ctx.get_state("log")).test_id("log"),
    ]).with_style(style().width(px(600)).height(px(400)))
}"#,
    );
    keys(&mut visual, "enter");
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "action:more");
}

#[gpui::test]
fn context_menu_opens_from_the_keyboard_at_its_trigger(cx: &mut TestAppContext) {
    for key in ["shift-f10", "menu"] {
        let (mut visual, view) = mount(
            cx,
            &format!(
                r#"fn view(ctx) {{ column([
    context_menu::ContextMenu(#{{ key: "rows", label: "Row actions",
        trigger: row([text("Area")]).with_style(style().width(px(200)).height(px(40)))
            .accessibility_role("group").accessibility_label("Area target"),
        open: ctx.get_state("log").ends_with("open:true"), active_value: "other",
        items: {MENU_ITEMS}, on_action: Fn("acted"), on_open_change: Fn("opened") }}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().padding(px(24)).width(px(600)).height(px(400))) }}"#
            ),
        );
        let area = bounds(&mut visual, &view, "group", "Area target");
        // The trigger area is the tab stop; the key opens the menu against its bounds.
        visual.update(|window, cx| window.focus_next(cx));
        settle(&mut visual);
        keys(&mut visual, key);
        assert_eq!(last_error(&mut visual, &view), None);
        assert_eq!(log(&mut visual, &view), "open:true", "{key}");
        let anchor = visual.update(|_, cx| {
            let root = view.root(cx).unwrap().unwrap();
            let UiNodeKind::Box { children } = root.kind() else {
                unreachable!()
            };
            let UiNodeKind::Overlay { spec, .. } = children[0].kind() else {
                unreachable!()
            };
            spec.anchor.unwrap()
        });
        assert_eq!(
            (anchor.x, anchor.y, anchor.width, anchor.height),
            (area.x, area.y, area.width, area.height),
            "{key}"
        );
        keys(&mut visual, "escape");
        assert_eq!(log(&mut visual, &view), "open:true open:false", "{key}");
    }
}

#[gpui::test]
fn alert_dialog_dismissal_reports_cancel_before_closing(cx: &mut TestAppContext) {
    let script = r#"fn cancelled(ctx, payload) { note(ctx, "cancel"); }
fn confirmed(ctx, payload) { note(ctx, "confirm"); }
fn view(ctx) { column([
    alert_dialog::AlertDialog(#{ key: "confirm", open: true, title: "Delete?",
        on_cancel: Fn("cancelled"), on_confirm: Fn("confirmed"), on_open_change: Fn("opened") }),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(600)).height(px(400))) }"#;
    let (mut visual, view) = mount(cx, script);
    keys(&mut visual, "escape");
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "cancel open:false");
    // A press on the backdrop, outside the centered panel.
    let (mut visual, view) = mount(cx, script);
    click(
        &mut visual,
        GeometryBounds {
            x: 4.0,
            y: 4.0,
            width: 2.0,
            height: 2.0,
        },
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "cancel open:false");
    // The buttons still report themselves once.
    let (mut visual, view) = mount(cx, script);
    let confirm = bounds(&mut visual, &view, "button", "Continue");
    click(&mut visual, confirm);
    assert_eq!(log(&mut visual, &view), "confirm open:false");
}

/// Escape closes a menu and gives focus back to where it was when the menu
/// opened, so the same key opens it again.
#[gpui::test]
fn a_closed_menu_returns_focus_to_where_it_opened(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &format!(
            r#"fn view(ctx) {{ column([
    menu::Menu(#{{ key: "m", label: "Actions", trigger: text("Actions"),
        open: ctx.get_state("log").ends_with("open:true"), active_value: "",
        items: {MENU_ITEMS}, on_action: Fn("acted"), on_active_change: Fn("moved"),
        on_open_change: Fn("opened") }}),
    context_menu::ContextMenu(#{{ key: "rows", label: "Row actions",
        trigger: row([text("Area")]).with_style(style().width(px(200)).height(px(40))),
        open: ctx.get_state("log").ends_with("ctx:true"), active_value: "",
        items: {MENU_ITEMS}, on_action: Fn("acted"), on_open_change: Fn("context_opened") }}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().padding(px(24)).width(px(600)).height(px(400))) }}
fn context_opened(ctx, open) {{ note(ctx, `ctx:${{open}}`); }}"#
        ),
    );
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    keys(&mut visual, "enter");
    keys(&mut visual, "escape");
    keys(&mut visual, "enter");
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "open:true open:false open:true");
    keys(&mut visual, "escape");
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    keys(&mut visual, "shift-f10");
    keys(&mut visual, "escape");
    keys(&mut visual, "shift-f10");
    assert_eq!(
        log(&mut visual, &view),
        "open:true open:false open:true open:false ctx:true ctx:false ctx:true"
    );
}
