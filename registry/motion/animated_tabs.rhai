/* gpui-rhai
{
  "id": "motion/animated_tabs",
  "export": "AnimatedTabs",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": ["components/tabs"],
  "capabilities": {}
}
*/
// Props: key plus the controlled Tabs contract. State: caller-owned. Events: change(string).
// Example: AnimatedTabs(#{ key: "docs", value: "one", label: "Docs", tabs: tabs })

import "components/tabs" as tabs_component;

define_component(#{
    metadata: #{ id: "motion/animated_tabs", "export": "AnimatedTabs", version: "0.1.6",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 },
        dependencies: ["components/tabs"], capabilities: #{} },
    schema: #{ props: #{
        key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
            sensitive: false, "default": #{ type: "string", value: "horizontal" } },
        layout: #{ schema: #{ type: "string", allowed: ["content", "equal"] }, required: false,
            sensitive: false, "default": #{ type: "string", value: "content" } },
        tabs: #{ schema: #{ type: "array", max_items: 128, items: #{
            type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
                label_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                    "default": #{ type: "bool", value: true } },
                content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                    "default": #{ type: "bool", value: false } },
            }
        } }, required: true, sensitive: false },
        on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
            required: false, sensitive: false },
    }, state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "string" } } },
       slots: #{}, parts: ["root"] },
    render: Fn("render_AnimatedTabs")
});

fn AnimatedTabs(props) { render_component("motion/animated_tabs", props) }
fn render_AnimatedTabs(ctx, props) {
    let inner = #{ value: props.value, label: props.label, tabs: props.tabs,
        orientation: if "orientation" in props { props.orientation } else { "horizontal" },
        layout: if "layout" in props { props.layout } else { "content" },
        motion_key: props.key };
    if "on_change" in props { inner.on_change = props.on_change; }
    tabs_component::Tabs(inner).with_key(props.key)
        .with_style(ctx.component_style("root", style()))
}
