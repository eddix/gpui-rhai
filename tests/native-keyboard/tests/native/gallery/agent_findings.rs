use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    ManualRuntimeClock, MotionPreference, ScriptViewHandle, WallClock, WallClockSource,
};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, bounds, painted_quads_including_rounded_clips, snapshot};

const CACHE: &str = "Cache key can collide";
const OLD: &str = "Document the empty cache";
const TABLE: &str = "Table footer clips at 150%";
const NEW: &str = "Missing focus indicator";

fn missing(v: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> bool {
    snapshot(v, view)
        .find_by_role_and_name(role, name)
        .next()
        .is_none()
}

fn publish(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    v.executor().advance_clock(Duration::from_millis(670));
    ready(v, view);
    node(v, view, "status", "A new finding arrived.");
}

#[gpui::test]
fn gallery_agent_findings_filter_retains_totals_and_clears_when_owner_leaves(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_findings", "clean-dark");
    v.simulate_resize(size(px(1140.), px(980.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Critical: 1 · Warning: 1");
    assert!(
        node(&mut v, &view, "button", "Storage review")
            .description
            .contains("7 of 20 checks")
    );
    click(&mut v, &view, "button", "Storage review");
    assert_eq!(
        node(&mut v, &view, "button", "Storage review").pressed,
        Some(true)
    );
    assert!(missing(&mut v, &view, "button", TABLE));
    node(&mut v, &view, "button", CACHE);
    node(&mut v, &view, "button", OLD);
    node(&mut v, &view, "status", "Critical: 1 · Warning: 1");
    click(
        &mut v,
        &view,
        "button",
        "Show all agents (now only Storage review)",
    );
    assert_eq!(
        node(&mut v, &view, "button", "Storage review").pressed,
        Some(false)
    );
    node(&mut v, &view, "button", TABLE);
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Storage review").pressed,
        Some(true)
    );
    click(&mut v, &view, "button", "Remove Storage review");
    assert!(missing(
        &mut v,
        &view,
        "button",
        "Show all agents (now only Storage review)"
    ));
    assert!(missing(&mut v, &view, "button", "Storage review"));
    let cache = node(&mut v, &view, "button", CACHE);
    assert!(
        cache.description.contains("index.rs:120") && !cache.description.contains("Storage review")
    );
    click(&mut v, &view, "button", CACHE);
    node(&mut v, &view, "status", "Opened path/边界 at revision 1");
    click(&mut v, &view, "button", "Restore Storage review");
    node(&mut v, &view, "button", TABLE);
    assert_eq!(
        node(&mut v, &view, "button", "Storage review").pressed,
        Some(false)
    );
}

#[gpui::test]
fn gallery_agent_findings_live_insertion_preserves_focus_and_evicts_older_rows(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_findings", "modern-light");
    v.simulate_resize(size(px(1140.), px(980.)));
    ready(&mut v, &view);
    let cache = node(&mut v, &view, "button", CACHE);
    assert!(cache.geometry.unwrap().visual.y < bounds(&mut v, &view, "button", TABLE).y);
    click(&mut v, &view, "button", "Schedule finding");
    click(&mut v, &view, "button", CACHE);
    node(&mut v, &view, "status", "Opened path/边界 at revision 0");
    let focus = v
        .update(|window, cx| window.focused(cx))
        .expect("finding received focus");
    publish(&mut v, &view);
    let fresh = bounds(&mut v, &view, "button", NEW);
    let moved = node(&mut v, &view, "button", CACHE);
    assert!(moved.geometry.unwrap().visual.y > fresh.y);
    assert_eq!(cache.id, moved.id);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Opened path/边界 at revision 1");
    click(&mut v, &view, "button", "Schedule finding");
    publish(&mut v, &view);
    node(&mut v, &view, "button", "Follow-up finding 2");
    assert!(missing(&mut v, &view, "button", OLD));
    node(&mut v, &view, "status", "Critical: 1 · Warning: 3");
}

#[gpui::test]
fn gallery_agent_findings_feed_height_moves_following_layout_on_the_native_clock(
    cx: &mut TestAppContext,
) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "agent_findings".into(),
        theme: "modern-dark".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(manual.clock())
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("findings-clock", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1100.), px(940.)));
    ready(&mut v, &view);
    manual.advance(Duration::from_millis(900));
    ready(&mut v, &view);
    let y = |v: &mut VisualTestContext| bounds(v, &view, "button", "Schedule finding").y;
    let before = y(&mut v);
    click(&mut v, &view, "button", "Schedule finding");
    manual.advance(Duration::from_millis(650));
    publish(&mut v, &view);
    let start = y(&mut v);
    assert!(
        (before - start).abs() < 1.0,
        "first inserted frame jumped: {before} → {start}"
    );
    manual.advance(Duration::from_millis(140));
    ready(&mut v, &view);
    let mid = y(&mut v);
    manual.advance(Duration::from_millis(550));
    ready(&mut v, &view);
    let end = y(&mut v);
    assert!(
        mid > before + 2.0 && end > mid + 3.0 && end > before + 30.0,
        "feed height did not animate: {before} {start} {mid} {end}"
    );
}

