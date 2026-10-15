/* gpui-rhai
{
  "id": "patterns/composer",
  "export": "Composer",
  "version": "0.2.2",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/textarea", "components/button"],
  "capabilities": {},
  "tokens": ["surface_raised", "border", "focus_ring", "text_muted", "text.danger", "metrics.inset", "space.related", "radius.lg", "typography.caption"],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled drafting surface. Busy blocks sending, never drafting; the application
// owns sending, stopping and clearing the accepted value.
// Example: Composer(#{ key: "prompt", label: "Prompt", value: draft,
//     on_change: Fn("draft_changed"), on_submit: Fn("send"), on_stop: Fn("stop"), busy: busy })

import "components/textarea" as textarea;
import "components/button" as button;

define_component(#{
    metadata: #{ id: "patterns/composer", "export": "Composer", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/textarea", "components/button"], capabilities: #{},
        tokens: ["surface_raised", "border", "focus_ring", "text_muted", "text.danger", "metrics.inset", "space.related", "radius.lg", "typography.caption"],
        environment: ["size", "density", "corners"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Stable drafting identity; changing it resets native editing state." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Accessible name of the drafting field." },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Controlled draft; sending does not clear it." },
            placeholder: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false, doc: "Hint while the draft is empty." },
            busy: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Shows Stop and blocks send while leaving the draft editable." },
            can_submit: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: true }, doc: "Application validation gate for sending, independent of busy." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Disables the editor and Send/Stop; caller content in leading retains its own policy." },
            read_only: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Preserves selection/copy but blocks drafting and submission." },
            min_rows: #{ schema: #{ type: "integer", min: 1, max: 1000 }, required: false, sensitive: false, "default": #{ type: "integer", value: 2 }, doc: "Minimum native visual rows." },
            max_rows: #{ schema: #{ type: "integer", min: 1, max: 1000 }, required: false, sensitive: false, "default": #{ type: "integer", value: 8 }, doc: "Maximum visual rows before internal scrolling." },
            max_length: #{ schema: #{ type: "optional", value: #{ type: "integer", min: 0, max: 1000000 } }, required: false, sensitive: false, doc: "Optional grapheme limit, also displayed in the counter." },
            show_count: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: true }, doc: "Displays the grapheme count in the action row." },
            submit_key: #{ schema: #{ type: "string", allowed: ["enter", "mod_enter"] }, required: false, sensitive: false, "default": #{ type: "string", value: "enter" }, doc: "Enter sends with Shift+Enter newline, or platform modifier+Enter sends with Enter newline." },
            autofocus: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Focuses the draft once on mount unless disabled." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } }, required: false, sensitive: false, doc: "Optional control size, otherwise inherited by drafting and action controls." },
            leading: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false, doc: "Caller-owned attachment/model/slash-command controls; no attachment or task policy is built in." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false, doc: "Full draft after each native edit." },
            on_submit: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false, doc: "Accepted full draft from keyboard or Send. The application clears value if desired." },
            on_stop: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false, doc: "Stop request while busy; the application owns cancellation." },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{ payload: #{ type: "string" }, doc: "Full edited draft." },
            submit: #{ payload: #{ type: "string" }, doc: "Full accepted draft, without clearing it." },
            stop: #{ payload: #{ type: "null" }, doc: "Application cancellation request." },
        },
        slots: #{ leading: #{ required: false, multiple: false, doc: "Independent caller-owned controls in the action row." } },
        parts: ["root", "editor", "actions", "leading", "counter", "limit", "send", "stop"],
    },
    render: Fn("render_Composer")
});

fn Composer(props) { render_component("patterns/composer", props) }
fn optional(props, key) { if key in props { props[key] } else { () } }
fn draft_changed(ctx, value) { ctx.emit("change", value); }
fn send_draft(enabled, ctx, value) { if enabled { ctx.emit("submit", value); } }
fn send_button(spec, ctx, payload) {
    if spec.enabled {
        ctx.focus("draft");
        ctx.emit("submit", spec.value);
    }
}
fn stop_button(ctx, payload) { ctx.emit("stop", ()); }
fn composer_number(ctx, count) {
    let label = count.to_string();
    try { label = ctx.format_number(count); } catch { }
    label
}

fn render_Composer(ctx, props) {
    let enabled = !props.busy && props.can_submit && !props.disabled && !props.read_only
        && optional(props, "on_submit") != ();
    let field = textarea::Textarea(#{ key: `${props.key}-draft`, label: props.label,
        editor_ref: element_ref("draft"), value: props.value, placeholder: optional(props, "placeholder"),
        size: optional(props, "size"),
        min_rows: props.min_rows, max_rows: props.max_rows, max_length: optional(props, "max_length"),
        disabled: props.disabled, read_only: props.read_only, autofocus: props.autofocus,
        submit_key: props.submit_key, on_change: Fn("draft_changed"), on_submit: Fn("send_draft").curry(enabled),
        part_styles: #{ editor: ctx.component_style("editor", style().border(px(0)).padding(px(0))) } });
    let actions = [];
    if optional(props, "leading") != () {
        actions.push(props.leading.with_style(ctx.component_style("leading", style())));
    }
    actions.push(row([]).with_style(style().flex_grow()));
    if props.show_count {
        let count = grapheme_count(props.value);
        let max = optional(props, "max_length");
        let label = if max == () { composer_number(ctx, count) } else { `${composer_number(ctx, count)} / ${composer_number(ctx, max)}` };
        let counter = ctx.component_style("counter", style().typography("caption").text_color(theme_color("text_muted")));
        if max != () && count >= max { counter = ctx.component_style("limit", counter.text_color(theme_color("text.danger"))); }
        actions.push(text(label).with_style(counter));
    }
    if props.busy {
        actions.push(button::Button(#{ key: "stop", text: "Stop", variant: "secondary",
            disabled: props.disabled || optional(props, "on_stop") == (), on_click: Fn("stop_button") })
            .with_style(ctx.component_style("stop", style())));
    } else {
        actions.push(button::Button(#{ key: "send", text: "Send", variant: "primary", disabled: !enabled,
            on_click: Fn("send_button").curry(#{ enabled: enabled, value: props.value }) })
            .with_style(ctx.component_style("send", style())));
    }
    let root = column([field, row(actions).with_style(ctx.component_style("actions", style()
        .items_center().gap(theme_length("space.related"))))])
        .with_style(ctx.component_style("root", style().padding(theme_length("metrics.inset"))
            .gap(theme_length("space.related")).background(theme_color("surface_raised"))
            .border(px(2)).border_color(theme_color("border")).radius(theme_radius("lg"))
            .focus_within(style().border_color(theme_color("focus_ring")))))
        .accessibility_role("group").accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
