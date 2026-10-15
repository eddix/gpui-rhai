//! Exercise the actual source-owned Gallery pages, not a separate specimen UI.
use gpui::TestAppContext;
use gpui_rhai::{AuditRule, AuditRules, UiValue};
use gpui_rhai_registry::{
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID, EN_LOCALE, GALLERY_SOURCES_BY_ID,
};

use crate::support::{Mount, bounds, click_center, last_error, nodes};

const PAGE_SOURCE: &str =
    include_str!("../../../../../registry/gallery/pages/inputs_and_choices.rhai");

fn page_main(page: &str, density: &str) -> String {
    format!(
        r#"
        import "gallery/pages/inputs_and_choices" as pages;
        fn state_schema() {{ #{{fields:#{{values:#{{schema:#{{type:"map",values:#{{type:"ui_value"}}}},"default":#{{type:"map",value:#{{}}}}}}}}}} }}
        fn noop(ctx,payload) {{}}
        fn init(ctx) {{ ctx.register_action("gallery.toggle_source",Fn("noop")); }}
        fn set_value(key,ctx,value) {{ let values=ctx.get_state("values"); values[key]=value; ctx.set_state("values",values); }}
        fn put_value(key,value,ctx,payload) {{ set_value(key,ctx,value); }}
        fn gallery_composer_submit(ctx,value) {{ pages::composer_send(ctx,value); }}
        fn gallery_composer_stop(ctx,payload) {{ pages::composer_stop(ctx,payload); }}
        fn gallery_inspect_region(value,ctx,payload) {{ pages::inspect_region(value,ctx,payload); }}
        fn view(ctx) {{ pages::view(ctx,#{{set:Fn("set_value"),put:Fn("put_value"),noop:Fn("noop"),
            composer_submit:Fn("gallery_composer_submit"),composer_stop:Fn("gallery_composer_stop"),inspect_region:Fn("gallery_inspect_region")}},ctx.get_state("values"),"{page}")
            .with_style(style().width(px(800)).height(px(700))).env(#{{density:"{density}"}}) }}
    "#
    )
}

#[gpui::test]
fn inputs_and_choices_gallery_pages_mount_interact_and_audit_cleanly(cx: &mut TestAppContext) {
    let kit = GALLERY_SOURCES_BY_ID
        .iter()
        .find(|(id, _)| *id == "gallery/kit")
        .unwrap()
        .1;
    let rules = AuditRules::from_ids(
        AuditRule::ALL
            .into_iter()
            .filter(|rule| *rule != AuditRule::UnresolvedFont)
            .map(AuditRule::id),
    )
    .unwrap();
    for density in ["comfortable", "compact"] {
        for page in ["composer", "selectable_card_group"] {
            let (mut visual, view) = Mount::components(page_main(page, density))
                .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
                .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
                .module("gallery/kit", kit)
                .module("gallery/pages/inputs_and_choices", PAGE_SOURCE)
                .locale("en.rhai", EN_LOCALE)
                .mount(cx);
            assert_eq!(last_error(&mut visual, &view), None);
            let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
            assert!(findings.is_empty(), "{page}/{density}: {findings:?}");
            if page == "composer" {
                let field = bounds(&mut visual, &view, "text_field", "Message");
                click_center(&mut visual, field);
                visual.simulate_input("Send from the Gallery");
                visual.simulate_keystrokes("enter");
                visual.run_until_parked();
                assert_eq!(last_error(&mut visual, &view), None);
                let submitted = nodes(&mut visual, &view);
                assert!(
                    submitted
                        .iter()
                        .any(|node| node.name == "Last accepted: Send from the Gallery"),
                    "{}",
                    submitted
                        .iter()
                        .map(|node| format!("{}:{:?}:{:?}", node.role, node.name, node.value))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                visual.simulate_input("next draft");
                let stop = bounds(&mut visual, &view, "button", "Stop");
                click_center(&mut visual, stop);
                assert!(
                    nodes(&mut visual, &view)
                        .iter()
                        .any(|node| node.role == "text_field"
                            && node.name == "Message"
                            && node.value == Some(UiValue::String("next draft".into())))
                );
            } else {
                let inspect = bounds(&mut visual, &view, "button", "Inspect Virginia");
                click_center(&mut visual, inspect);
                let snapshot = nodes(&mut visual, &view);
                assert!(
                    snapshot
                        .iter()
                        .any(|node| node.name == "Selected region: [\"sg\"]")
                );
                assert!(
                    snapshot
                        .iter()
                        .any(|node| node.name == "Inspected separately: Virginia")
                );
                let virginia = bounds(&mut visual, &view, "radio", "Virginia");
                click_center(&mut visual, virginia);
                assert!(
                    nodes(&mut visual, &view)
                        .iter()
                        .any(|node| node.name == "Selected region: [\"va\"]")
                );
            }
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}
