#![cfg(target_os = "macos")]

use std::{cell::RefCell, rc::Rc, sync::Arc};

use gpui::{
    App, Bounds, Context, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    ParentElement as _, Pixels, Render, Styled as _, StyledText, TestAppContext, TextAlign,
    TextExclusion, TextExclusionSide, Window, div, point, px,
};
use gpui_rhai::text_geometry::{CaretAffinity, TextGeometry};

use crate::support::{Mount, bounds};

type Projection = Rc<RefCell<Option<(Bounds<Pixels>, TextGeometry)>>>;
const WRAPPED: &str = "a wider than average line with CJK 光標 and a family 👩‍👧 marker";

#[gpui::test]
fn authored_underline_offset_stays_relative_to_each_wrapped_baseline(cx: &mut TestAppContext) {
    use crate::support::{Step, last_error, logical_paint_rect, settle, walk_live_paint};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui::{TransformationMatrix, size};

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("underline_offset"),
        Arc::new(apple::AppleTextSystem::new()),
    );
    let source = r#"
        fn view(ctx) {
            let copy="A reference line with a long title that wraps before the end.";
            let measure=style().width(relative(1.0)).min_width(px(0))
                .font_size(px(15)).line_height(px(27)).whitespace_normal();
            column([
                text([span(copy).underline(1.5,rgb(0xb02f70),1.0)])
                    .with_key("near").accessibility_role("text").accessibility_label("Near")
                    .with_style(measure),
                text([span(copy).underline(1.5,rgb(0x0c8c50),7.0)])
                    .with_key("far").accessibility_role("text").accessibility_label("Far")
                    .with_style(measure)
            ]).with_style(style().width(relative(1.0)).gap(px(15)))
        }
    "#;
    let (mut v, view) = Mount::new(source).mount(&mut real);
    for width in [165., 255.] {
        v.simulate_resize(size(px(width), px(410.)));
        settle(&mut v, Step::Frame, 4);
        assert_eq!(last_error(&mut v, &view), None);
        let [near, far] = ["Near", "Far"].map(|name| bounds(&mut v, &view, "text", name));
        assert!((near.height - far.height).abs() <= 0.01);
        assert!(near.height >= 54., "{near:?}");
        let [mut near_lines, mut far_lines] = v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut lines = [Vec::new(), Vec::new()];
                walk_live_paint(
                    scene,
                    TransformationMatrix::default(),
                    &mut |scene, pose| {
                        for line in &scene.underlines {
                            let ink = line.color.to_rgb();
                            let kind = usize::from(ink.g > ink.r);
                            let local = logical_paint_rect(line.bounds, pose, scale);
                            lines[kind].push(local);
                        }
                    },
                );
                lines
            })
        });
        near_lines.sort_by(|a, b| a.y.total_cmp(&b.y));
        far_lines.sort_by(|a, b| a.y.total_cmp(&b.y));
        assert!(
            near_lines.len() >= 2 && near_lines.len() == far_lines.len(),
            "{width}: {near_lines:?} {far_lines:?}"
        );
        for (a, b) in near_lines.iter().zip(far_lines) {
            let displacement = (f64::from(b.y) - far.y) - (f64::from(a.y) - near.y);
            assert!(
                (displacement - 6.).abs() <= 0.1
                    && (a.width - b.width).abs() <= 0.5
                    && (a.x - b.x).abs() <= 0.5,
                "{width}: {a:?} {b:?}, displacement {displacement}"
            );
        }
    }
}

/// Capture the geometry of the same native element that paints the glyphs.
/// There is no second shape, transparent measuring control or DOM mock.
struct Probe {
    text: StyledText,
    captured: Projection,
}

impl IntoElement for Probe {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Probe {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.text.request_layout(id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text.prepaint(id, inspector, bounds, state, window, cx);
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text
            .paint(id, inspector, bounds, state, prepaint, window, cx);
        let geometry =
            TextGeometry::from_layout(self.text.layout(), window.text_style().text_align)
                .expect("real painted layout is projectable");
        self.captured
            .borrow_mut()
            .replace((self.text.layout().bounds(), geometry));
    }
}

struct Document {
    source: &'static str,
    width: Pixels,
    right: bool,
    exclusion: Option<TextExclusion>,
    captured: Projection,
}

impl Render for Document {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut text = StyledText::new(self.source);
        if let Some(exclusion) = self.exclusion {
            text = text.with_prefix_exclusion(exclusion);
        }
        let node = div()
            .w(self.width)
            .ml(px(23.0))
            .mt(px(17.0))
            .text_size(px(17.0))
            .line_height(px(24.0))
            .child(Probe {
                text,
                captured: self.captured.clone(),
            });
        if self.right {
            node.text_align(TextAlign::Right)
        } else {
            node.text_align(TextAlign::Left)
        }
    }
}

fn actual_fonts(cx: &TestAppContext) -> TestAppContext {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("text_geometry"),
        Arc::new(apple::AppleTextSystem::new()),
    )
}

