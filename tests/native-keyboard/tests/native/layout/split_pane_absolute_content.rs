//! Absolutely positioned content inside either SplitPane pane paints at its offset from that
//! pane (#140): horizontal and vertical, with and without a `handle` grip, LTR and RTL, and
//! with the other pane collapsed.
//!
//! Each pane gets a 40x30 probe at (10, 10). `direct` makes the probe the pane's content;
//! `filled` puts it in a clipping box that fills the pane with `flex_grow`, the shape of a
//! canvas or treemap stage whose children are all absolute; `wide` also gives that box
//! `width(relative(1.0))`, as #156's treemap viewport does. The panes are flex columns, so
//! that box is as tall as the pane; in a block pane it would collapse to its in-flow content,
//! which is nothing, and clip the probe away.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

use crate::support::{Mount, Step, filled};

const START: u32 = 0xff00_00ff;
const END: u32 = 0x00ff_00ff;

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main)
        .locale("en.rhai", EN_LOCALE)
        .locale("ar.rhai", AR_LOCALE)
        .motion(MotionPreference::None)
        .id("split")
        .mount(cx);
    crate::support::settle(&mut visual, Step::Refresh, 3);
    (visual, view)
}

/// `(x, y, width, height)` in logical pixels.
type Rect = (f32, f32, f32, f32);

/// The visible part (bounds within the content mask) of each quad filled with `color`.
fn visible(visual: &mut VisualTestContext, color: u32) -> Vec<Rect> {
    filled(visual, color)
        .iter()
        .map(|quad| {
            let visible = quad.visible();
            (visible.x, visible.y, visible.width, visible.height)
        })
        .collect()
}

fn bounds(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> Rect {
    let bounds = crate::support::bounds(visual, view, role, name);
    #[allow(clippy::cast_possible_truncation)]
    (
        bounds.x as f32,
        bounds.y as f32,
        bounds.width as f32,
        bounds.height as f32,
    )
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Direct,
    Filled,
    Wide,
}

fn probe(shape: Shape, color: u32) -> String {
    match shape {
        Shape::Direct => format!("probe(rgba({color:#x}))"),
        Shape::Filled => format!("filled(rgba({color:#x}))"),
        Shape::Wide => format!("wide(rgba({color:#x}))"),
    }
}

/// A 420x240 frame holding the SplitPane, so the group's bounds are the frame's.
fn script(locale: &str, props: &str) -> String {
    format!(
        r#"import "components/split_pane" as split_pane;
fn init(ctx) {{ ctx.set_locale("{locale}"); }}
fn probe(color) {{
    box([]).with_style(style().absolute().left(px(10)).top(px(10)).width(px(40)).height(px(30))
        .background(color))
}}
fn filled(color) {{
    box([probe(color)]).with_style(style().flex_grow().relative().overflow_hidden())
}}
fn wide(color) {{
    box([probe(color)]).with_style(style().flex_grow().width(relative(1.0)).relative().overflow_hidden())
}}
fn view(ctx) {{ column([
    box([split_pane::SplitPane(#{{ key: "layout", label: "Resize panels", size: 0.5,
        min_start: 0.0, min_end: 0.0, {props} }})])
        .with_style(style().width(px(420)).height(px(240)))
        .accessibility_role("group").accessibility_label("Frame"),
]).with_style(style().padding(px(20))) }}"#
    )
}

fn assert_probe(visual: &mut VisualTestContext, color: u32, pane: Rect, case: &str) {
    let quads = visible(visual, color);
    let expected = (pane.0 + 10., pane.1 + 10., 40., 30.);
    let near = |a: Rect, b: Rect| {
        (a.0 - b.0).abs() < 0.5
            && (a.1 - b.1).abs() < 0.5
            && (a.2 - b.2).abs() < 0.5
            && (a.3 - b.3).abs() < 0.5
    };
    assert!(
        quads.iter().any(|quad| near(*quad, expected)),
        "{case}: the probe should show whole at {expected:?} in the pane at {pane:?}; \
         visible quads in its colour: {quads:?}"
    );
}

