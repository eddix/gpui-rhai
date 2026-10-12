//! The component catalog story switches categories and live themes.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::UiValue;
use gpui_rhai_cli::gallery::{GalleryLaunch, prepare as prepare_gallery};

use super::mount_prepared;
use crate::support::{
    Host, bounds, by_role, click_center, dispatch, dispatch_script_button, last_error,
    palette_texts, snapshot,
};

#[gpui::test]
fn component_catalog_story_switches_categories_and_live_themes(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = prepare_gallery(&GalleryLaunch {
        story: "components/catalog".to_owned(),
        ..GalleryLaunch::default()
    })
    .unwrap();
    let (window, view) = mount_prepared(
        cx,
        prepared,
        ("component-gallery-window", "component-gallery-view"),
        false,
        |host, view| Host { host, view },
    );
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    let initial = palette_texts(&mut visual, &view);
    assert!(initial.contains(&"ACTIONS".to_owned()));
    dispatch_script_button(&mut visual, &view, "Code & diff");
    let documents = snapshot(&mut visual, &view);
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

    let trigger = snapshot(&mut visual, &view)
        .find_by_role_and_name("combobox", "Gallery theme")
        .next()
        .unwrap()
        .clone();
    assert_eq!(
        trigger.value,
        Some(UiValue::String("Default Dark".to_owned()))
    );
    click_center(&mut visual, trigger.geometry.unwrap().visual);
    visual.run_until_parked();
    let search = bounds(&mut visual, &view, "text_field", "Gallery theme");
    click_center(&mut visual, search);
    visual.simulate_input("nord");
    visual.run_until_parked();
    dispatch(&mut visual, &view, by_role("option", "Nord"), "click").unwrap();
    visual.run_until_parked();
    let trigger = snapshot(&mut visual, &view)
        .find_by_role_and_name("combobox", "Gallery theme")
        .next()
        .unwrap()
        .clone();
    assert_eq!(trigger.value, Some(UiValue::String("Nord".to_owned())));
    assert!(last_error(&mut visual, &view).is_none());
}
