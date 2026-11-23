//! The authored transform uses the same presented frame for native input and accessibility.
use gpui::{
    Bounds, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px,
};
use gpui_rhai::{MotionPreference, UiValue};

use crate::support::{
    Mount, Step, bounds, by_test_id, click_center, dispatch, nodes, settle, text_of,
};

fn mouse(visual: &mut VisualTestContext, position: Point<Pixels>) {
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    settle(visual, Step::Refresh, 3);
}

fn rotate(p: Point<Pixels>, origin: (f64, f64), degrees: f64) -> Point<Pixels> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let x = f64::from(p.x) - origin.0;
    let y = f64::from(p.y) - origin.1;
    point(
        px((origin.0 + cos * x - sin * y) as f32),
        px((origin.1 + sin * x + cos * y) as f32),
    )
}

fn projected_aabb(bounds: Bounds<Pixels>, origin: (f64, f64), angle: f64) -> Bounds<Pixels> {
    let projected = [
        bounds.origin,
        point(bounds.right(), bounds.top()),
        bounds.bottom_right(),
        point(bounds.left(), bounds.bottom()),
    ]
    .map(|p| rotate(p, origin, angle));
    let x = |cmp: fn(f32, f32) -> f32, init: f32| {
        projected.iter().map(|p| f32::from(p.x)).fold(init, cmp)
    };
    let y = |cmp: fn(f32, f32) -> f32, init: f32| {
        projected.iter().map(|p| f32::from(p.y)).fold(init, cmp)
    };
    Bounds::from_corners(
        point(
            px(x(f32::min, f32::INFINITY)),
            px(y(f32::min, f32::INFINITY)),
        ),
        point(
            px(x(f32::max, f32::NEG_INFINITY)),
            px(y(f32::max, f32::NEG_INFINITY)),
        ),
    )
}

fn bounds_close(a: Bounds<Pixels>, b: Bounds<Pixels>, tolerance: f32) {
    assert!(
        [
            f32::from(a.left() - b.left()),
            f32::from(a.top() - b.top()),
            f32::from(a.right() - b.right()),
            f32::from(a.bottom() - b.bottom()),
        ]
        .into_iter()
        .all(|v| v.abs() <= tolerance),
        "{a:?} != {b:?}"
    );
}

fn input<T>(
    visual: &mut VisualTestContext,
    f: impl FnOnce(&mut gpui::PlatformInputHandler) -> T,
) -> T {
    let mut handler = visual
        .update(|w, _| w.visual_probe_take_input())
        .expect("focused platform handler");
    let value = f(&mut handler);
    visual.update(|w, _| w.visual_probe_restore_input(handler));
    value
}

