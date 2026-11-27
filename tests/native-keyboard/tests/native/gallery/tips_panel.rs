use std::time::Duration;

use gpui::{ScrollDelta, ScrollEvent, TestAppContext, point, px, size};

use super::conversation::{click, gallery, node, ready, resize_to_group_width};
use crate::support::{bounds, center, snapshot};

#[gpui::test]
#[cfg(target_os = "macos")]
fn tips_panel_uses_the_inner_grid_width_and_scales_visible_text(cx: &mut TestAppContext) {
    use crate::support::Mount;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui_rhai::MotionPreference;
    use gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("tip_container_size"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = Mount::new(
        r#"import "patterns/tips_panel" as cards;
fn view(ctx) { column([cards::TipsPanel(#{
    key:"card", series:"test", revision:0, title:"Tips", tips:[
        #{id:"one",title:"Search",description:"Search here",action:"Try"},
        #{id:"two",title:"Review",description:"Review here",action:"Try"},
        #{id:"three",title:"Sync",description:"Sync here",action:"Try"}
    ]
})]).with_style(style().width(relative(1.0))).env(#{scale:"200"}) }"#,
    )
    .registry("patterns/tips_panel")
    .motion(MotionPreference::None)
    .with(|b| {
        b.theme_sources(
            BUNDLED_DESIGN_PRESET_SOURCES
                .iter()
                .map(|&(name, source)| (name.to_owned(), source.to_owned())),
        )
    })
    .mount(&mut real);
    // At 200% the Modern tray has two 12px insets; the 1px body
    // border remains 1px on each side and does not follow reading scale.
    for (family, appearance, offset, line) in [
        ("Modern", "Light", 26., 44.),
        ("Modern", "Dark", 26., 44.),
        ("Clean", "Light", 0., 48.),
        ("Clean", "Dark", 0., 48.),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        for (content, cols) in [(380., 1), (381., 2), (560., 2), (561., 3)] {
            resize_to_group_width(&mut v, &view, "Tips", content + offset, 800.);
            let search = bounds(&mut v, &view, "listitem", "Search");
            let review = bounds(&mut v, &view, "listitem", "Review");
            let sync = bounds(&mut v, &view, "listitem", "Sync");
            let title = bounds(&mut v, &view, "heading", "Tips");
            let prose = bounds(&mut v, &view, "text", "Search here");
            assert!(
                (title.height - line).abs() <= 1.,
                "{family}/{appearance}/{content}: header did not scale: {title:?}"
            );
            // At the narrow two- and three-column widths the same
            // description can wrap. Test the 40px line rhythm rather than
            // requiring the complete description to stay on a single line.
            let lines = (prose.height / 40.).round();
            assert!(
                (1. ..=2.).contains(&lines) && (prose.height - lines * 40.).abs() <= 1.,
                "{family}/{appearance}/{content}: description did not scale: {prose:?}"
            );
            if cols == 1 {
                assert_eq!(lines, 1., "{family}/{appearance}/{content}");
            } else if family == "Modern" && content == 381. {
                assert_eq!(lines, 2., "{family}/{appearance}/{content}");
            }
            assert_eq!(
                (search.y - review.y).abs() < 1.,
                cols >= 2,
                "{family}/{appearance}/{content}: {search:?} {review:?} {sync:?}"
            );
            assert_eq!(
                (search.y - sync.y).abs() < 1.,
                cols == 3,
                "{family}/{appearance}/{content}: {search:?} {review:?} {sync:?}"
            );
        }
    }
}

#[gpui::test]
fn tips_panel_waits_for_host_acceptance_and_keeps_completed_state_on_reopen(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "tips_panel", "modern-light");
    v.simulate_resize(size(px(1280.), px(1050.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "0/9 done");
    click(&mut v, &view, "button", "Refuse next tip");
    click(&mut v, &view, "button", "Try commands");
    node(&mut v, &view, "status", "Waiting for the Host…");
    node(&mut v, &view, "status", "0/9 done");
    v.executor().advance_clock(Duration::from_millis(400));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The Host declined this tip action.",
    );
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Waiting for the Host…");
    v.executor().advance_clock(Duration::from_millis(400));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "1/9 done");
    node(&mut v, &view, "status", "Commands enabled");
    click(&mut v, &view, "button", "Close");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("heading", "Tips")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Reopen tips");
    node(&mut v, &view, "status", "1/9 done");
    node(&mut v, &view, "status", "Commands enabled");
}

#[gpui::test]
fn tips_panel_scrolls_only_the_grid_and_preserves_tile_identity_on_resize(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "tips_panel", "clean-dark");
    v.simulate_resize(size(px(1300.), px(1040.)));
    ready(&mut v, &view);
    let name = "Find any command";
    let id = node(&mut v, &view, "listitem", name).id;
    let viewport = bounds(&mut v, &view, "group", "Tip list");
    assert!((viewport.height - 240.).abs() < 2.);
    let header = bounds(&mut v, &view, "status", "0/9 done");
    let footer = bounds(&mut v, &view, "button", "Close");
    let one = bounds(&mut v, &view, "listitem", name);
    let two = bounds(&mut v, &view, "listitem", "Choose a theme");
    assert!((one.y - two.y).abs() < 1.);
    v.simulate_event(ScrollEvent {
        position: center(viewport),
        delta: ScrollDelta::Pixels(point(px(0.), px(-38.))),
        ..Default::default()
    });
    ready(&mut v, &view);
    let moved = bounds(&mut v, &view, "listitem", name);
    assert!(moved.y < one.y - 10., "{moved:?} vs {one:?}");
    assert_eq!(bounds(&mut v, &view, "status", "0/9 done"), header);
    assert_eq!(bounds(&mut v, &view, "button", "Close"), footer);
    assert_eq!(bounds(&mut v, &view, "group", "Tip list"), viewport);
    v.simulate_event(ScrollEvent {
        position: center(viewport),
        delta: ScrollDelta::Pixels(point(px(0.), px(38.))),
        ..Default::default()
    });
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Narrow card");
    assert_eq!(node(&mut v, &view, "listitem", name).id, id);
    let one = bounds(&mut v, &view, "listitem", name);
    let two = bounds(&mut v, &view, "listitem", "Choose a theme");
    assert!(two.y > one.y + one.height);
    assert!((two.x - one.x).abs() < 1.);
}
