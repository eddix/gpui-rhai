/* gpui-rhai
{
  "id": "components/kbd",
  "export": "Kbd",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "typography.key_legend", "typography.key_legend_cap", "typography.key_legend_menu", "metrics.kbd_h", "metrics.kbd_glyph", "space.unit"
  ],
  "environment": ["size", "density"]
}
*/
// Kbd presents one framed key or combination, or a bare inline key legend.
//
// Example: Kbd(#{ keys: ["mod", "shift", "p"], size: "md" })
//
// Named keys have platform-specific visual and spoken names. A combination uses
// one keycap, keeps its input order in RTL, and does not register a key binding.

define_component(#{
    metadata: #{ id: "components/kbd", "export": "Kbd", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "typography.key_legend", "typography.key_legend_cap", "typography.key_legend_menu", "metrics.kbd_h", "metrics.kbd_glyph", "space.unit"
        ],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "A literal legend used when keys is empty; not parsed as key names or bindings. Empty content produces no chip." },
            keys: #{ schema: #{ type: "one_of", variants: [#{ type: "string" },
                #{ type: "array", max_items: 8, items: #{ type: "string" } }] },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "One named key or a combination in one chip, in input order. Whitespace-only keys are ignored. Supports mod, cmd, ctrl, alt, shift, enter, esc, tab, space, backspace, delete, arrows, pageup, pagedown, home and end; other names stay literal." },
            label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Accessible name; otherwise uses spelled-out key names, or the literal text." },
            size: #{ schema: #{ type: "string", allowed: ["sm", "md"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "sm" },
                doc: "Keycap size, independent of surrounding control size: Modern 18/22px, Clean 16/20px, before reading scale." },
            platform: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["mac", "other"] } },
                required: false, sensitive: false, doc: "Explicit key naming convention, or current platform when omitted. Does not change binding semantics." },
            appearance: #{ schema: #{ type: "string", allowed: ["keycap", "inline", "menu"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "keycap" },
                doc: "keycap frames the whole combination; inline shows a bare legend; menu uses a 20px contextual keycap, inheriting its surrounding menu/command ink. None install bindings." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "keys", "keys_other", "key", "glyph", "inline", "menu", "menu_key", "menu_keys"],
    },
    render: Fn("render_Kbd")
});

fn Kbd(props) { render_component("components/kbd", props) }
fn render_Kbd(ctx, props) {
    let requested = if type_of(props.keys) == "string" { [props.keys] } else { props.keys };
    let keys = [];
    for raw in requested {
        let key = if "platform" in props && props.platform != () { key_legend(raw, props.platform) }
            else { key_legend(raw) };
        if key.label != "" { keys.push(key); }
    }
    if keys.is_empty() && props.text == "" { return fragment([]); }
    let spoken = if keys.is_empty() { props.text }
        else { keys.reduce(|sum,key| if sum == () { key.spoken } else { `${sum} ${key.spoken}` }) };
    let name = if "label" in props && props.label != () { props.label } else { spoken };
    let inline = props.appearance == "inline";
    let menu = props.appearance == "menu";
    if inline {
        let separator = if !keys.is_empty() && keys[0].platform == "other" { " " } else { "" };
        let run = if keys.is_empty() { props.text }
            else { keys.reduce(|sum,key| if sum == () { key.label } else { `${sum}${separator}${key.label}` }) };
        return text(run).with_style(ctx.component_style(["root","key","inline"], style()
            .typography("key_legend")))
            .accessibility_role("text").accessibility_label(name).env(#{size:props.size});
    }
    let contents = [];
    if keys.is_empty() {
        contents.push(text(props.text).with_style(ctx.component_style(if menu {["key","menu_key"]}else{["key"]},style()))
            .accessibility_role("presentation"));
    } else {
        let glyphs = [];
        let index = 1;
        for key in keys {
            let parts = if menu {["key","menu_key"]}else if key.glyph { ["key","glyph"] } else { ["key"] };
            glyphs.push(text(key.label).with_key(`key-${index}`)
                .with_style(ctx.component_style(parts,style().grid_cell(index,1)))
                .accessibility_role("presentation"));
            index += 1;
        }
        let parts = if keys[0].platform == "other" { ["keys","keys_other"] } else { ["keys"] };
        if menu {parts.push("menu_keys");}
        // Explicit columns preserve left-to-right key order without changing
        // locale or directional navigation outside the combination.
        contents.push(box(glyphs).with_key("keys").with_style(ctx.component_style(parts,
            style().grid_cols_max_content(keys.len).items_center())));
    }
    row(contents).with_style(ctx.component_style(if menu {["root","menu"]}else{["root"]},style().flex_row().items_center()
        .justify_center().flex_shrink(false))).env(#{size:props.size})
        .accessibility_role("text").accessibility_label(name)
}
