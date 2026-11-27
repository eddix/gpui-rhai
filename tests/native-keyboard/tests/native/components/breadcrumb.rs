//! Actual width and keyboard transactions, not a computed string count.
use gpui::{Modifiers, TestAppContext, px, size};
use gpui_rhai::MotionPreference;

use crate::support::{
    Mount, Step, bounds, center, click_center, last_error, nodes, settle, text_of,
};

const SOURCE: &str = r#"
  import "components/breadcrumb" as breadcrumb;
  fn state_schema(){#{fields:#{chosen:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
  fn chose(ctx,value){ctx.set_state("chosen",value);}
  fn view(ctx){column([
      breadcrumb::Breadcrumb(#{key:"trail",label:"Directory trail",
          items:[#{value:"a",label:"Workspace"}, #{value:"b",label:"Services",disabled:true},
              #{value:"c",label:"Production deployment"}, #{value:"d",label:"Release candidates"},
              #{value:"e",label:"Checkout rollout"}],on_navigate:Fn("chose")}),
      text(ctx.get_state("chosen")).test_id("chosen")
  ]).with_style(style().width(relative(1.0)).min_width(px(0)))}
"#;

#[gpui::test]
fn breadcrumb_folds_its_measured_trail_and_routes_overflow_by_key(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SOURCE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    let shows = |visual: &mut gpui::VisualTestContext, name: &str, role: &str| {
        nodes(visual, &view)
            .into_iter()
            .any(|n| n.name == name && n.role == role)
    };
    visual.simulate_resize(size(px(750.), px(300.)));
    settle(&mut visual, Step::TickRefresh, 8);
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(shows(&mut visual, "Workspace", "button"));
    assert!(
        shows(&mut visual, "Earlier pages", "button"),
        "count folds one ancestor"
    );
    assert!(
        shows(&mut visual, "Production deployment", "button"),
        "wide trail restores suffix"
    );
    assert!(shows(&mut visual, "Release candidates", "button"));
    assert!(
        !shows(&mut visual, "Services", "button"),
        "measurement has no action"
    );
    assert!(
        !shows(&mut visual, "Checkout rollout", "button"),
        "current page is not an action"
    );

    visual.simulate_resize(size(px(177.), px(300.)));
    settle(&mut visual, Step::TickRefresh, 8);
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        !shows(&mut visual, "Production deployment", "button"),
        "measured width folds more than max_items"
    );
    assert!(!shows(&mut visual, "Release candidates", "button"));
    let more = bounds(&mut visual, &view, "button", "Earlier pages");
    click_center(&mut visual, more);
    settle(&mut visual, Step::TickRefresh, 6);
    assert!(shows(&mut visual, "Services", "menuitem"));
    assert!(shows(&mut visual, "Production deployment", "menuitem"));
    assert!(shows(&mut visual, "Release candidates", "menuitem"));
    visual.simulate_keystrokes("down enter");
    settle(&mut visual, Step::TickRefresh, 6);
    assert_eq!(
        text_of(&mut visual, &view, "chosen"),
        "d",
        "first enabled key is c; Down selects d"
    );
    assert!(
        !shows(&mut visual, "Release candidates", "menuitem"),
        "controlled request closes overflow"
    );
    assert!(shows(&mut visual, "Checkout rollout", "text"));
    visual.simulate_mouse_move(center(more), None, Modifiers::none());
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::TickRefresh, 6);
    assert!(
        shows(&mut visual, "Release candidates", "menuitem"),
        "Menu restores focus to live trigger"
    );
    visual.simulate_keystrokes("escape");
    settle(&mut visual, Step::TickRefresh, 6);
    assert!(!shows(&mut visual, "Release candidates", "menuitem"));

    visual.simulate_resize(size(px(750.), px(300.)));
    settle(&mut visual, Step::TickRefresh, 8);
    assert!(shows(&mut visual, "Release candidates", "button"));
    let visible = bounds(&mut visual, &view, "button", "Production deployment");
    click_center(&mut visual, visible);
    settle(&mut visual, Step::TickRefresh, 5);
    assert_eq!(text_of(&mut visual, &view, "chosen"), "c");
    assert_eq!(last_error(&mut visual, &view), None);
}
