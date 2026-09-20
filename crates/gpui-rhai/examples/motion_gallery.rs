use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

#[allow(dead_code)]
#[path = "../../../registry/src/lib.rs"]
mod registry_snapshot;
use registry_snapshot::{BUNDLED_MOTION_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE, TABS_SOURCE};

const MAIN: &str = r#"
import "motion/text_reveal" as text_reveal;
import "motion/number_ticker" as number_ticker;
import "motion/marquee" as marquee;
import "motion/shimmer" as shimmer;
import "motion/border_beam" as border_beam;
import "motion/orbit" as orbit;
import "motion/particles" as particles;
import "motion/animated_tabs" as animated_tabs;
import "motion/reorder_list" as reorder_list;
import "motion/shared_layout_cards" as shared_layout_cards;

fn view(ctx) {
    column([
        text("MOTION GALLERY").with_style(theme_typography("heading")),
        text("Runtime API 2 · native sampling · reduced-motion aware")
            .with_style(theme_typography("body_small").text_color(theme_color("text_muted"))),
        row([
            text_reveal::TextReveal(#{ key: "reveal", text: "Motion belongs to the runtime." }),
            number_ticker::NumberTicker(#{ key: "ticker", value: 128 }),
        ]).with_style(style().gap(theme_spacing("lg")).items_center()),
        marquee::Marquee(#{
            key: "marquee", distance: 420,
            content: text(" RHАI FLEXIBILITY  ·  RUST HOT PATHS  ·  GPUI GPU PAINT ")
                .with_style(theme_typography("subtitle")),
        }).with_style(style().width(px(560)).height(px(34))),
        row([
            shimmer::Shimmer(#{ key: "shimmer", width: 220, height: 72 }),
            border_beam::BorderBeam(#{ key: "beam", width: 220, height: 72 }),
            orbit::Orbit(#{ key: "orbit", size: 96 }),
        ]).with_style(style().gap(theme_spacing("lg")).items_center()),
        particles::Particles(#{ key: "particles", width: 720, height: 240, count: 96 }),
        animated_tabs::AnimatedTabs(#{ key: "tabs", value: "motion", label: "Runtime",
            tabs: [
                #{ value: "motion", label: "Motion", content: text("Native frame sampling") },
                #{ value: "effects", label: "Effects", content: text("Public source pack") },
            ]
        }),
        reorder_list::ReorderList(#{ key: "order", items: [
            #{ key: "one", label: "One" }, #{ key: "two", label: "Two" },
            #{ key: "three", label: "Three" }
        ] }),
        shared_layout_cards::SharedLayoutCards(#{ key: "cards", selected: "beta", cards: [
            #{ key: "alpha", title: "Alpha" }, #{ key: "beta", title: "Beta" }
        ] }),
    ]).with_style(style().size_full().padding(px(28)).gap(px(24))
        .background(theme_color("surface")).text_color(theme_color("text_primary")))
}
"#;

fn source() -> EmbeddedScriptSource {
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), MAIN.to_owned())]);
    modules.extend(BUNDLED_MOTION_SOURCES_BY_ID.iter().map(|(id, source)| {
        (
            ModuleId::parse(*id).expect("static motion module id"),
            (*source).to_owned(),
        )
    }));
    modules.insert(
        ModuleId::parse("components/tabs").unwrap(),
        TABS_SOURCE.to_owned(),
    );
    EmbeddedScriptSource::new(modules)
}

fn prepared() -> Result<gpui_rhai::PreparedScriptView, gpui_rhai::ScriptViewError> {
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), source(), DEFAULT_THEME)
        .locale_sources([("en.rhai".to_owned(), EN_LOCALE.to_owned())])
        .prepare()
}

fn main() {
    ScriptApplication::new(prepared().expect("motion gallery prepares"))
        .window_size(980.0, 760.0)
        .run()
        .expect("motion gallery runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_motion_pack_prepares_together() {
        prepared().unwrap();
        assert_eq!(BUNDLED_MOTION_SOURCES_BY_ID.len(), 10);
    }
}
