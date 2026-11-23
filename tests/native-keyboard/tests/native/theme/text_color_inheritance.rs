//! Observe the native text-style stack, not only a serialized reset flag.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{AnyElement, App, Hsla, IntoElement, Modifiers, Styled, TestAppContext, Window};
use gpui_rhai::*;

use crate::support::{Mount, Step, bounds, center, last_error, settle};

#[gpui::test]
fn inherited_glyph_paint_samples_parent_motion_without_recoloring_explicit_spans(
    cx: &mut TestAppContext,
) {
    use crate::support::glyph_rects_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("inherited_paint_motion"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let clock = ManualRuntimeClock::new(Instant::now());
    let (mut visual, view) = Mount::new(
        r#"fn view(ctx) {
            column([row([
                text("Inherited").accessibility_role("text").accessibility_label("Inherited"),
                text([span("Named ink").color(rgb(0xd04070))])
                    .accessibility_role("text").accessibility_label("Explicit")
            ]).with_style(style().width(px(300)).height(px(52)).gap(px(10)).items_center()
                .text_color(rgb(0x304050)).hover(style().text_color(rgb(0xb0c0d0))))
              .paint_transition(160,"linear").accessibility_role("group").accessibility_label("Row")])
              .with_style(style().padding(px(25)))
        }"#,
    )
    .with({
        let clock = clock.clock();
        move |view| view.runtime_clock(clock)
    })
    .motion(MotionPreference::Normal)
    .mount(&mut native);
    settle(&mut visual, Step::Tick, 4);
    let row = bounds(&mut visual, &view, "group", "Row");
    let plain_glyphs = glyph_rects_with_ink(&mut visual, row, 0x304050ff);
    let authored_glyphs = glyph_rects_with_ink(&mut visual, row, 0xd04070ff);
    assert!(
        plain_glyphs.len() > 5 && authored_glyphs.len() > 5,
        "{row:?}: plain {plain_glyphs:?}, explicit {authored_glyphs:?}"
    );
    visual.simulate_mouse_move(center(row), None, Modifiers::none());
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        glyph_rects_with_ink(&mut visual, row, 0x304050ff),
        plain_glyphs
    );
    clock.advance(Duration::from_millis(80));
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        glyph_rects_with_ink(&mut visual, row, 0x708090ff),
        plain_glyphs
    );
    assert_eq!(
        glyph_rects_with_ink(&mut visual, row, 0xd04070ff),
        authored_glyphs
    );
    clock.advance(Duration::from_millis(80));
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        glyph_rects_with_ink(&mut visual, row, 0xb0c0d0ff),
        plain_glyphs
    );
    assert_eq!(
        glyph_rects_with_ink(&mut visual, row, 0xd04070ff),
        authored_glyphs
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn mixed_rich_text_keeps_dynamic_and_fixed_ink_in_the_same_shaped_line(cx: &mut TestAppContext) {
    use crate::support::glyph_rects_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("mixed_inline_paint_motion"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let clock = ManualRuntimeClock::new(Instant::now());
    let (mut visual, view) = Mount::new(
        r#"fn view(ctx) {
            column([
                text([span("Inherited "), span("Fixed ink").color(rgb(0xd04070)),
                      span(" still inherited").bold()])
                    .with_style(style().width(px(340)).height(px(52))
                        .text_color(rgb(0x304050)).hover(style().text_color(rgb(0xb0c0d0))))
                    .paint_transition(160,"linear").accessibility_role("text")
                    .accessibility_label("Adjacent spans")
            ]).with_style(style().padding(px(25)))
        }"#,
    )
    .with({
        let clock = clock.clock();
        move |view| view.runtime_clock(clock)
    })
    .motion(MotionPreference::Normal)
    .mount(&mut native);
    settle(&mut visual, Step::Tick, 4);
    let region = bounds(&mut visual, &view, "text", "Adjacent spans");
    let inherited = glyph_rects_with_ink(&mut visual, region, 0x304050ff);
    let fixed = glyph_rects_with_ink(&mut visual, region, 0xd04070ff);
    assert!(
        inherited.len() >= 18 && fixed.len() >= 6,
        "positive control"
    );
    visual.simulate_mouse_move(center(region), None, Modifiers::none());
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        glyph_rects_with_ink(&mut visual, region, 0x304050ff),
        inherited,
        "animated paint must not jump to the target before the clock advances"
    );
    clock.advance(Duration::from_millis(80));
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        glyph_rects_with_ink(&mut visual, region, 0x708090ff),
        inherited,
        "both uncolored spans must sample the same midpoint, including the bold span"
    );
    assert_eq!(glyph_rects_with_ink(&mut visual, region, 0xd04070ff), fixed);
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn mixed_rich_ink_survives_wrapping_and_ellipsis(cx: &mut TestAppContext) {
    use crate::support::glyph_rects_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("rich_ink_wrapping"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let clock = ManualRuntimeClock::new(Instant::now());
    let (mut visual, view) = Mount::new(
        r#"fn sample(name, width, ellipsis) {
            let design = style().width(px(width)).text_color(rgb(0x304050))
                .hover(style().text_color(rgb(0xb0c0d0)));
            if ellipsis { design = design.whitespace_nowrap().text_ellipsis(); }
            text([span("Mark "), span("Fixed ").color(rgb(0xd04070)),
                span("Tint ").color(rgba(0x80402080)),
                span("keep reading across several visual rows until the marker")])
                .with_style(design).paint_transition(160,"linear")
                .accessibility_role("text").accessibility_label(name)
        }
        fn view(ctx) {
            column([sample("Wrapped", 108.0, false), sample("Ellipsis", 160.0, true)])
                .with_style(style().padding(px(25)).gap(px(15)))
        }"#,
    )
    .with({
        let clock = clock.clock();
        move |view| view.runtime_clock(clock)
    })
    .motion(MotionPreference::Normal)
    .mount(&mut native);
    settle(&mut visual, Step::Tick, 4);
    let wrapped = bounds(&mut visual, &view, "text", "Wrapped");
    let ellipsized = bounds(&mut visual, &view, "text", "Ellipsis");
    assert!(
        wrapped.height > 2.0 * ellipsized.height,
        "native wrap/truncation witnesses: {wrapped:?} {ellipsized:?}"
    );
    for region in [wrapped, ellipsized] {
        let base = glyph_rects_with_ink(&mut visual, region, 0x304050ff);
        let fixed = glyph_rects_with_ink(&mut visual, region, 0xd04070ff);
        let tinted = glyph_rects_with_ink(&mut visual, region, 0x584038ff);
        assert!(base.len() >= 4 && fixed.len() >= 4 && tinted.len() >= 3);
        visual.simulate_mouse_move(center(region), None, Modifiers::none());
        settle(&mut visual, Step::Tick, 4);
        clock.advance(Duration::from_millis(80));
        settle(&mut visual, Step::Tick, 4);
        assert_eq!(glyph_rects_with_ink(&mut visual, region, 0x708090ff), base);
        // The 50% author ink is composed on the mid-transition color, not
        // on the layout-phase target. Its glyph rects don't move either.
        assert_eq!(
            glyph_rects_with_ink(&mut visual, region, 0x786058ff),
            tinted
        );
        assert_eq!(glyph_rects_with_ink(&mut visual, region, 0xd04070ff), fixed);
        visual.simulate_mouse_move(
            gpui::point(gpui::px(1.), gpui::px(1.)),
            None,
            Modifiers::none(),
        );
        settle(&mut visual, Step::Tick, 4);
        clock.advance(Duration::from_millis(160));
        settle(&mut visual, Step::Tick, 4);
        assert_eq!(glyph_rects_with_ink(&mut visual, region, 0x304050ff), base);
    }
    assert_eq!(last_error(&mut visual, &view), None);
}

