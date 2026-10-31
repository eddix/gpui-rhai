/* gpui-rhai
{
  "id": "components/selectable_card_group",
  "export": "SelectableCardGroup",
  "version": "0.2.2",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/toggle_group"],
  "assets": ["icons/check.svg"],
  "capabilities": {},
  "tokens": ["surface_raised", "surface_hover", "border", "accent", "on_accent", "focus_ring", "text_primary", "text_muted", "disabled", "metrics.inset", "metrics.mark", "space.unit", "space.related", "space.group", "radius.xs", "typography.subtitle", "typography.caption"],
  "environment": ["size", "density"]
}
*/
// Small controlled rich choices with one roving selection tab stop. `content`
// is presentational; independent interactive controls belong to `actions`,
// outside the radio/checkbox. Loading/error content does not disable selection.
// Example: SelectableCardGroup(#{ key: "region", label: "Region", values: ["sg"],
//     items: [#{ value: "sg", label: "Singapore", content: text("3 replicas") }], on_change: Fn("changed") })

import "components/toggle_group" as choices;

define_component(#{
    metadata: #{ id: "components/selectable_card_group", "export": "SelectableCardGroup", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: ["components/toggle_group"], capabilities: #{}, assets: ["icons/check.svg"],
        tokens: ["surface_raised", "surface_hover", "border", "accent", "on_accent", "focus_ring", "text_primary", "text_muted", "disabled", "metrics.inset", "metrics.mark", "space.unit", "space.related", "space.group", "radius.xs", "typography.subtitle", "typography.caption"],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Stable group identity and item-ref prefix." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Accessible group name." },
            values: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "string" } }, required: true, sensitive: false, doc: "Controlled selected values, at most one in single mode; no implicit initial selection." },
            items: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Unique nonempty value." },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Accessible name and visible title." },
                content: #{ schema: #{ type: "node" }, required: true, sensitive: false, doc: "Presentational rich content, exposed as the choice description. Do not place links, inputs or buttons here." },
                actions: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false, doc: "Independent action area outside the choice. Clicking or typing here never changes selection; its controls retain their own disabled policy." },
                disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Blocks selection and skips this item during navigation." },
            } } }, required: true, sensitive: false, doc: "Small choices in visual order. Use List or Select for larger collections." },
            mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] }, required: false, sensitive: false, "default": #{ type: "string", value: "single" }, doc: "Radio choices or independent checkboxes." },
            layout: #{ schema: #{ type: "string", allowed: ["row", "column", "grid"] }, required: false, sensitive: false, "default": #{ type: "string", value: "row" }, doc: "Row, column or equal-column grid. Row cards stretch to equal heights." },
            columns: #{ schema: #{ type: "integer", min: 1, max: 64 }, required: false, sensitive: false, "default": #{ type: "integer", value: 3 }, doc: "Grid column count; Up/Down move by this display-index stride." },
            indicator: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["radio", "check", "none"] } }, required: false, sensitive: false, doc: "Additional visible selection mark; defaults to radio in single mode and check in multiple mode. With none, a Selected caption preserves a non-color cue." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } }, required: false, sensitive: false, doc: "Optional size environment for content and independent actions; card insets follow density." },
            allow_empty: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Allows deselecting the final value; false rejects empty user proposals but never rewrites controlled values." },
            select_on_focus: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: true }, doc: "Single mode arrows select their focused target. False moves focus only until Enter/Space. Multiple mode always moves focus only." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Blocks selection and removes choice surfaces from tab order; independent actions keep their own policy." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false, doc: "Full next values array from selection. Focus movement alone never emits." },
        },
        state: #{ fields: #{ active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } } } },
        events: #{ change: #{ payload: #{ type: "array", max_items: 64, items: #{ type: "string" } }, doc: "Full next selected values; app owns acceptance." } },
        slots: #{}, parts: ["root", "item", "card", "card_selected", "title", "content", "indicator", "actions"],
    },
    render: Fn("render_SelectableCardGroup")
});

