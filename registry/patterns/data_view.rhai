/* gpui-rhai
{
  "id": "patterns/data_view",
  "export": "DataView",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["layouts/region", "layouts/toolbar", "patterns/inline_state"],
  "capabilities": {},
  "tokens": ["text_primary", "text_muted", "space.related"],
  "environment": []
}
*/
// DataView is a browsable collection in one region: toolbar, table or list, and a footer
// with status, selection, count and data time.
//
// Props: key and label (required), body (required), title, toolbar (Toolbar slot map:
//        context, filters, actions, primary), state (InlineState props without key),
//        footer (status, selection, count, updated).
// Events/state: none. Parts: root.
// Example: DataView(#{ key: "hosts", label: "Hosts", title: "Hosts", body: table_node,
//              toolbar: #{ filters: [filter_input], primary: add_button },
//              footer: #{ count: "128 hosts", updated: "Updated 12:04" } })
//
// Loading, empty and error states replace the body in place; stale and refreshing keep
// the data and add one quiet line above it.

import "layouts/region" as region;
import "layouts/toolbar" as toolbar;
import "patterns/inline_state" as inline_state;

define_component(#{
    metadata: #{
        id: "patterns/data_view", "export": "DataView", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["layouts/region", "layouts/toolbar", "patterns/inline_state"], capabilities: #{},
        tokens: ["text_primary", "text_muted", "space.related"],
        environment: [],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            title: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            toolbar: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                context: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                filters: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                actions: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                primary: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            } } }, required: false, sensitive: false },
            body: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            state: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                state: #{ schema: #{ type: "string", allowed: ["loading", "empty", "error", "stale", "refreshing"] },
                    required: true, sensitive: false },
                title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                detail: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                actions: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            } } }, required: false, sensitive: false },
            footer: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                status: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
                selection: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                count: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                updated: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            } } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ body: #{ required: true, multiple: false } },
        parts: ["root"],
    },
    render: Fn("render_DataView")
});

fn DataView(props) { render_component("patterns/data_view", props) }
fn has(map, name) { name in map && map[name] != () }

fn render_DataView(ctx, props) {
    // Rows carry their own inset, so the body bleeds to the region sides and row text
    // starts on the title's edge.
    let config = #{ label: props.label, body: props.body, bleed: true };
    if has(props, "title") { config.title = props.title; }
    if has(props, "toolbar") {
        let slots = props.toolbar;
        slots.label = `${props.label} toolbar`;
        config.toolbar = toolbar::Toolbar(slots);
    }
    if has(props, "state") {
        let spec = props.state;
        spec.key = `${props.key}-state`;
        let feedback = inline_state::InlineState(spec);
        config.body = if spec.state == "stale" || spec.state == "refreshing" {
            column([feedback, props.body]).with_style(style().flex_col().flex_grow().min_height(px(0))
                .gap(theme_length("space.related")))
        } else { feedback };
    }
    if has(props, "footer") {
        let footer = props.footer;
        let start = [];
        if has(footer, "status") { start.push(footer.status); }
        if has(footer, "selection") {
            start.push(text(footer.selection).with_style(style().text_color(theme_color("text_primary"))));
        }
        let end = [];
        for name in ["count", "updated"] {
            if has(footer, name) { end.push(text(footer[name]).with_style(style().font_feature("tnum", 1))); }
        }
        config.footer = [
            row(start).with_style(style().flex_row().items_center().flex_grow().gap(theme_length("space.related"))),
            row(end).with_style(style().flex_row().items_center().gap(theme_length("space.related"))),
        ];
    }
    region::Region(config).with_key(props.key).with_style(ctx.component_style("root", style()))
}
