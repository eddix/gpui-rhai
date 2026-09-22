/* gpui-rhai
{
  "id": "motion/reorder_list",
  "export": "ReorderList",
  "version": "0.1.4",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Props: key and ordered keyed labels. State: controlled order. Emits no events.
// Example: ReorderList(#{ key: "queue", items: [#{ key: "a", label: "A" }] })

define_component(#{
    metadata: #{ id: "motion/reorder_list", "export": "ReorderList", version: "0.1.4",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{ props: #{
        key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        items: #{ schema: #{ type: "array", max_items: 256, items: #{
            type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            }
        } }, required: true, sensitive: false },
    }, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "item"] },
    render: Fn("render_ReorderList")
});

fn ReorderList(props) { render_component("motion/reorder_list", props) }
fn render_ReorderList(ctx, props) {
    let rows = [];
    for item in props.items {
        rows.push(text(item.label).with_key(item.key)
            .with_style(ctx.component_style("item", style().padding(px(8))
                .border_bottom(px(1)).border_color(theme_color("border"))))
            .layout_motion(ctx.motion_duration("normal"), ctx.motion_easing("standard")));
    }
    column(rows).with_key(props.key).with_style(ctx.component_style("root", style()))
}
