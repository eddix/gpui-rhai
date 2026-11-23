//! Image cues are native presentation, not extra script/semantic owners.
use crate::support::{
    Mount, ReplaceOnce, Step, by_role, dispatch, filled_including_rounded_clips, last_error,
    settle, snapshot,
};
use gpui::TestAppContext;
use gpui_rhai::{AssetData, MotionPreference, ScriptErrorDomain, ThemeTokenOverrides};
use std::{cell::RefCell, rc::Rc};

const SOURCE: &str = r#"
fn state_schema(){#{fields:#{missing:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
fn toggle(ctx,value){ctx.set_state("missing",!ctx.get_state("missing"));}
fn view(ctx){column([
    image_source(asset(if ctx.get_state("missing"){"app/icons/absent"}else{"app/icons/search"}))
        .with_key("logo").image_fallback_text("AL")
        .with_style(style().width(px(32)).height(px(32)))
        .with_part_style("image_fallback",style().background(rgb(0xff00ff)).text_color(rgb(0x000000)))
        .accessibility_role("image").accessibility_label("Ada logo"),
    text("Toggle").accessibility_role("button").on_click(Fn("toggle"))
])}
"#;

#[gpui::test]
fn image_fallback_keeps_semantic_owner_and_deduplicates_until_resource_recovers(
    cx: &mut TestAppContext,
) {
    for formal in [false, true] {
        for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
            let source = if formal {
                format!("import \"components/icon\" as icon;\n{}",SOURCE.replace_once(
            "image_source(asset(if ctx.get_state(\"missing\"){\"app/icons/absent\"}else{\"app/icons/search\"}))",
            "icon::Icon(#{source:asset(if ctx.get_state(\"missing\"){\"app/icons/absent\"}else{\"app/icons/search\"}),label:\"Ada logo\"})"
        ))
            } else {
                SOURCE.to_owned()
            };
            let reports = Rc::new(RefCell::new(Vec::new()));
            let captured = reports.clone();
            let (mut visual, view) = Mount::components(source)
                .theme(theme)
                .motion(MotionPreference::None)
                .id(&format!("image-fallback-{}", name.replace('.', "-")))
                .config(move |config| {
                    config.on_error(move |report, _| captured.borrow_mut().push(report.clone()))
                })
                .mount(cx);
            settle(&mut visual, Step::Tick, 5);
            assert!(last_error(&mut visual, &view).is_none());
            let tree = snapshot(&mut visual, &view);
            let owner = tree
                .find_by_role_and_name("image", "Ada logo")
                .next()
                .unwrap();
            let id = owner.id;
            assert!(owner.children.is_empty(), "cue is not a semantic child");
            let cues = filled_including_rounded_clips(&mut visual, 0xff00ffff);
            assert_eq!(cues.len(), 1, "{name}");
            assert_eq!(cues[0].bounds.width, 32.);
            assert_eq!(cues[0].bounds.height, 32.);
            for _ in 0..3 {
                settle(&mut visual, Step::Tick, 2);
            }
            assert_eq!(reports.borrow().len(), 1);
            assert_eq!(reports.borrow()[0].domain, ScriptErrorDomain::Presentation);
            dispatch(&mut visual, &view, by_role("button", "Toggle"), "click").unwrap();
            settle(&mut visual, Step::Tick, 5);
            assert!(filled_including_rounded_clips(&mut visual, 0xff00ffff).is_empty());
            assert_eq!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("image", "Ada logo")
                    .next()
                    .unwrap()
                    .id,
                id
            );
            dispatch(&mut visual, &view, by_role("button", "Toggle"), "click").unwrap();
            settle(&mut visual, Step::Tick, 5);
            assert_eq!(reports.borrow().len(), 2);
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}

#[gpui::test]
fn command_resource_fallback_uses_one_native_slot_in_rtl_and_reading_scale(
    cx: &mut TestAppContext,
) {
    let source = r#"
        import "components/command" as command;
        fn init(ctx){ctx.set_locale("ar");}
        fn view(ctx){command::Command(#{key:"commands",label:"Commands",query:"",active_value:"a",items:[
            #{value:"a",label:"🎨 Équipe",logo:"media/missing"},
            #{value:"b",label:"王小明",icon:image_source(asset("media/other")).image_fallback_text("王")
                .with_part_style("image_fallback",style().background(rgb(0x00ff00)))},
            #{value:"c",label:"Empty slot"}
        ],part_styles:#{image_fallback:style().background(rgb(0xff00ff))}}).with_style(style().width(px(560)))}
    "#;
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let overrides = ThemeTokenOverrides {
            environment_defaults: [("scale".into(), "200".into())].into(),
            ..Default::default()
        };
        let (mut visual, view) = Mount::components(source)
            .theme(theme)
            .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
            .with(move |view| view.theme_token_overrides(overrides))
            .motion(MotionPreference::None)
            .mount(cx);
        settle(&mut visual, Step::Tick, 8);
        assert!(last_error(&mut visual, &view).is_none(), "{name}");
        let cue = filled_including_rounded_clips(&mut visual, 0xff00ffff);
        assert_eq!(cue.len(), 1, "{name}");
        let custom = filled_including_rounded_clips(&mut visual, 0x00ff00ff);
        assert_eq!(custom.len(), 1, "{name}");
        let expected = if name.starts_with("clean") { 24. } else { 28. };
        assert_eq!(cue[0].bounds.width, expected);
        assert_eq!(cue[0].bounds.height, expected);
        let tree = snapshot(&mut visual, &view);
        assert_eq!(tree.find_by_role_and_name("option", "🎨 Équipe").count(), 1);
        assert_eq!(tree.find_by_role_and_name("option", "王小明").count(), 1);
        assert!(
            !tree.nodes().any(|node| node.name == "É"),
            "decorative cue must not become an extra accessible name"
        );
    }
}

#[gpui::test]
fn avatar_uses_initials_when_native_raster_decode_fails(cx: &mut TestAppContext) {
    let source = r#"
        import "components/avatar" as avatar;
        fn state_schema(){#{fields:#{picture:#{schema:#{type:"optional",value:#{type:"handle",kind:"image"}},"default":#{type:"null"}}}}}
        fn init(ctx){ctx.set_state("picture",ctx.load_image(asset("app/broken")));}
        fn view(ctx){avatar::Avatar(#{name:"Ada Lovelace",initials:"AL",handle:ctx.get_state("picture"),
            part_styles:#{initials:style().background(rgb(0xff00ff))}})}
    "#;
    let (mut visual, view) = Mount::new(source)
        .registry("components/avatar")
        .with(|view| {
            view.asset_sources([(
                "broken".into(),
                AssetData {
                    mime_type: "image/png".into(),
                    bytes: b"not a PNG".to_vec(),
                },
            )])
        })
        .mount(cx);
    for _ in 0..40 {
        settle(&mut visual, Step::Tick, 1);
        if !filled_including_rounded_clips(&mut visual, 0xff00ffff).is_empty() {
            break;
        }
    }
    assert!(last_error(&mut visual, &view).is_none());
    assert_eq!(
        filled_including_rounded_clips(&mut visual, 0xff00ffff).len(),
        1
    );
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("image", "Ada Lovelace")
            .count(),
        1
    );
}