#[gpui::test]
fn absolute_content_paints_in_both_panes(cx: &mut TestAppContext) {
    for shape in [Shape::Direct, Shape::Filled, Shape::Wide] {
        for orientation in ["horizontal", "vertical"] {
            for grip in [false, true] {
                for locale in ["en", "ar"] {
                    let case = format!("{shape:?} {orientation} grip={grip} {locale}");
                    let handle = if grip {
                        "handle: box([]).with_style(style().width(px(6)).height(px(6))),"
                    } else {
                        ""
                    };
                    let main = script(
                        locale,
                        &format!(
                            "orientation: \"{orientation}\", {handle} start: {}, end: {}",
                            probe(shape, START),
                            probe(shape, END)
                        ),
                    );
                    let (mut visual, view) = mount(cx, &main);
                    let group = bounds(&mut visual, &view, "group", "Frame");
                    let separator = bounds(&mut visual, &view, "separator", "Resize panels");
                    let (start, end) = if orientation == "vertical" {
                        let after = separator.1 + separator.3;
                        (
                            (group.0, group.1, group.2, separator.1 - group.1),
                            (group.0, after, group.2, group.1 + group.3 - after),
                        )
                    } else {
                        let before = (group.0, group.1, separator.0 - group.0, group.3);
                        let after_x = separator.0 + separator.2;
                        let after = (after_x, group.1, group.0 + group.2 - after_x, group.3);
                        if locale == "ar" {
                            (after, before)
                        } else {
                            (before, after)
                        }
                    };
                    assert_probe(&mut visual, START, start, &format!("{case}, start pane"));
                    assert_probe(&mut visual, END, end, &format!("{case}, end pane"));
                }
            }
        }
    }
}

#[gpui::test]
fn absolute_content_paints_beside_a_collapsed_pane(cx: &mut TestAppContext) {
    for shape in [Shape::Direct, Shape::Filled, Shape::Wide] {
        for (collapsed, color) in [("start_collapsed", END), ("end_collapsed", START)] {
            let case = format!("{shape:?} {collapsed}");
            let main = script(
                "en",
                &format!(
                    "{collapsed}: true, start: {}, end: {}",
                    probe(shape, START),
                    probe(shape, END)
                ),
            );
            let (mut visual, view) = mount(cx, &main);
            let group = bounds(&mut visual, &view, "group", "Frame");
            assert_probe(&mut visual, color, group, &case);
        }
    }
}

#[gpui::test]
fn committed_ref_geometry_sizes_absolute_content_in_either_pane(cx: &mut TestAppContext) {
    // #156's updated report concerns a stage whose own ref sizes its absolute child,
    // not the absolute positioning itself. Keep an executable control for that feedback.
    const STAGE: &str = r#"
define_component(#{
    metadata: #{id:"tests/measured_stage", "export":"MeasuredStage", version:"0.0.1",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
    schema: #{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},
        color:#{schema:#{type:"integer"},required:true,sensitive:false}},
        state:#{fields:#{}},events:#{},slots:#{},parts:["root"]},
    render:Fn("measured_stage")
});
fn measured_stage(ctx, props) {
    let target = element_ref("mosaic");
    let facts = ctx.element_bounds(target);
    let width = if facts == () {0.0} else {facts.layout.width};
    let height = if facts == () {0.0} else {facts.layout.height};
    box([box([]).with_key("canvas").with_style(style().absolute().left(px(0)).top(px(0))
        .width(px(width)).height(px(height)).background(rgba(props.color)))])
        .with_key("mosaic").with_ref(target)
        .with_style(style().flex_grow().width(relative(1.0)).relative().overflow_hidden())
}
"#;
    for orientation in ["horizontal", "vertical"] {
        for locale in ["en", "ar"] {
            let main = format!(
                "{STAGE}\n{}",
                script(
                    locale,
                    &format!(
                        "orientation: \"{orientation}\", \
                     start: render_component(\"tests/measured_stage\", #{{key:\"start-stage\",color:{START}}}), \
                     end: render_component(\"tests/measured_stage\", #{{key:\"end-stage\",color:{END}}})"
                    )
                )
            );
            let (mut visual, _) = mount(cx, &main);
            crate::support::settle(&mut visual, Step::Tick, 6);
            for color in [START, END] {
                let painted = visible(&mut visual, color);
                assert!(
                    painted
                        .iter()
                        .any(|(_, _, width, height)| *width > 100.0 && *height > 100.0),
                    "{orientation} {locale}: measured stage must paint, got {painted:?}"
                );
            }
        }
    }
}
