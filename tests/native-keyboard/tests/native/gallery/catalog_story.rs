//! The component catalog story switches categories and live themes.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, dispatch_script_button, palette_texts};
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ScriptViewConfig, ScriptViewHost, UiValue};
use gpui_rhai_cli::gallery::{GalleryLaunch, prepare as prepare_gallery};

#[gpui::test]
fn component_catalog_story_switches_categories_and_live_themes(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = prepare_gallery(&GalleryLaunch {
        story: "components/catalog".to_owned(),
        ..GalleryLaunch::default()
    })
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("component-gallery-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("component-gallery-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let initial = palette_texts(&mut visual, &view);
    assert!(initial.contains(&"ACTIONS".to_owned()));
    dispatch_script_button(&mut visual, &view, "Code & diff");
    let documents = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert!(
        documents
            .find_by_role_and_name("document", "Rhai source")
            .next()
            .is_some()
    );
    assert!(
        documents
            .find_by_role_and_name("document", "Server A compared with Server B")
            .next()
            .is_some()
    );
    dispatch_script_button(&mut visual, &view, "Forms");
    let forms = palette_texts(&mut visual, &view);
    assert!(forms.contains(&"INPUTS".to_owned()));
    assert!(forms.contains(&"CHOICES".to_owned()));
    assert!(!forms.contains(&"ACTIONS".to_owned()));
    assert!(!forms.contains(&"DATA".to_owned()));
    dispatch_script_button(&mut visual, &view, "Foundations");

    let trigger = visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let trigger = snapshot
            .find_by_role_and_name("combobox", "Gallery theme")
            .next()
            .unwrap();
        assert_eq!(
            trigger.value,
            Some(UiValue::String("Default Dark".to_owned()))
        );
        trigger.geometry.unwrap().visual
    });
    let center = |bounds: gpui_rhai::GeometryBounds| {
        point(
            px((bounds.x + bounds.width / 2.0) as f32),
            px((bounds.y + bounds.height / 2.0) as f32),
        )
    };
    visual.simulate_click(center(trigger), Modifiers::default());
    visual.run_until_parked();
    let search = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text_field", "Gallery theme")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    visual.simulate_click(center(search), Modifiers::default());
    visual.simulate_input("nord");
    visual.run_until_parked();
    visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Dispatch {
                    locator: gpui_rhai::AutomationLocator::RoleName {
                        role: "option".to_owned(),
                        name: "Nord".to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
    visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let trigger = snapshot
            .find_by_role_and_name("combobox", "Gallery theme")
            .next()
            .unwrap();
        assert_eq!(trigger.value, Some(UiValue::String("Nord".to_owned())));
    });
    assert!(
        visual
            .update(|_, cx| view.last_error(cx))
            .unwrap()
            .is_none()
    );
}
