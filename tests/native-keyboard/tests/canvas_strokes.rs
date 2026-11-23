//! Actual Metal coverage of open caps, joins and a newly trimmed endpoint.

use std::collections::BTreeMap;

use gpui_rhai::offscreen::{ShotPlan, run_plan};
use gpui_rhai::{EmbeddedScriptSource, ModuleId, MotionPreference, ScriptView};

const SOURCE: &str = r#"
    fn view(ctx){
        let commands=[];
        let caps=["butt","round","square"];
        let joins=["miter","round","bevel"];
        for index in 0..3 {
            let offset=index*100;
            commands.push(canvas_line(`cap-${index}`,20+offset,30,70+offset,30,12,rgb(0)).stroke_cap(caps[index]));
            commands.push(canvas_stroke_path(`join-${index}`,[path_move(20+offset,100),path_line(50+offset,100),path_line(50+offset,70)],12,rgb(0)).stroke_join(joins[index]));
        }
        column([
            canvas(canvas_scene(commands).view_box(0,0,320,160))
                .with_style(style().width(px(320)).height(px(160))),
            canvas(canvas_scene([canvas_stroke_path("trim",[path_move(20,20),path_line(80,20)],12,rgb(0))
                .stroke_cap("round")]).view_box(0,0,320,64)).with_key("trimmed")
                .with_style(style().width(px(320)).height(px(64)))
                .motion(motion_transition("path_progress",0.5,0.5,#{duration_ms:120}))
        ]).with_style(style().width(px(352)).height(px(272)).padding(px(16)).gap(px(8)).background(rgb(0xffffff)))
    }
"#;

const CHECKBOX: &str = r#"
    import "components/checkbox" as checkbox;
    fn state_schema(){#{fields:#{
        checked:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        mixed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
    }}}
    fn changed(ctx,value){ctx.set_state("checked",value.checked);ctx.set_state("mixed",value.indeterminate);}
    fn mixed(ctx,value){ctx.set_state("checked",false);ctx.set_state("mixed",true);}
    fn view(ctx){column([
        checkbox::Checkbox(#{key:"choice",label:"Choice",checked:ctx.get_state("checked"),
            indeterminate:ctx.get_state("mixed"),on_change:Fn("changed"),part_styles:#{
                box:style().background(rgb(0)).shadows([]).radius(px(0)).border(px(0)),
                mark:style().text_color(rgb(0xffffff))}}),
        text("Mixed").accessibility_role("button").accessibility_label("Mixed").on_click(Fn("mixed"))
    ]).with_style(style().width(px(300)).height(px(180)).padding(px(20)).gap(px(12)).items_start().background(rgb(0xffffff)))}
"#;

fn mark_ink(directory: &std::path::Path, label: &str) -> usize {
    let frame = image::open(directory.join(format!("{label}.png")))
        .unwrap()
        .to_rgba8();
    let scale = frame.width() / 300;
    frame
        .enumerate_pixels()
        .filter(|(x, y, pixel)| {
            *x >= 21 * scale
                && *x < 35 * scale
                && *y >= 23 * scale
                && *y < 37 * scale
                && pixel.0[..3].iter().all(|v| *v > 230)
        })
        .count()
}

