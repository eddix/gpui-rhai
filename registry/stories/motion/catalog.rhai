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
        .with_style(style().padding_x(theme_spacing("sm")).padding_y(theme_spacing("xs"))
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
        text("MOTION").with_style(theme_typography("heading")),
        text("Native frame sampling, retained identity, and motion-preference policy.")
            .with_style(theme_typography("body_small").text_color(theme_color("text_muted"))),
        row([
            text_reveal::TextReveal(#{ key: "reveal", text: "Motion belongs to the runtime." }),
            number_ticker::NumberTicker(#{ key: "ticker", value: 128 }),
        ]).with_style(style().gap(theme_spacing("lg")).items_center()),
        marquee::Marquee(#{
            key: "marquee", distance: 420,
            content: text(" RHAI FLEXIBILITY  ·  RUST HOT PATHS  ·  GPUI GPU PAINT ")
                .with_style(theme_typography("subtitle")),
        }).with_style(style().width(relative(1.0)).height(px(34))),
        row([
            shimmer::Shimmer(#{ key: "shimmer", width: 220, height: 72 }),
            border_beam::BorderBeam(#{ key: "beam", width: 220, height: 72 }),
            orbit::Orbit(#{ key: "orbit", size: 96 }),
        ]).with_style(style().gap(theme_spacing("lg")).items_center().flex_wrap()),
        particles::Particles(#{ key: "particles", width: 720, height: 240, count: 96 })
            .with_style(style().width(relative(1.0))),
        row([
            text(`Timeline: ${ctx.get_state("timeline_status")}`).with_key("timeline-probe")
                .timeline(demo_timeline).with_style(theme_typography("body")),
            control("play", "Play", Fn("play_demo")),
            control("pause", "Pause", Fn("pause_demo")),
            control("seek", "Seek", Fn("seek_demo")),
            control("restart", "Restart", Fn("restart_demo")),
        ]).with_style(style().gap(theme_spacing("sm")).items_center().flex_wrap()),
        animated_tabs::AnimatedTabs(#{ key: "tabs", value: ctx.get_state("active_tab"), label: "Runtime",
            tabs: [
                #{ value: "motion", label: "Motion", content: text("Native frame sampling") },
                #{ value: "effects", label: "Effects", content: text("Public source pack") },
            ], on_change: Fn("select_tab")
        }),
        row([
            control("reorder", "Reverse order", Fn("toggle_order")),
            reorder_list::ReorderList(#{ key: "order", items: order }),
        ]).with_style(style().gap(theme_spacing("md")).items_center()),
        row([
            control("select-card", "Move selected card", Fn("toggle_card")),
            shared_layout_cards::SharedLayoutCards(#{ key: "cards",
                selected: ctx.get_state("selected_card"), cards: [
                    #{ key: "alpha", title: "Alpha" }, #{ key: "beta", title: "Beta" }
                ] }),
        ]).with_style(style().gap(theme_spacing("md")).items_center()),
    ]).with_style(style().width(relative(1.0)).min_height(relative(1.0))
        .padding(theme_spacing("lg")).gap(theme_spacing("lg")).overflow_y_scroll()
        .background(theme_color("surface")).text_color(theme_color("text_primary")))
        .scrollbars("hidden", "auto")
}