#[gpui::test]
fn text_geometry_projects_actual_wrapping_and_right_alignment(cx: &mut TestAppContext) {
    let mut cx = actual_fonts(cx);
    let captured = Projection::default();
    let copy = captured.clone();
    let window = cx.add_window(move |_, _| Document {
        source: WRAPPED,
        width: px(155.0),
        right: true,
        exclusion: None,
        captured: copy,
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let (bounds, geometry) = captured.borrow_mut().take().expect("one real frame");
    assert!(geometry.rows().len() >= 2, "text must actually wrap");
    for row in geometry.rows() {
        assert!((row.right() - bounds.right()).abs() <= px(0.01));
    }
    assert!(bounds.left() >= px(20.0));
    let end = geometry.rows().len();
    let first = geometry.caret(0, CaretAffinity::Downstream).unwrap();
    assert_eq!(geometry.hit(first.position), 0);
    window
        .update(&mut cx, |doc, _, cx| {
            doc.width = px(338.0);
            cx.notify();
        })
        .unwrap();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let (wide, reflow) = captured.borrow_mut().take().expect("one resized frame");
    assert!(wide.size.width > bounds.size.width);
    assert!(reflow.rows().len() < end);
    assert_eq!(
        reflow.hit(point(wide.right(), reflow.rows().last().unwrap().top())),
        WRAPPED.len()
    );
}

#[gpui::test]
fn text_geometry_uses_real_mixed_direction_glyph_order(cx: &mut TestAppContext) {
    let mut cx = actual_fonts(cx);
    let captured = Projection::default();
    let copy = captured.clone();
    let _window = cx.add_window(move |_, _| Document {
        source: "aאבz",
        width: px(250.0),
        right: false,
        exclusion: None,
        captured: copy,
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let (_, geometry) = captured
        .borrow_mut()
        .take()
        .expect("one mixed-direction frame");
    let before = geometry.caret(1, CaretAffinity::Upstream).unwrap();
    let after = geometry.caret(1, CaretAffinity::Downstream).unwrap();
    assert!(after.position.x > before.position.x);
    let pieces = geometry.range(0..2);
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    assert!(pieces[1].left() > pieces[0].right());
}

#[gpui::test]
#[allow(clippy::cast_possible_truncation)]
fn selectable_text_drag_uses_actual_right_aligned_glyphs(cx: &mut TestAppContext) {
    let mut cx = actual_fonts(cx);
    let (mut visual, view) = Mount::components(
        r#"
            fn view(ctx){
                column([text("字形 abc").with_key("witness").selectable(true)
                    .accessibility_role("document").accessibility_label("Right aligned")
                    .with_style(style().width(px(340)).font_size(px(17)).line_height(px(26)).text_end())])
                    .with_style(style().padding(px(20)))
            }
        "#,
    )
    .motion(gpui_rhai::MotionPreference::None)
    .mount(&mut cx);
    visual.update(|window, _| window.activate_window());
    visual.run_until_parked();
    let text = bounds(&mut visual, &view, "document", "Right aligned");
    let right = px((text.x + text.width) as f32);
    let y = px((text.y + text.height * 0.5) as f32);
    let start = point(right - px(108.0), y);
    let end = point(right - px(0.5), y);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".to_owned()));
    visual.simulate_mouse_down(start, gpui::MouseButton::Left, gpui::Modifiers::none());
    visual.run_until_parked();
    visual.simulate_mouse_move(end, gpui::MouseButton::Left, gpui::Modifiers::none());
    visual.run_until_parked();
    visual.simulate_mouse_up(end, gpui::MouseButton::Left, gpui::Modifiers::none());
    visual.run_until_parked();
    visual.simulate_keystrokes("cmd-c");
    visual.run_until_parked();
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("字形 abc".to_owned())
    );
}

const PROSE: &str = "One bright canvas\nThe next logical line keeps the remaining cutout rows, then fills the full width below the figure. 中文光標 and العربية and 👩‍👧 still belong to the same text and the anchors follow their real glyphs.";

#[gpui::test]
fn wrap_exclusion_uses_real_font_rows_and_preserves_graphemes(cx: &mut TestAppContext) {
    let mut cx = actual_fonts(cx);
    let captured = Projection::default();
    let copy = captured.clone();
    let window = cx.add_window(move |_, _| Document {
        source: PROSE,
        width: px(274.0),
        right: false,
        exclusion: Some(TextExclusion {
            side: TextExclusionSide::Left,
            width: px(112.),
            height: px(48.),
        }),
        captured: copy,
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let (left_bounds, left) = captured.borrow_mut().take().expect("left cutout frame");
    assert!(left.rows().len() > 4);
    for (index, row) in left.rows().enumerate() {
        let front = left_bounds.left() + if index < 2 { px(112.) } else { px(0.) };
        assert!((row.left() - front).abs() < px(0.01), "{index}");
        assert!(row.right() <= left_bounds.right() + px(2.));
    }
    let old_start = left.caret(0, CaretAffinity::Downstream).unwrap();
    assert!((old_start.position.x - left_bounds.left() - px(112.)).abs() < px(0.01));
    assert_eq!(left.hit(old_start.position), 0);
    assert_eq!(
        left.byte_at_grapheme(left.grapheme_count()),
        Some(PROSE.len())
    );
    let selected = left.range(0..left.grapheme_count());
    assert!(
        selected
            .iter()
            .any(|rect| rect.left() == left_bounds.left())
    );
    assert!(
        selected
            .iter()
            .any(|rect| rect.left() >= left_bounds.left() + px(112.))
    );

    window
        .update(&mut cx, |document, _, cx| {
            document.right = true;
            document.width = px(330.);
            document.exclusion = Some(TextExclusion {
                side: TextExclusionSide::Right,
                width: px(91.),
                height: px(49.),
            });
            cx.notify();
        })
        .unwrap();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let (right_bounds, right) = captured.borrow_mut().take().expect("right cutout frame");
    assert!(right.rows().len() > 3);
    for (index, row) in right.rows().enumerate() {
        let back = right_bounds.right() - if index < 3 { px(91.) } else { px(0.) };
        assert!((row.right() - back).abs() < px(0.01), "{index}");
    }
    assert_eq!(left.grapheme_count(), right.grapheme_count());
}