fn SelectableCardGroup(props) { render_component("components/selectable_card_group", props) }
fn card_ref_key(props, value) {
    for index in 0..props.items.len {
        if props.items[index].value == value { return `choice-${index}`; }
    }
    "choice-none"
}
fn choice_activate(spec, ctx, payload) {
    ctx.set_state("active", spec.value);
    if spec.next != spec.values { ctx.emit("change", spec.next); }
    event_response().stop()
}
fn choice_move(ctx, spec) {
    ctx.set_state("active", spec.value);
    try { ctx.focus(spec.key); } catch { }
    if spec.select && spec.values != [spec.value] { ctx.emit("change", [spec.value]); }
    event_response().prevent_default().stop()
}
fn choice_focus(value, ctx, focused) { if focused { ctx.set_state("active", value); } }
fn card_navigation(node, ctx, props, value, step, edge, key) {
    let target = choices::choice_target(props.items, value, step, edge);
    node.on_key_value(key, Fn("choice_move"), #{ value: target, key: card_ref_key(props, target),
        select: props.mode == "single" && props.select_on_focus, values: props.values })
}
fn card_indicator(ctx, kind, selected) {
    if kind == "none" {
        return text("Selected").with_style(ctx.component_style("indicator", style()
            .typography("caption").opacity(if selected { 1.0 } else { 0.0 })))
            .accessibility_role("presentation");
    }
    // Radio's semantic circle and 6px dot, or Checkbox's ordinary SVG mark;
    // neither marker is itself an interactive control or accessibility radio.
    let mark = if kind == "radio" {
        row([]).with_style(style().width(px(6)).height(px(6)).radius(px(3))
            .background(theme_color("accent")).opacity(if selected { 1.0 } else { 0.0 }))
    } else {
        image_source(asset("app/icons/check")).with_style(style()
            .width(theme_length("metrics.mark") * 0.75).height(theme_length("metrics.mark") * 0.75)
            .text_color(theme_color("on_accent")).opacity(if selected { 1.0 } else { 0.0 }))
            .accessibility_role("presentation")
    };
    row([mark]).with_style(ctx.component_style("indicator", style()
        .width(theme_length("metrics.mark")).height(theme_length("metrics.mark"))
        .items_center().justify_center().flex_shrink(false).border(px(2))
        .radius(if kind == "radio" { theme_length("metrics.mark") * 0.5 } else { theme_radius("xs") })
        .border_color(if selected { theme_color("accent") } else { theme_color("border") })
        .background(if selected && kind == "check" { theme_color("accent") } else { theme_color("surface_raised") })))
        .accessibility_role("presentation")
}
fn render_choice_card(ctx, props, item, active) {
    let selected = choices::choice_includes(props.values, item.value);
    let unavailable = props.disabled || ("disabled" in item && item.disabled) || !("on_change" in props) || props.on_change == ();
    let reference = card_ref_key(props, item.value);
    let choice_ref = element_ref(reference);
    let description = choice_ref.semantic_id;
    let indicator = if "indicator" in props && props.indicator != () { props.indicator }
        else if props.mode == "multiple" { "check" } else { "radio" };
    let heading = row([
        text(item.label).with_style(ctx.component_style("title", style().typography("subtitle").flex_grow())),
        card_indicator(ctx, indicator, selected),
    ]).with_style(style().items_center().gap(theme_length("space.unit")));
    let surface = style().flex_col().flex_grow().min_width(px(0)).padding(theme_length("metrics.inset"))
        .gap(theme_length("space.group")).text_color(theme_color("text_primary"))
        .hover(style().background(theme_color("surface_hover")))
        .disabled(style().text_color(theme_color("disabled")).opacity(0.65));
    let node = column([heading, item.content.accessibility_id(description)
        .with_style(ctx.component_style("content", style()))])
        .with_key("choice").with_ref(choice_ref).with_style(surface)
        .accessibility_role(if props.mode == "single" { "radio" } else { "checkbox" })
        .accessibility_label(item.label).accessibility_described_by(description)
        .accessibility_checked(selected).disabled(unavailable).tab_stop(!unavailable && active == item.value);
    if !unavailable {
        let spec = #{ value: item.value, values: props.values,
            next: choices::choice_next_values(props.values, item.value, props.mode, props.allow_empty) };
        node = node.on_click(Fn("choice_activate").curry(spec))
            .on_key_value("enter", Fn("choice_activate").curry(spec), ())
            .on_key_value("space", Fn("choice_activate").curry(spec), ())
            .on_focus_within_change(Fn("choice_focus").curry(item.value));
        // Runtime node-key routing already maps physical Left/Right to logical
        // directions in RTL. Do not mirror a second time in this component.
        if props.layout != "column" {
            node = card_navigation(node, ctx, props, item.value, -1, 0, "left");
            node = card_navigation(node, ctx, props, item.value, 1, 0, "right");
        }
        if props.layout != "row" {
            let stride = if props.layout == "grid" { props.columns } else { 1 };
            node = card_navigation(node, ctx, props, item.value, -stride, 0, "up");
            node = card_navigation(node, ctx, props, item.value, stride, 0, "down");
        }
        node = card_navigation(node, ctx, props, item.value, 0, -1, "home");
        node = card_navigation(node, ctx, props, item.value, 0, 1, "end");
    }
    let children = [node];
    if "actions" in item && item.actions != () {
        children.push(item.actions.with_style(ctx.component_style("actions", style()
            .padding_x(theme_length("metrics.inset")).padding_bottom(theme_length("metrics.inset")))));
    }
    // One Card surface frames the presentational radio and its independent
    // actions. The frame is not an input target: action input cannot select it.
    let frame = ctx.component_style("card", style().flex_col().flex_grow().min_width(px(0))
        .background(theme_color("surface_raised"))
        .border(px(2)).border_color(theme_color("border"))
        .focus_within(style().border_color(theme_color("focus_ring"))));
    if selected { frame = ctx.component_style("card_selected", frame.border_color(theme_color("accent"))); }
    // Identity follows value even when items reorder. Refs use the current
    // numeric index only as a component-local lookup (values may be Unicode).
    column(children).with_key(item.value)
        .with_style(ctx.component_style("item", frame))
        .accessibility_role("group").accessibility_label(item.label)
}
fn render_SelectableCardGroup(ctx, props) {
    if props.mode == "single" && props.values.len > 1 { throw "single selectable card group accepts at most one value"; }
    let seen = [];
    for item in props.items {
        if item.value == "" || choices::choice_includes(seen, item.value) { throw "selectable card values must be nonempty and unique"; }
        seen.push(item.value);
    }
    let selected_seen = [];
    for value in props.values {
        if !choices::choice_includes(seen, value) || choices::choice_includes(selected_seen, value) { throw "selected card values must be unique members of items"; }
        selected_seen.push(value);
    }
    let active = choices::choice_cursor(props.items, ctx.get_state("active"), props.values);
    let children = [];
    for item in props.items { children.push(render_choice_card(ctx, props, item, active)); }
    // A grid must start from box, not column: a retained flex direction on a
    // column is intentionally rendered as flex by the runtime.
    let root = if props.layout == "row" { row(children) }
        else if props.layout == "grid" { box(children) } else { column(children) };
    let root_style = style().gap(theme_length("space.related"));
    if props.layout == "grid" { root_style = root_style.grid().grid_cols(props.columns); }
    root = root.with_style(ctx.component_style("root", root_style)).tab_stop(false)
        .accessibility_role(if props.mode == "single" { "radiogroup" } else { "group" })
        .accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
