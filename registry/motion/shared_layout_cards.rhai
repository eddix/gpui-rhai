/* gpui-rhai
{
  "id": "motion/shared_layout_cards",
  "export": "SharedLayoutCards",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// SharedLayoutCards moves the selected card from the list to a detail area with a shared-layout animation.
// State: caller-owned selection. Emits no events.
// Example: SharedLayoutCards(#{ key: "cards", selected: "a", cards: cards })

define_component(#{
    metadata: #{ id: "motion/shared_layout_cards", "export": "SharedLayoutCards", version: "0.1.6",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{ props: #{
        key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
            doc: "Motion identity: names the motion group the cards' shared-layout ids belong to." },
        selected: #{ schema: #{ type: "string" }, required: true, sensitive: false,
            doc: "Key of the card shown in the detail area; changing it animates the cards to their new places." },
        cards: #{ schema: #{ type: "array", max_items: 128, items: #{
            type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Card identity and shared-layout id; unique among the cards." },
                title: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Card text." },
            }
        } }, required: true, sensitive: false,
            doc: "Cards in list order; every card except the selected one stays in the list." },
    }, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "list", "detail", "card"] },
    render: Fn("render_SharedLayoutCards")
});

fn SharedLayoutCards(props) { render_component("motion/shared_layout_cards", props) }
fn render_SharedLayoutCards(ctx, props) {
    let list = [];
    let detail = row([]);
    for card in props.cards {
        let node = text(card.title).with_key(card.key).shared_layout(card.key)
            .layout_motion(ctx.motion_duration("normal"), ctx.motion_easing("standard"))
            .with_style(ctx.component_style("card", style().padding(px(12))
                .border(px(1)).border_color(theme_color("border"))
                .background(theme_color("surface_raised"))));
        if card.key == props.selected { detail = node; } else { list.push(node); }
    }
    motion_group(props.key, [
        row(list).with_style(ctx.component_style("list", style().gap(px(8)))),
        row([detail]).with_style(ctx.component_style("detail", style().padding(px(16)))),
    ])
}
