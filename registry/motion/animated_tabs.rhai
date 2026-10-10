/* gpui-rhai
{
  "id": "motion/animated_tabs",
  "export": "AnimatedTabs",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/tabs"],
  "capabilities": {}
}
*/
// AnimatedTabs is a controlled `Tabs` whose selection indicator slides to the selected tab.
// State: caller-owned.
// Example: AnimatedTabs(#{ key: "docs", value: "one", label: "Docs", tabs: tabs })

import "components/tabs" as tabs_component;

define_component(#{
    metadata: #{ id: "motion/animated_tabs", "export": "AnimatedTabs", version: "0.1.6",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/tabs"], capabilities: #{} },
    schema: #{ props: #{
        key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
            doc: "Instance key and motion identity: the indicator slides within this key's shared-layout group; at most 128 characters." },
        value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
            doc: "Value of the selected tab; the caller stores the `change` payload and passes it back." },
        label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
            doc: "Accessible name of the tab list." },
        orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
            sensitive: false, "default": #{ type: "string", value: "horizontal" },
            doc: "Lays the tabs out in a row or a column; the arrow keys follow it." },
        layout: #{ schema: #{ type: "string", allowed: ["content", "equal"] }, required: false,
            sensitive: false, "default": #{ type: "string", value: "content" },
            doc: "`content` sizes each tab to its label; `equal` gives horizontal tabs equal widths across the full row." },
        tabs: #{ schema: #{ type: "array", max_items: 128, items: #{
            type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Value reported when this tab is chosen; unique among the tabs." },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Tab text; also its accessible name." },
                icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                    doc: "Icon node drawn before the label." },
                label_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                    "default": #{ type: "bool", value: true },
                    doc: "Shows the label; a tab that hides it needs an `icon`." },
                content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                    doc: "Panel shown while this tab is selected." },
                disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                    "default": #{ type: "bool", value: false },
                    doc: "Ignores clicks on the tab and skips it for the arrow keys." },
            }
        } }, required: true, sensitive: false,
            doc: "Tabs in display order." },
        on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
            required: false, sensitive: false,
            doc: "Called with the chosen tab's `value` when a tab is clicked or an arrow key moves; without it the tabs ignore input." },
    }, state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "string" },
           doc: "Emitted when a tab is clicked or an arrow key moves the selection; the payload is the chosen tab's `value`." } },
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