struct PaintColor(Rc<Cell<Hsla>>);

impl PrimitiveHandler for PaintColor {
    fn render(
        &mut self,
        _: &PrimitiveInstance,
        _: &PrimitiveContext,
        _: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let painted = self.0.clone();
        Ok(gpui::canvas(
            |_, _, _| (),
            move |_, (), window, _| painted.set(window.text_style().color),
        )
        .w(gpui::px(8.0))
        .h(gpui::px(8.0))
        .into_any_element())
    }
}

struct ProbeExtension(Rc<Cell<Hsla>>);

impl ScriptViewExtension for ProbeExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_primitive(
                PrimitiveDescriptor {
                    id: PrimitiveId::parse("test.text_color").unwrap(),
                    export: "TextColor".to_owned(),
                    props: BTreeMap::new(),
                    events: BTreeMap::new(),
                    state: ComponentStateSchema::default(),
                    lifecycle: false,
                    effect: None,
                },
                PaintColor(self.0.clone()),
            )
            .map_err(|error| error.to_string())
    }
}

fn check_paint(cx: &mut TestAppContext, hovered: bool, parent_hover: bool) {
    let style = if hovered {
        "style().text_color(rgb(0xff0000)).hover(style().inherit_text_color())"
    } else {
        "style().inherit_text_color()"
    };
    let parent_style = if parent_hover {
        ".hover(style().text_color(rgb(0x654321)))"
    } else {
        ""
    };
    let source = format!(
        r#"fn view(ctx){{column([
            row([test::TextColor(#{{key:"probe"}})]).with_style(style().text_color(rgb(0xff0000)))
                .with_style({style}).with_style(style().width(px(80)).height(px(32)).items_center())
                .accessibility_role("button").accessibility_label("Target")
        ]).with_style(style().width(px(400)).height(px(300)).padding(px(20)).text_color(rgb(0x123456)){parent_style})}}"#
    );
    let color = Rc::new(Cell::new(Hsla::default()));
    let probe = color.clone();
    let (mut visual, view) = Mount::new(source)
        .motion(MotionPreference::None)
        .with(move |view| view.extension(ProbeExtension(probe)))
        .id("inherit-paint")
        .mount(cx);
    visual.simulate_mouse_move(
        gpui::point(gpui::px(600.0), gpui::px(400.0)),
        None,
        Modifiers::none(),
    );
    settle(&mut visual, Step::Refresh, 3);
    let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
    visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    let parent: Hsla = gpui::rgb(0x123456).into();
    let over_parent: Hsla = if parent_hover {
        gpui::rgb(0x654321).into()
    } else {
        parent
    };
    let local: Hsla = gpui::rgb(0xff0000).into();
    assert_eq!(color.get(), if hovered { local } else { parent });
    let target = bounds(&mut visual, &view, "button", "Target");
    visual.simulate_mouse_move(center(target), None, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    assert_eq!(
        color.get(),
        over_parent,
        "hover reset must reach the native text-style stack"
    );
    visual.simulate_mouse_move(
        gpui::point(gpui::px(300.0), gpui::px(200.0)),
        None,
        Modifiers::none(),
    );
    settle(&mut visual, Step::Refresh, 3);
    assert_eq!(color.get(), if hovered { local } else { over_parent });
    visual.simulate_mouse_move(
        gpui::point(gpui::px(600.0), gpui::px(400.0)),
        None,
        Modifiers::none(),
    );
    settle(&mut visual, Step::Refresh, 3);
    assert_eq!(color.get(), if hovered { local } else { parent });
    assert_eq!(
        visual.update(|_, cx| view.committed_revision(cx).unwrap()),
        revision,
        "native color changes must not rerender Rhai"
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let timings = visual
        .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
        .timings;
    assert!(
        timings.is_empty(),
        "native inheritance must not execute Rhai: {timings:?}"
    );
}

#[gpui::test]
fn text_color_reset_reaches_native_paint_in_base_and_hover_states(cx: &mut TestAppContext) {
    for hovered in [false, true] {
        for parent_hover in [false, true] {
            check_paint(cx, hovered, parent_hover);
        }
    }
}
