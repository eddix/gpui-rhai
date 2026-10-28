/* gpui-rhai
{
  "id": "components/sliding_panel",
  "export": "SlidingPanel",
  "version": "0.3.1",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [],
  "environment": []
}
*/
// Shows `content`, and when `value` changes slides the new content in from the side of the
// one it replaces as it fades in: a short way (`motion_distance("moderate")`) in the `fast`
// duration, mirrored in right-to-left text and settled by the Host's reduced or no-motion
// policy. It remembers the value it showed last, so the components around it can stay
// stateless: Tabs uses it for `panel_motion: "slide"`, Carousel for its items.
//
// Only the new content moves; the one it replaces is gone at once. Nothing has to be painted
// after it leaves, so any content can slide, native fields and virtual lists included.
//
// Example: SlidingPanel(#{ key: "pages", value: current, order: ["one", "two", "three"],
//              content: page(current) })

define_component(#{
    metadata: #{
        id: "components/sliding_panel", "export": "SlidingPanel", version: "0.3.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{}, tokens: [], environment: [],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of the panel, which keeps the value it showed last." },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the shown content; when it changes, the new content slides in." },
            order: #{ schema: #{ type: "array", max_items: 256, items: #{ type: "string" } }, required: true, sensitive: false,
                doc: "Every value in display order. Content after the one it replaces comes in from the end side, content before it from the start side; when either value is not in `order` the content comes in without moving." },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "horizontal" },
                doc: "Axis of the slide: `horizontal` from the start or end side, `vertical` from above or below." },
            wrap: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Whether `order` is a loop: the content comes in from the side of the shorter way round, so the first value after the last comes from the end side." },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The shown content, owned by the caller." },
        },
        state: #{ fields: #{
            shown: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            direction: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
            ready: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        } },
        events: #{}, slots: #{}, parts: ["root", "slide"], effects: ["settle"],
    },
    render: Fn("render_SlidingPanel")
});

fn SlidingPanel(props) { render_component("components/sliding_panel", props) }
fn prop_or(props, name, fallback) { if name in props && props[name] != () { props[name] } else { fallback } }

// 1 when `to` comes after `from` in `order`, -1 when before, 0 when either is missing. In a
// loop the shorter way round decides, and half way round goes forward.
fn slide_direction(order, from, to, wrap) {
    let start = -1;
    let end = -1;
    for index in 0..order.len {
        if order[index] == from { start = index; }
        if order[index] == to { end = index; }
    }
    if start < 0 || end < 0 { return 0; }
    let step = end - start;
    if wrap && step * 2 > order.len { step -= order.len; }
    if wrap && -step * 2 >= order.len { step += order.len; }
    if step > 0 { 1 } else if step < 0 { -1 } else { 0 }
}

// After a commit, remember what is shown and the side it came from: the next render then
// keeps the same motion source, so the slide is neither restarted nor retargeted.
fn settle_start(ctx, deps) {
    ctx.set_state("shown", deps.value);
    ctx.set_state("direction", deps.direction);
    ctx.set_state("ready", true);
}
fn settle_stop(ctx, deps) {}

fn render_SlidingPanel(ctx, props) {
    let vertical = prop_or(props, "orientation", "horizontal") == "vertical";
    // The first content shown comes in without moving.
    let direction = if !ctx.get_state("ready") { 0 }
        else if props.value == ctx.get_state("shown") { ctx.get_state("direction") }
        else { slide_direction(props.order, ctx.get_state("shown"), props.value, prop_or(props, "wrap", false)) };
    effect("settle", #{ value: props.value, direction: direction }, Fn("settle_start"), Fn("settle_stop"));
    // The end side is on the right in left-to-right text and on the left in right-to-left text.
    let side = if !vertical && ctx.text_direction() == "rtl" { -1.0 } else { 1.0 };
    let timing = #{ duration_ms: ctx.motion_duration("fast"), easing: ctx.motion_easing("entrance"),
        intent: "feedback" };
    let slide = column([props.content]).with_key("slide")
        .motion(motion_transition(if vertical { "translate_y" } else { "translate_x" },
            side * direction * ctx.motion_distance("moderate"), 0.0, timing))
        .motion(motion_transition("opacity", if direction == 0 { 1.0 } else { 0.0 }, 1.0, timing))
        // Prefixed, so an empty value is still a replay key.
        .motion_replay_key(`shown:${props.value}`)
        .with_style(ctx.component_style("slide", style().flex_grow().min_width(px(0)).min_height(px(0))));
    // The root stays put and carries the caller's surface, so only the content moves over it.
    column([slide]).with_style(ctx.component_style("root", style().min_width(px(0)).min_height(px(0))))
}
