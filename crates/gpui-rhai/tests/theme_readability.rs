//! Palette calibration is tested on final resolved tokens, not raw RGB guesses.
use gpui_rhai::{
    Rgba8, RuntimeEngine, ThemeMode, ThemeTokenOverrides, load_theme_with_layers, load_token_base,
};
use gpui_rhai_registry::{BUNDLED_THEME_SOURCES, TOKEN_BASE_SOURCE};

#[allow(clippy::many_single_char_names)] // Standard Oklab matrix notation.
fn lightness(color: Rgba8) -> f64 {
    let [r, g, b, _] = color.as_rgba_hex().to_be_bytes();
    let linear = |c: u8| {
        let x = f64::from(c) / 255.0;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (linear(r), linear(g), linear(b));
    // Oklab L (Björn Ottosson's 2021 sRGB matrices); perceptual tonal steps,
    // not HSL lightness, which changes meaning between different hues.
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s
}

#[test]
#[allow(clippy::too_many_lines)] // One complete per-theme matrix with diagnostic table.
fn all_palettes_have_reading_margin_and_ordered_perceptual_surface_steps() {
    let engine = RuntimeEngine::new();
    let base = load_token_base(engine.engine(), "tokens.rhai", TOKEN_BASE_SOURCE).unwrap();
    println!(
        "| Theme | Primary minimum | Muted minimum | Page L | Raised L | Hover L | Border/well | Primary hover text |"
    );
    for &(file, source) in BUNDLED_THEME_SOURCES {
        let theme = load_theme_with_layers(
            engine.engine(),
            Some(&base),
            file,
            source,
            &ThemeTokenOverrides::default(),
        )
        .unwrap();
        let color = |name: &str| theme.tokens.color(name).unwrap();
        let backgrounds = ["surface", "surface_raised", "surface_hover", "selection"];
        let minimum = |foreground: &str| {
            backgrounds
                .iter()
                .map(|b| color(foreground).contrast_ratio(color(b)))
                .fold(f64::INFINITY, f64::min)
        };
        assert!(
            minimum("text_primary") >= 7.0,
            "{file}: main reading contrast {}",
            minimum("text_primary")
        );
        assert!(
            minimum("text_muted") >= 5.2,
            "{file}: auxiliary text needs margin, not merely a rounded 4.5"
        );
        let page = lightness(color("surface"));
        let raised = lightness(color("surface_raised"));
        let hover = lightness(color("surface_hover"));
        assert!(
            (hover - lightness(color("selection"))).abs() >= 0.04,
            "{file}: selection and hover must remain distinct even without hue"
        );
        if theme.mode == ThemeMode::Dark {
            assert!(
                (0.04..=0.065).contains(&(raised - page)),
                "{file}: raised step {}",
                raised - page
            );
            assert!(
                (0.04..=0.065).contains(&(hover - raised)),
                "{file}: hover step {}",
                hover - raised
            );
        } else {
            assert!(
                (0.015..=0.04).contains(&(raised - page)),
                "{file}: light raised step"
            );
            assert!(
                (0.05..=0.075).contains(&(page - hover)),
                "{file}: light hover step"
            );
        }
        assert!(
            (lightness(color("text_primary")) - lightness(color("text_muted"))).abs() >= 0.075,
            "{file}: primary and secondary are practically the same tone"
        );
        for background in ["surface", "surface_raised"] {
            assert!(
                color("border").contrast_ratio(color(background)) >= 2.0,
                "{file}: invisible decorative border on {background}"
            );
            assert!(
                color("control.border").contrast_ratio(color(background)) >= 3.0,
                "{file}: control boundary on {background}"
            );
        }
        for background in ["surface", "surface_raised", "surface_hover"] {
            assert!(
                color("focus_ring").contrast_ratio(color(background)) >= 3.0,
                "{file}: focus on {background}"
            );
            assert!(
                color("scrollbar.thumb").contrast_ratio(color(background)) >= 3.0,
                "{file}: thumb on {background}"
            );
        }
        assert!(
            color("on_accent").contrast_ratio(color("accent_hover")) >= 4.5,
            "{file}: primary hover loses its foreground"
        );
        for status in ["danger", "warning", "success"] {
            assert!(
                color(&format!("on_{status}"))
                    .contrast_ratio(color(&format!("control.{status}_hover")))
                    >= 4.5,
                "{file}: {status} hover"
            );
        }
        for token in [
            "syntax.comment",
            "syntax.string",
            "syntax.number",
            "syntax.keyword",
            "syntax.function",
            "syntax.type",
            "syntax.variable",
            "syntax.constant",
            "syntax.operator",
            "syntax.punctuation",
            "syntax.tag",
            "syntax.attribute",
        ] {
            for background in ["surface", "surface_raised", "surface_hover"] {
                assert!(
                    color(token).contrast_ratio(color(background)) >= 4.5,
                    "{file}: {token} on {background}"
                );
            }
        }
        println!(
            "| {} {} | {:.2} | {:.2} | {page:.3} | {raised:.3} | {hover:.3} | {:.2} | {:.2} |",
            theme.family,
            theme.name,
            minimum("text_primary"),
            minimum("text_muted"),
            color("border").contrast_ratio(color("surface_raised")),
            color("on_accent").contrast_ratio(color("accent_hover"))
        );
    }
}
