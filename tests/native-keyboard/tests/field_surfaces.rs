//! Real native-editor paint: temporal wrappers retain Input's surface states.

use gpui_rhai::offscreen::{ShotPlan, run_plan};
use gpui_rhai::{EmbeddedScriptSource, FontSource, ModuleId, MotionPreference, ScriptView};
use std::{collections::BTreeMap, path::Path};

const SOURCE: &str = r#"
    import "components/input" as input;
    import "components/time_input" as time_input;
    fn state_schema(){#{fields:#{
        invalid:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
    }}}
    fn noop(ctx,payload){()}
    fn invalid(ctx,payload){ctx.set_state("invalid",true);}
    fn disabled(ctx,payload){ctx.set_state("disabled",true);}
    fn view(ctx){column([
        input::Input(#{key:"plain",label:"Plain",value:"11:59 PM",
            error:ctx.get_state("invalid"),disabled:ctx.get_state("disabled"),style:style().width(px(220))}),
        time_input::TimeInput(#{key:"clock",label:"Clock",value:"11:59 PM",
            error:ctx.get_state("invalid"),disabled:ctx.get_state("disabled"),style:style().width(px(220))}),
        row([
            text("Outside").accessibility_role("button").accessibility_label("Outside").on_click(Fn("noop")),
            text("Invalid").accessibility_role("button").accessibility_label("Invalid").on_click(Fn("invalid")),
            text("Disabled").accessibility_role("button").accessibility_label("Disabled").on_click(Fn("disabled"))
        ]).with_style(style().gap(px(12)))
    ]).with_style(style().width(px(360)).height(px(260)).items_start().padding(px(16)).gap(px(16))
        .typography("body").text_color(theme_color("text_primary")).background(theme_color("surface")))}
"#;

fn image(directory: &Path, state: &str) -> image::RgbaImage {
    image::open(directory.join(format!("{state}.png")))
        .unwrap()
        .to_rgba8()
}

fn assert_fields_equal(
    label: &str,
    left: &image::RgbaImage,
    right: &image::RgbaImage,
    height: u32,
) {
    assert_eq!(left.dimensions(), right.dimensions());
    assert!([360, 720].contains(&left.width()));
    let scale = left.width() / 360;
    let first_y = 16 * scale;
    let second_y = (32 + height) * scale;
    // Include the actual outer focus edge and shadows, not only the fill.
    let padding = 8 * scale;
    let mut differences = 0;
    let mut max_error = 0;
    for y in 0..(height + 16) * scale {
        for x in 8 * scale..244 * scale {
            let a = left.get_pixel(x, first_y - padding + y);
            let b = right.get_pixel(x, second_y - padding + y);
            if a != b {
                differences += 1;
            }
            for channel in 0..4 {
                max_error = max_error.max(a.0[channel].abs_diff(b.0[channel]));
            }
        }
    }
    assert_eq!(
        differences, 0,
        "{label}: {differences} differing pixels, max channel error {max_error}"
    );
}

fn changed_field_edge(a: &image::RgbaImage, b: &image::RgbaImage, height: u32) -> usize {
    let scale = a.width() / 360;
    let mut changed = 0;
    for y in 8 * scale..(24 + height) * scale {
        for x in 8 * scale..244 * scale {
            // Exclude the text/caret interior: this checks real surface paint.
            if x > 20 * scale && x < 232 * scale && y > 20 * scale && y < (12 + height) * scale {
                continue;
            }
            if a.get_pixel(x, y) != b.get_pixel(x, y) {
                changed += 1;
            }
        }
    }
    changed
}

fn preset(name: &str, theme: &str, directory: &Path) {
    let path = |state: &str| directory.join(format!("{state}.png")).display().to_string();
    let plan = ShotPlan::parse([
        "--size".to_owned(),
        "360x260".to_owned(),
        "--click".into(),
        "Outside".into(),
        "--out".into(),
        path("normal"),
        "--click".into(),
        "Plain".into(),
        "--out".into(),
        path("plain-focus"),
        "--click".into(),
        "Clock".into(),
        "--out".into(),
        path("clock-focus"),
        "--click".into(),
        "Invalid".into(),
        "--out".into(),
        path("invalid"),
        "--click".into(),
        "Disabled".into(),
        "--out".into(),
        path("disabled"),
    ])
    .unwrap();
    run_plan(&plan, || {
        let main = ModuleId::parse("main").unwrap();
        let mut modules = BTreeMap::from([(main.clone(), SOURCE.to_owned())]);
        for id in ["components/input", "components/time_input"] {
            let source = gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
                .iter()
                .find(|(candidate, _)| *candidate == id)
                .unwrap()
                .1;
            modules.insert(ModuleId::parse(id).unwrap(), source.to_owned());
        }
        ScriptView::sources(main, EmbeddedScriptSource::new(modules), theme)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .locale_sources([(
                "en.rhai".to_owned(),
                gpui_rhai_registry::EN_LOCALE.to_owned(),
            )])
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
    let height = if name.starts_with("clean_") { 32 } else { 36 };
    let normal = image(directory, "normal");
    let scale = normal.width() / 360;
    let glyphs = normal
        .enumerate_pixels()
        .filter(|(x, y, pixel)| {
            *x >= 24 * scale
                && *x < 120 * scale
                && *y >= (16 + (height - 20) / 2) * scale
                && *y < (36 + (height - 20) / 2) * scale
                && if name.contains("_dark") {
                    pixel.0[0] > 150
                } else {
                    pixel.0[0] < 100
                }
        })
        .count();
    assert!(
        glyphs > 10,
        "{name}: the control must paint real text, got {glyphs} ink pixels"
    );
    for state in ["plain-focus", "invalid", "disabled"] {
        assert!(
            changed_field_edge(&normal, &image(directory, state), height) > 10,
            "{name}/{state}: the real field surface must change, not just the caret"
        );
    }
    for state in ["normal", "invalid", "disabled"] {
        let frame = image(directory, state);
        assert_fields_equal(&format!("{name}/{state}"), &frame, &frame, height);
    }
    assert_fields_equal(
        &format!("{name}/focus"),
        &image(directory, "plain-focus"),
        &image(directory, "clock-focus"),
        height,
    );
}

fn main() {
    if !cfg!(target_os = "macos") {
        println!("field_surfaces: real renderer requires macOS");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        preset(name, theme, directory.path());
    }
    println!("field_surfaces: native Input/TimeInput surfaces match in every preset and state");
}