#[gpui::test]
fn agent_findings_read_only_feed_bounds_and_narrow_metadata_are_not_focusable(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = Mount::components(r#"
        import "patterns/agent_findings" as f;
        fn view(ctx){column([f::AgentFindings(#{key:"result",run:"r",revision:1,now_ms:90000,rows:2,
            agents:[#{id:"g",name:"Arabic agent",done:20,total:30}],
            findings:[
                #{id:"timeless",agent:"g",title:"Missing time",severity:"note"},
                #{id:"second",agent:"g",title:"Second finding",severity:"warning",at_ms:52000},
                #{id:"first",agent:"left",title:"First finding",location:"catalog.rs:12",severity:"critical",at_ms:87000},
            ]})]).with_style(style().width(px(300)))}
    "#).registry("patterns/agent_findings").theme(gpui_rhai_registry::CLEAN_DARK_THEME).motion(MotionPreference::None).mount(cx);
    ready(&mut v, &view);
    let first = node(&mut v, &view, "text", "First finding");
    let second = node(&mut v, &view, "text", "Second finding");
    assert!(first.geometry.unwrap().visual.y < second.geometry.unwrap().visual.y);
    assert!(first.description.contains("Critical") && first.description.contains("catalog.rs:12"));
    assert!(missing(&mut v, &view, "button", "First finding"));
    assert!(missing(&mut v, &view, "text", "Missing time"));
    assert!(
        missing(&mut v, &view, "text", "catalog.rs:12"),
        "narrow mode removes the visual metadata cell"
    );
}

#[gpui::test]
fn agent_findings_shared_progress_driver_keeps_native_progress_continuous(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut v, view) = Mount::components(r#"
        import "components/progress" as p;
        fn state_schema() { #{fields:#{value:#{schema:#{type:"integer"},"default":#{type:"integer",value:25}}}} }
        fn advance(ctx,e){ctx.set_state("value",75);}
        fn view(ctx){
            column([p::Progress(#{key:"progress",label:"Verification progress",width:160,value:ctx.get_state("value")})
                .with_part_style("indicator",style().background(rgba(0xfe02fffe))),
                text("Advance").accessibility_role("button").on_click(Fn("advance"))])
        }
    "#).motion(MotionPreference::Normal).with(move |v| v.runtime_clock(clock)).mount(cx);
    ready(&mut v, &view);
    let width = |v: &mut VisualTestContext| {
        let rect = bounds(v, &view, "progressbar", "Verification progress");
        painted_quads_including_rounded_clips(v)
            .into_iter()
            .find(|q| {
                (f64::from(q.bounds.x) - rect.x).abs() < 0.6
                    && (f64::from(q.bounds.y) - rect.y).abs() < 0.6
                    && q.bounds.height >= 1.0
                    && q.bounds.width < 158.0
            })
            .expect("native progress painted an indicator")
            .visible()
            .width
    };
    manual.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    assert!((width(&mut v) - 40.0).abs() < 2.0);
    click(&mut v, &view, "button", "Advance");
    assert!((width(&mut v) - 40.0).abs() < 2.0);
    manual.advance(Duration::from_millis(70));
    ready(&mut v, &view);
    let mid = width(&mut v);
    assert!(mid > 42.0 && mid < 118.0, "mid: {mid}");
    manual.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    assert!((width(&mut v) - 120.0).abs() < 2.0);
}

#[gpui::test]
fn agent_findings_relative_time_uses_the_optional_host_clock(cx: &mut TestAppContext) {
    #[derive(Debug)]
    struct Epoch(Rc<Cell<i64>>);
    impl WallClockSource for Epoch {
        fn now_ms(&self) -> i64 {
            self.0.get()
        }
    }
    let epoch = Rc::new(Cell::new(91_000));
    let clock = WallClock::from_source(Epoch(Rc::clone(&epoch)));
    let (mut v, view) = Mount::components(
        r#"
        import "patterns/agent_findings" as f;
        fn view(ctx){column([f::AgentFindings(#{key:"live",run:"r",revision:0,agents:[],
            findings:[#{id:"p",agent:"",title:"Timed finding",severity:"note",at_ms:90000}]}),
            f::AgentFindings(#{key:"fixed",run:"r",revision:0,now_ms:91000,agents:[],
            findings:[#{id:"p",agent:"",title:"Fixed finding",severity:"note",at_ms:90000}]})])}
    "#,
    )
    .registry("patterns/agent_findings")
    .motion(MotionPreference::None)
    .with(move |v| v.wall_clock(clock))
    .mount(cx);
    ready(&mut v, &view);
    assert!(
        node(&mut v, &view, "text", "Timed finding")
            .description
            .contains("Now")
    );
    epoch.set(115_000);
    v.executor().advance_clock(Duration::from_millis(5_200));
    ready(&mut v, &view);
    assert!(
        node(&mut v, &view, "text", "Timed finding")
            .description
            .contains("25s")
    );
    assert!(
        node(&mut v, &view, "text", "Fixed finding")
            .description
            .contains("Now")
    );
}
