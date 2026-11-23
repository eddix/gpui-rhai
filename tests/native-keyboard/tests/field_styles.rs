//! Compare native editor glyphs with ordinary text under identical public styles.

use std::collections::BTreeMap;

use gpui_rhai::offscreen::{ShotPlan, run_plan};
use gpui_rhai::{
    EmbeddedScriptSource, FontSource, ModuleId, MotionPreference, Rgba8, ScriptView,
    ThemeTokenOverrides,
};

const SOURCE: &str = r#"
    import "components/input" as input;
    import "components/textarea" as textarea;
    fn common(size,weight,alpha){style().width(px(260)).height(px(56)).min_height(px(0))
        .padding(px(0)).border(px(0)).radius(px(0)).shadows([]).background(rgb(0xffffff))
        .text_color(rgb(0)).typography("__ROLE__").font_family("Inter")
        .font_size(px(size)).line_height(px(__LINE__)).font_weight(weight).opacity(alpha)
        .hover(style().opacity(0.55).shadows([]))
        .focus(style().opacity(1.0).shadows([]))
        .disabled(style().background(rgb(0xffffff)).text_color(rgb(0)).shadows([]).opacity(alpha))}
    fn field(key,size,weight,disabled,alpha){
        if __AREA__ {textarea::Textarea(#{key:key,label:key,value:"MMMM",disabled:disabled,rows:1,min_rows:1,max_rows:1,
            style:style().width(px(260)).height(px(56)).padding(px(0)).gap(px(0)),
            part_styles:#{editor:common(size,weight,alpha),caret:style().background(rgba(0))}})}
        else {input::Input(#{key:key,label:key,value:"MMMM",disabled:disabled,style:common(size,weight,alpha)})}
    }
    fn example(key,size,weight,disabled,alpha){row([
        field(key,size,weight,disabled,alpha),
        row([text("MMMM").with_style(style().text_color(rgb(0)).typography("__ROLE__")
            .font_family("Inter").font_size(px(size)).line_height(px(__LINE__)).font_weight(weight))])
            .with_style(style().width(px(260)).height(px(56)).__ALIGN__().background(rgb(0xffffff)).opacity(alpha))
    ]).with_style(style().gap(px(16)))}
    fn noop(ctx,value){()}
    fn view(ctx){let base_size=__BASE_SIZE__;column([
        example("baseline",base_size,400,false,1.0),
        example("size",20.0,400,false,1.0),
        example("weight",base_size,600,false,1.0),
        example("disabled",base_size,400,true,0.55),
        example("disabled-override",base_size,400,true,1.0),
        example("medium",base_size,500,false,1.0),
        text("Outside").accessibility_role("button").accessibility_label("Outside").on_click(Fn("noop"))
    ]).with_style(style().width(px(600)).height(px(508)).padding(px(16)).gap(px(12)).items_start().background(rgb(0xffffff)))}
"#;

#[derive(Debug, PartialEq)]
struct Ink {
    width: u32,
    height: u32,
    pixels: Vec<image::Rgba<u8>>,
}

fn ink(image: &image::RgbaImage, column: u32, index: u32) -> Ink {
    let scale = image.width() / 600;
    let origin = ((16 + 276 * column) * scale, (16 + 68 * index) * scale);
    let mut points = Vec::new();
    for y in 0..56 * scale {
        for x in 0..260 * scale {
            let p = image.get_pixel(origin.0 + x, origin.1 + y);
            if p.0[..3].iter().any(|v| *v < 250) {
                points.push((x, y));
            }
        }
    }
    assert!(
        points.len() > 10,
        "the glyph positive control must actually paint"
    );
    let x0 = points.iter().map(|p| p.0).min().unwrap();
    let y0 = points.iter().map(|p| p.1).min().unwrap();
    let x1 = points.iter().map(|p| p.0).max().unwrap() + 1;
    let y1 = points.iter().map(|p| p.1).max().unwrap() + 1;
    Ink {
        width: x1 - x0,
        height: y1 - y0,
        pixels: (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| *image.get_pixel(origin.0 + x, origin.1 + y)))
            .collect(),
    }
}

fn main() {
    if !cfg!(target_os = "macos") {
        println!("field_styles: real renderer requires macOS");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for area in [false, true] {
            let output = directory.path().join(format!("{name}.png"));
            let hover_output = directory.path().join(format!("{name}-hover.png"));
            let focus_output = directory.path().join(format!("{name}-focus.png"));
            let restored_output = directory.path().join(format!("{name}-restored.png"));
            let plan = ShotPlan::parse([
                "--size".to_owned(),
                "600x508".to_owned(),
                "--click".into(),
                "Outside".into(),
                "--out".into(),
                output.display().to_string(),
                "--hover".into(),
                "baseline".into(),
                "--out".into(),
                hover_output.display().to_string(),
                "--click".into(),
                "baseline".into(),
                "--hover".into(),
                "Outside".into(),
                "--out".into(),
                focus_output.display().to_string(),
                "--click".into(),
                "Outside".into(),
                "--out".into(),
                restored_output.display().to_string(),
            ])
            .unwrap();
            run_plan(&plan, || {
                let main = ModuleId::parse("main").unwrap();
                let widget_source = |id| {
                    gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
                        .iter()
                        .find(|(candidate, _)| *candidate == id)
                        .unwrap()
                        .1
                };
                ScriptView::sources(
                    main.clone(),
                    EmbeddedScriptSource::new(BTreeMap::from([
                        (
                            main,
                            SOURCE
                                .replace("__AREA__", if area { "true" } else { "false" })
                                .replace("__ROLE__", if area { "textarea" } else { "control" })
                                .replace(
                                    "__LINE__",
                                    if area && !name.starts_with("clean_") {
                                        "21"
                                    } else {
                                        "20"
                                    },
                                )
                                .replace(
                                    "__ALIGN__",
                                    if area { "items_start" } else { "items_center" },
                                )
                                .replace(
                                    "__BASE_SIZE__",
                                    if area || name.starts_with("clean_") {
                                        "13.0"
                                    } else {
                                        "13.5"
                                    },
                                ),
                        ),
                        (
                            ModuleId::parse("components/input").unwrap(),
                            widget_source("components/input").to_owned(),
                        ),
                        (
                            ModuleId::parse("components/textarea").unwrap(),
                            widget_source("components/textarea").to_owned(),
                        ),
                    ])),
                    theme,
                )
                .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
                // The caret is not part of a glyph comparison. Native caret
                // geometry and its visible color have separate behavior tests.
                .theme_token_overrides(ThemeTokenOverrides {
                    colors: [("accent".to_owned(), Rgba8::from_rgba_hex(0))].into(),
                    ..Default::default()
                })
                .font_sources(
                    gpui_rhai_registry::BUNDLED_FONT_SOURCES
                        .iter()
                        .map(|&(name, bytes)| FontSource::new(name, bytes.to_vec()).unwrap())
                        .collect::<Vec<_>>(),
                )
                .motion_preference(MotionPreference::None)
                .prepare()
            })
            .unwrap();
            let frame = image::open(output).unwrap().to_rgba8();
            assert!([600, 1200].contains(&frame.width()));
            assert!(
                ink(&frame, 1, 0) != ink(&frame, 1, 2),
                "{name}: the plain-text weight reference must visibly distinguish 400 and 600"
            );
            assert!(
                ink(&frame, 1, 0) != ink(&frame, 1, 5) && ink(&frame, 1, 5) != ink(&frame, 1, 2),
                "{name}: the native medium face must differ from both regular and semibold"
            );
            for (index, label) in [
                "baseline",
                "size",
                "weight",
                "disabled",
                "disabled-override",
                "medium",
            ]
            .into_iter()
            .enumerate()
            {
                let index = u32::try_from(index).unwrap();
                let actual = ink(&frame, 0, index);
                let expected = ink(&frame, 1, index);
                if index == 0 {
                    assert!(
                        actual == expected,
                        "{name}/area={area}: matching typography positive control; native {}x{}, reference {}x{}",
                        actual.width,
                        actual.height,
                        expected.width,
                        expected.height
                    );
                } else if actual != expected {
                    failures.push(format!(
                        "{name}/area={area}/{label}: native {}x{} vs reference {}x{}",
                        actual.width, actual.height, expected.width, expected.height
                    ));
                }
            }
            for (state, output, reference) in [
                ("hover", hover_output, 3),
                ("focus", focus_output, 0),
                ("restored", restored_output, 0),
            ] {
                let image = image::open(output).unwrap().to_rgba8();
                if ink(&image, 0, 0) != ink(&frame, 1, reference) {
                    failures.push(format!("{name}/area={area}/{state}: native state opacity did not match the reference"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("field_styles: native typography and disabled opacity follow public styles");
}
