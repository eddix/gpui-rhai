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

fn state_schema() { #{ fields: #{
    active_tab: #{ schema: #{ type: "string" },
        "default": #{ type: "string", value: "motion" } },
    reversed: #{ schema: #{ type: "bool" },
        "default": #{ type: "bool", value: false } },
    selected_card: #{ schema: #{ type: "string" },
        "default": #{ type: "string", value: "beta" } },
    timeline_status: #{ schema: #{ type: "string" },
        "default": #{ type: "string", value: "idle" } },
} } }

fn select_tab(ctx, value) { ctx.set_state("active_tab", value); }
fn toggle_order(ctx, payload) { ctx.set_state("reversed", !ctx.get_state("reversed")); }
fn toggle_card(ctx, payload) {
    ctx.set_state("selected_card",
        if ctx.get_state("selected_card") == "alpha" { "beta" } else { "alpha" });
}
fn timeline_complete(ctx, payload) { ctx.set_state("timeline_status", "complete"); }
fn play_demo(ctx, payload) {
    ctx.play_motion(ctx.motion_handle("demo"));
    ctx.set_state("timeline_status", "playing");
}
fn pause_demo(ctx, payload) {
    ctx.pause_motion(ctx.motion_handle("demo"));
    ctx.set_state("timeline_status", "paused");
}
fn seek_demo(ctx, payload) {
    ctx.seek_motion(ctx.motion_handle("demo"), 180);
    ctx.set_state("timeline_status", "seek 180ms");
}
fn restart_demo(ctx, payload) {
    ctx.restart_motion(ctx.motion_handle("demo"));
    ctx.set_state("timeline_status", "restarted");
}
fn control(key, label, handler) {
    text(label).with_key(key).test_id(key).accessibility_role("button").on_click(handler)
        .with_style(style().padding_x(px(10)).padding_y(px(6))
            .border(px(1)).border_color(theme_color("border")))
}

fn view(ctx) {
    let order = if ctx.get_state("reversed") {
        [#{ key: "three", label: "Three" }, #{ key: "two", label: "Two" },
         #{ key: "one", label: "One" }]
    } else {
        [#{ key: "one", label: "One" }, #{ key: "two", label: "Two" },
         #{ key: "three", label: "Three" }]
    };
    let demo_timeline = motion_timeline("demo", motion_sequence([
        motion_track(".", motion_transition("opacity", 0.35, 1.0,
            #{ duration_ms: 180, easing: "ease_out", intent: "feedback" })),
        motion_track(".", motion_spring("translate_y", 12.0, 0.0,
            #{ intent: "feedback" })),
    ]), #{ autoplay: false, on_complete: Fn("timeline_complete") });
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
        row([
            text(`Timeline: ${ctx.get_state("timeline_status")}`).with_key("timeline-probe")
                .timeline(demo_timeline).with_style(theme_typography("body")),
            control("play", "Play", Fn("play_demo")),
            control("pause", "Pause", Fn("pause_demo")),
            control("seek", "Seek", Fn("seek_demo")),
            control("restart", "Restart", Fn("restart_demo")),
        ]).with_style(style().gap(px(8)).items_center()),
        animated_tabs::AnimatedTabs(#{ key: "tabs", value: ctx.get_state("active_tab"), label: "Runtime",
            tabs: [
                #{ value: "motion", label: "Motion", content: text("Native frame sampling") },
                #{ value: "effects", label: "Effects", content: text("Public source pack") },
            ], on_change: Fn("select_tab")
        }),
        row([
            control("reorder", "Reverse order", Fn("toggle_order")),
            reorder_list::ReorderList(#{ key: "order", items: order }),
        ]).with_style(style().gap(px(12)).items_center()),
        row([
            control("select-card", "Move selected card", Fn("toggle_card")),
            shared_layout_cards::SharedLayoutCards(#{ key: "cards",
                selected: ctx.get_state("selected_card"), cards: [
            #{ key: "alpha", title: "Alpha" }, #{ key: "beta", title: "Beta" }
            ] }),
        ]).with_style(style().gap(px(12)).items_center()),
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).padding(px(28)).gap(px(24))
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

pub(crate) fn prepared() -> Result<gpui_rhai::PreparedScriptView, gpui_rhai::ScriptViewError> {
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