#[gpui::test]
fn transformed_hit_rejects_empty_corners_of_the_semantic_box(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(
        r#"
      fn state_schema(){#{fields:#{hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
      fn hit(ctx,value){ctx.set_state("hits",ctx.get_state("hits")+1);}
      fn view(ctx){
        box([
          box([]).with_key("rotated").with_style(style().absolute().left(px(140)).top(px(85))
            .width(px(110)).height(px(30)).background(rgb(0xff0000))
            .transform(#{rotate:30})).accessibility_role("button")
            .accessibility_label("Rotated action").on_click(Fn("hit")),
          text(ctx.get_state("hits").to_string()).test_id("hits")
        ]).with_style(style().relative().width(px(380)).height(px(210)))
      }
    "#,
    )
    .motion(MotionPreference::None)
    .mount(cx);
    visual.update(|w, _| {
        w.activate_window();
        w.visual_probe_activate_a11y();
    });
    settle(&mut visual, Step::Refresh, 5);
    let node = nodes(&mut visual, &view)
        .into_iter()
        .find(|n| n.name == "Rotated action")
        .unwrap();
    let geometry = node.geometry.unwrap();
    let rect = bounds(&mut visual, &view, "button", "Rotated action");
    assert!((geometry.layout.width - 110.).abs() < 0.5);
    assert!((geometry.layout.height - 30.).abs() < 0.5);
    assert!(
        rect.height > 75.,
        "semantic bounds did not follow rotation: {rect:?}"
    );

    // Inside the AABB but far from the narrow diagonal button.
    mouse(
        &mut visual,
        point(px((rect.x + 2.) as f32), px((rect.y + 2.) as f32)),
    );
    assert_eq!(text_of(&mut visual, &view, "hits"), "0");
    click_center(&mut visual, rect);
    settle(&mut visual, Step::Refresh, 3);
    assert_eq!(text_of(&mut visual, &view, "hits"), "1");

    visual.update(|window, _| {
        let scale = f64::from(window.scale_factor());
        let expected = [
            geometry.visual.x,
            geometry.visual.y,
            geometry.visual.x + geometry.visual.width,
            geometry.visual.y + geometry.visual.height,
        ]
        .map(|x| x * scale);
        let candidates = window.visual_probe_a11y_bounds();
        assert!(
            candidates
                .iter()
                .any(|(name, actual)| name == "Rotated action"
                    && actual
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| (a - b).abs() < 0.5)),
            "AccessKit bounds do not match the presented node: {candidates:?}, {expected:?}"
        );
    });
}

#[gpui::test]
fn real_editor_click_caret_and_composition_anchor_share_the_visual_matrix(cx: &mut TestAppContext) {
    let (mut visual,view) = Mount::components(r#"
      import "components/input" as input;
      fn state_schema(){#{fields:#{
        text:#{schema:#{type:"string"},"default":#{type:"string",value:"abcdefghi"}},
        rotated:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
      }}}
      fn changed(ctx,value){ctx.set_state("text",value);}
      fn toggle(ctx,value){ctx.set_state("rotated",true);}
      fn view(ctx){box([
        box([input::Input(#{key:"editor", label:"Query", value:ctx.get_state("text"),
          on_change:Fn("changed")})]).with_key("frame").accessibility_role("group")
          .accessibility_label("Editor frame").with_style(style().absolute().left(px(120)).top(px(80))
            .width(px(240)).height(px(48)).transform(#{rotate:if ctx.get_state("rotated"){23}else{0}})),
        text("Turn").test_id("switch").on_click(Fn("toggle"))
      ]).with_style(style().relative().width(px(520)).height(px(240)))}
    "#).motion(MotionPreference::None).mount(cx);
    visual.update(|w, _| w.activate_window());
    settle(&mut visual, Step::Refresh, 3);
    let editor = nodes(&mut visual, &view)
        .into_iter()
        .find(|node| node.name == "Query" && node.value.is_some())
        .expect("native editor");
    let layout = editor.geometry.unwrap().layout;
    let frame = bounds(&mut visual, &view, "group", "Editor frame");
    let origin = (frame.x + frame.width / 2., frame.y + frame.height / 2.);
    mouse(
        &mut visual,
        point(
            px((layout.x + layout.width / 2.) as f32),
            px((layout.y + layout.height / 2.) as f32),
        ),
    );
    let (plain_glyph, plain_caret, focus) = input(&mut visual, |handler| {
        (
            handler.bounds_for_range(3..4).unwrap(),
            handler.bounds_for_range(3..3).unwrap(),
            handler.focus_id(),
        )
    });
    let plain_index = input(&mut visual, |handler| {
        handler.character_index_for_point(plain_glyph.center())
    })
    .unwrap();
    let plain_click = input(&mut visual, |handler| {
        handler.bounds_for_range(plain_index..plain_index)
    })
    .unwrap()
    .center();

    dispatch(&mut visual, &view, by_test_id("switch"), "click").unwrap();
    settle(&mut visual, Step::Refresh, 5);
    let new_geometry = nodes(&mut visual, &view)
        .into_iter()
        .find(|n| n.name == "Query" && n.value.is_some())
        .unwrap()
        .geometry
        .unwrap();
    assert_eq!(new_geometry.layout, layout);
    assert_ne!(new_geometry.visual, new_geometry.layout);
    input(&mut visual, |handler| {
        assert_eq!(handler.focus_id(), focus);
        bounds_close(
            handler.bounds_for_range(3..4).unwrap(),
            projected_aabb(plain_glyph, origin, 23.),
            0.2,
        );
        bounds_close(
            handler.bounds_for_range(3..3).unwrap(),
            projected_aabb(plain_caret, origin, 23.),
            0.2,
        );
        assert_eq!(
            handler.character_index_for_point(rotate(plain_glyph.center(), origin, 23.)),
            Some(plain_index)
        );
    });
    mouse(&mut visual, rotate(plain_click, origin, 23.));
    let selection = input(&mut visual, |handler| {
        handler.selected_text_range(false).unwrap().range
    });
    // At the exact glyph edge both directions may land on adjacent indices
    // due to subpixel rounding. Test offset insertion using that local point.
    assert_eq!(selection, plain_index..plain_index);
    visual.simulate_input("X");
    settle(&mut visual, Step::Refresh, 3);
    let mut expected = "abcdefghi".to_owned();
    expected.insert(plain_index, 'X');
    assert_eq!(
        nodes(&mut visual, &view)
            .into_iter()
            .find(|n| n.name == "Query" && n.value.is_some())
            .unwrap()
            .value,
        Some(UiValue::String(expected))
    );

    input(&mut visual, |handler| {
        handler.replace_and_mark_text_in_range(Some(0..0), "汉字", Some(2..2))
    });
    settle(&mut visual, Step::Refresh, 3);
    input(&mut visual, |handler| {
        assert_eq!(handler.marked_text_range(), Some(0..2));
        // Rotation makes the transformed carets' Y values unequal, even on
        // one composing line. The IME anchor must remain the unprojected line
        // start's caret, mapped to visual space only after selecting the line.
        let start = handler.bounds_for_range(0..0).unwrap();
        let end = handler.bounds_for_range(2..2).unwrap();
        assert!(f32::from(start.top() - end.top()).abs() > 4.);
        bounds_close(handler.ime_candidate_bounds().unwrap(), start, 0.2);
    });
}

#[gpui::test]
fn transformed_text_audit_rejects_real_local_misalignment_only(cx: &mut TestAppContext) {
    for (inset, expected) in [(0, false), (6, true)] {
        let source = format!(
            r#"
          fn view(ctx) {{
            column([
                text("One"),
                box([text("Two")]).with_style(style().padding_start(px({inset})))
            ]).with_style(style().width(px(180)).gap(px(6)).margin(px(80)).transform(#{{rotate:23}}))
          }}
        "#
        );
        let (mut visual, view) = Mount::components(source)
            .motion(MotionPreference::None)
            .mount(cx);
        settle(&mut visual, Step::Refresh, 3);
        let rules = gpui_rhai::AuditRules::from_ids(["text-edge-misaligned"]).unwrap();
        let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        assert_eq!(!findings.is_empty(), expected, "{findings:?}");
    }
}
