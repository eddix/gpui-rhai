//! Table's render-ahead settings over its virtual body: `batch_rows` renders the rows a scroll
//! needs in batches, `retain_rows` keeps the rows a scroll left and brings them back without
//! rendering until the Table renders again, and the Host's virtual collection policy applies
//! where a Table sets nothing.

use gpui::{
    Pixels, Point, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px,
};
use gpui_rhai::{
    ExecutionOperation, RuntimeEngine, ScriptViewExtension, ScriptViewHandle,
    VirtualCollectionPolicy,
};

use super::{mount_refreshed, table};
use crate::support::{Step, bounds, center, click_center, settle};

/// 400 rows in a 320px Table, `props` added to it. The Select button selects row 50, which
/// renders the Table again; a selected row is revealed, so it is one a scroll has reached.
fn script(props: &str) -> String {
    format!(
        r#"import "components/table" as table;
fn state_schema(){{#{{fields:#{{selected:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn select(ctx,payload){{ctx.set_state("selected","r50");}}
fn rows(){{let result=[];for index in 0..400{{result.push(#{{id:`r${{index}}`,label:`Row ${{index}}`}});}}result}}
fn view(ctx){{
    let selected=ctx.get_state("selected");
    column([
        table::Table(#{{key:"rows",label:"Rows",row_key:"id",height:320,rows:rows(),{props}
            columns:[#{{key:"label",title:"Label",width:#{{kind:"fixed",value:200}}}}],
            selection_mode:"single",selected_keys:if selected=="" {{[]}} else {{[selected]}}}}),
        text("Select").accessibility_role("button").accessibility_label("Select").on_click(Fn("select")),
    ]).with_style(style().width(px(400)))
}}"#
    )
}

struct Policy(VirtualCollectionPolicy);

impl ScriptViewExtension for Policy {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine.set_virtual_collection_policy(self.0);
        Ok(())
    }
}

struct Scrolled {
    visual: VisualTestContext,
    view: ScriptViewHandle,
    at: Point<Pixels>,
}

impl Scrolled {
    fn mount(
        cx: &mut TestAppContext,
        props: &str,
        policy: Option<VirtualCollectionPolicy>,
    ) -> Self {
        let mut mount = table(script(props));
        if let Some(policy) = policy {
            mount = mount.with(move |view| view.extension(Policy(policy)));
        }
        let (window, view) = mount_refreshed(cx, mount);
        let mut visual = VisualTestContext::from_window(*window, cx);
        settle(&mut visual, Step::Tick, 2);
        let at = center(bounds(&mut visual, &view, "table", "Rows"));
        let mut scrolled = Self { visual, view, at };
        scrolled.renders();
        scrolled
    }

    /// The rows the renderer rendered since the last call.
    fn renders(&mut self) -> usize {
        self.visual
            .update(|_, cx| self.view.take_performance_snapshot(cx).unwrap())
            .timings
            .iter()
            .filter(|timing| matches!(timing.operation, ExecutionOperation::VirtualCollection(_)))
            .count()
    }

    /// Scroll `steps` rows of 32px, down or up. Returns how many steps rendered rows and how
    /// many rows they rendered.
    fn scroll(&mut self, steps: usize, down: bool) -> (usize, usize) {
        let (mut rendering, mut rows) = (0, 0);
        for _ in 0..steps {
            self.visual.simulate_event(ScrollWheelEvent {
                position: self.at,
                delta: ScrollDelta::Pixels(point(px(0.0), px(if down { -32.0 } else { 32.0 }))),
                ..ScrollWheelEvent::default()
            });
            settle(&mut self.visual, Step::Tick, 2);
            let rendered = self.renders();
            rendering += usize::from(rendered > 0);
            rows += rendered;
        }
        (rendering, rows)
    }
}

#[gpui::test]
fn batch_rows_render_a_scroll_in_batches(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (one_steps, one_rows) =
        Scrolled::mount(cx, "batch_rows:1,retain_rows:0,", None).scroll(64, true);
    let (batch_steps, batch_rows) =
        Scrolled::mount(cx, "batch_rows:16,retain_rows:0,", None).scroll(64, true);
    eprintln!(
        "batch 1: {one_steps} steps, {one_rows} rows; batch 16: {batch_steps} steps, {batch_rows} rows"
    );
    // Row by row, nearly every step renders; in batches of 16, about one step in 16.
    assert!(
        one_steps >= 48,
        "batch 1 rendered on {one_steps} of 64 steps"
    );
    assert!(
        batch_steps * 4 <= one_steps,
        "batch 16 rendered on {batch_steps} of 64 steps"
    );
    // The same rows either way, give or take one batch rendered ahead.
    assert!(
        batch_rows <= one_rows + 16,
        "{batch_rows} rows against {one_rows}"
    );
}

#[gpui::test]
fn retained_rows_come_back_without_rendering(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut kept = Scrolled::mount(cx, "batch_rows:1,retain_rows:200,", None);
    kept.scroll(48, true);
    let (_, back) = kept.scroll(48, false);
    assert_eq!(
        back, 0,
        "the rows a scroll left come back without rendering"
    );
    bounds(&mut kept.visual, &kept.view, "text", "Row 0");

    let mut released = Scrolled::mount(cx, "batch_rows:1,retain_rows:0,", None);
    released.scroll(48, true);
    let (_, back) = released.scroll(48, false);
    assert!(
        back >= 32,
        "with none kept, the way back renders {back} rows"
    );
}

#[gpui::test]
fn a_new_render_drops_the_kept_rows(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut table = Scrolled::mount(cx, "batch_rows:1,retain_rows:200,", None);
    table.scroll(48, true);
    let select = bounds(&mut table.visual, &table.view, "button", "Select");
    click_center(&mut table.visual, select);
    settle(&mut table.visual, Step::Tick, 2);
    table.renders();
    // The selected row is in view: the selection did not move the rows.
    bounds(&mut table.visual, &table.view, "text", "Row 50");
    let (_, back) = table.scroll(48, false);
    assert!(
        back >= 32,
        "after the selection rendered the Table, the way back renders {back} rows"
    );
}

#[gpui::test]
fn the_host_policy_applies_where_a_table_sets_nothing(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let host = VirtualCollectionPolicy {
        overscan_items: None,
        batch_items: Some(1),
        retain_items: Some(0),
    };
    let (host_steps, _) = Scrolled::mount(cx, "", Some(host)).scroll(32, true);
    assert!(
        host_steps >= 24,
        "the Host's batch of one rendered on {host_steps} of 32 steps"
    );
    let (own_steps, _) = Scrolled::mount(cx, "batch_rows:16,", Some(host)).scroll(32, true);
    assert!(
        own_steps * 4 <= host_steps,
        "the Table's own batch rendered on {own_steps} of 32 steps"
    );
}