fn checkbox_motion(directory: &std::path::Path) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for policy in [
            MotionPreference::Normal,
            MotionPreference::Reduced,
            MotionPreference::None,
        ] {
            let path = |label: &str| directory.join(format!("{label}.png")).display().to_string();
            let plan = ShotPlan::parse([
                "--size".into(),
                "300x180".into(),
                "--out".into(),
                path("initial"),
                "--click".into(),
                "Choice".into(),
                "--out".into(),
                path("zero"),
                "--wait".into(),
                "20".into(),
                "--out".into(),
                path("delayed"),
                "--wait".into(),
                "40".into(),
                "--out".into(),
                path("partial"),
                "--wait".into(),
                "500".into(),
                "--out".into(),
                path("checked"),
                "--click".into(),
                "Choice".into(),
                "--out".into(),
                path("off"),
                "--click".into(),
                "Mixed".into(),
                "--out".into(),
                path("mixed-zero"),
                "--wait".into(),
                "40".into(),
                "--out".into(),
                path("mixed-partial"),
                "--wait".into(),
                "200".into(),
                "--out".into(),
                path("mixed"),
            ])
            .unwrap();
            run_plan(&plan, || {
                let main = ModuleId::parse("main").unwrap();
                ScriptView::sources(
                    main.clone(),
                    EmbeddedScriptSource::new(BTreeMap::from([
                        (main, CHECKBOX.to_owned()),
                        (
                            ModuleId::parse("components/checkbox").unwrap(),
                            gpui_rhai_registry::CHECKBOX_SOURCE.to_owned(),
                        ),
                    ])),
                    theme,
                )
                .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
                .motion_preference(policy)
                .prepare()
            })
            .unwrap();
            let counts = [
                "initial",
                "zero",
                "delayed",
                "partial",
                "checked",
                "off",
                "mixed-zero",
                "mixed-partial",
                "mixed",
            ]
            .map(|label| mark_ink(directory, label));
            assert_eq!(counts[0], 0, "{name}/{policy:?}: initial {counts:?}");
            assert!(
                counts[4] > 20 && counts[8] > 10,
                "{name}/{policy:?}: actual full glyphs {counts:?}"
            );
            assert_eq!(
                counts[5], 0,
                "{name}/{policy:?}: uncheck clears immediately {counts:?}"
            );
            if policy == MotionPreference::Normal {
                assert_eq!(&counts[1..3], &[0, 0], "{name}: 40ms draw delay {counts:?}");
                assert!(
                    counts[3] > 0 && counts[3] < counts[4],
                    "{name}: partial tick {counts:?}"
                );
                assert_eq!(counts[6], 0, "{name}: mixed starts at zero {counts:?}");
                assert!(
                    counts[7] > 0 && counts[7] < counts[8],
                    "{name}: partial dash {counts:?}"
                );
            } else {
                assert_eq!(
                    counts[1], counts[4],
                    "{name}/{policy:?}: static tick endpoint {counts:?}"
                );
                assert_eq!(
                    counts[6], counts[8],
                    "{name}/{policy:?}: static mixed endpoint {counts:?}"
                );
            }
        }
    }
}

fn assert_ink(frame: &image::RgbaImage, x: u32, y: u32, expected: bool, context: &str) {
    let scale = frame.width() / 352;
    let color = frame.get_pixel(x * scale, y * scale).0;
    assert!(
        if expected {
            color[..3].iter().all(|v| *v < 30)
        } else {
            color[..3].iter().all(|v| *v > 240)
        },
        "{context}: ({x},{y}) expected ink={expected}, got {color:?}"
    );
}

fn main() {
    if !cfg!(target_os = "macos") {
        println!("canvas_strokes: real renderer requires macOS");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("strokes.png");
    let plan = ShotPlan::parse([
        "--size".into(),
        "352x272".into(),
        "--out".into(),
        output.display().to_string(),
    ])
    .unwrap();
    run_plan(&plan, || {
        let main = ModuleId::parse("main").unwrap();
        ScriptView::sources(
            main.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(main, SOURCE.to_owned())])),
            gpui_rhai_registry::MODERN_LIGHT_THEME,
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .motion_preference(MotionPreference::None)
        .prepare()
    })
    .unwrap();
    let frame = image::open(output).unwrap().to_rgba8();
    assert!([352, 704].contains(&frame.width()));
    // Independent interior/exterior points, not a renderer-generated baseline.
    for index in 0..3 {
        let x = 16 + index * 100;
        assert_ink(&frame, x + 50, 46, true, "line body positive control");
        assert_ink(&frame, x + 16, 46, index != 0, "cap extension");
        assert_ink(
            &frame,
            x + 16,
            51,
            index == 2,
            "square, not circular cap corner",
        );
        assert_ink(
            &frame,
            x + 55,
            121,
            index == 0,
            "miter, not round/bevel outer corner",
        );
        assert_ink(
            &frame,
            x + 54,
            119,
            index != 2,
            "round, not bevel outer corner",
        );
    }
    assert_ink(
        &frame,
        16 + 35,
        184 + 20,
        true,
        "trim body positive control",
    );
    assert_ink(
        &frame,
        16 + 55,
        184 + 20,
        true,
        "round cap at halfway endpoint",
    );
    assert_ink(&frame, 16 + 60, 184 + 20, false, "hidden path remainder");
    println!("canvas_strokes: native caps, joins and trim endpoint coverage pass");
    checkbox_motion(directory.path());
    println!("canvas_strokes: Checkbox draw/delay/clear and motion policies pass in every preset");
}
