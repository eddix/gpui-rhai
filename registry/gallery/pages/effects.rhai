// Gallery page: the motion pack. Effects follow the Host motion policy and are never
// used by components.

import "gallery/kit" as kit;
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
import "components/button" as button;
import "layouts/stack" as stack;

fn pages() {
    [#{ id: "effects", title: "Motion pack", group: "Effects" }]
}

fn view(ctx, h, v, id) {
    if id == "effects" { effects_page(h, v) } else { () }
}

fn reorder_demo(h, v) {
    let reversed = kit::val(v, "effects.reversed", false);
    let order = if reversed {
        [#{ key: "three", label: "Three" }, #{ key: "two", label: "Two" }, #{ key: "one", label: "One" }]
    } else {
        [#{ key: "one", label: "One" }, #{ key: "two", label: "Two" }, #{ key: "three", label: "Three" }]
    };
    let reverse = button::Button(#{ key: "fx-reverse", text: "Reverse order",
        on_click: kit::put(h, "effects.reversed", !reversed) });
    kit::line([reverse, reorder_list::ReorderList(#{ key: "fx-order", items: order })])
}

fn cards_demo(h, v) {
    let card = kit::val(v, "effects.card", "beta");
    let next = if card == "alpha" { "beta" } else { "alpha" };
    let move_button = button::Button(#{ key: "fx-card", text: "Move selection",
        on_click: kit::put(h, "effects.card", next) });
    let cards = shared_layout_cards::SharedLayoutCards(#{ key: "fx-cards", selected: card,
        cards: [#{ key: "alpha", title: "Alpha" }, #{ key: "beta", title: "Beta" }] });
    kit::line([move_button, cards])
}

fn tabs_demo(h, v) {
    let tabs = [#{ value: "motion", label: "Motion", content: text("Native frame sampling") },
        #{ value: "effects", label: "Effects", content: text("Public source pack") }];
    animated_tabs::AnimatedTabs(#{ key: "fx-tabs", value: kit::val(v, "effects.tab", "motion"), label: "Runtime",
        tabs: tabs, on_change: kit::store(h, "effects.tab") })
}

fn text_demo() {
    kit::sizes([
        text_reveal::TextReveal(#{ key: "fx-reveal", text: "Motion belongs to the runtime." }),
        number_ticker::NumberTicker(#{ key: "fx-ticker", value: 128 }),
    ])
}

fn marquee_demo() {
    let content = text(" RHAI SOURCE  ·  RUST HOT PATHS  ·  GPUI PAINT ");
    marquee::Marquee(#{ key: "fx-marquee", distance: 420, content: content })
        .with_style(style().width(px(480)).height(theme_length("metrics.control")))
}

fn surfaces_demo() {
    kit::line([
        shimmer::Shimmer(#{ key: "fx-shimmer", width: 200, height: 64 }),
        border_beam::BorderBeam(#{ key: "fx-beam", width: 200, height: 64 }),
        orbit::Orbit(#{ key: "fx-orbit", size: 64 }),
    ])
}

fn effects_page(h, v) {
    let layout = stack::Stack(#{ gap: "section", align: "start", children: [tabs_demo(h, v), reorder_demo(h, v), cards_demo(h, v)] });
    kit::page(h, v, "effects", "Motion pack", "Decorative and feedback motion. With motion set to none, final states appear directly.", [
        kit::sec("Text", text_demo()),
        kit::sec("Marquee", marquee_demo()),
        kit::sec("Surfaces", surfaces_demo()),
        kit::sec("Particles", particles::Particles(#{ key: "fx-particles", width: 480, height: 160, count: 64 })),
        kit::sec("Layout", layout),
    ])
}
