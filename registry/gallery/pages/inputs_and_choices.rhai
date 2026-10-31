// Native drafting and controlled rich choices. Gallery root owns demo values;
// root-owned handlers call these exported policy functions. Do not retain an
// unbound Fn name from a plain imported page module across a component boundary.
import "gallery/kit" as kit;
import "patterns/composer" as composer;
import "components/selectable_card_group" as cards;
import "components/button" as button;
import "components/tag" as tag;

fn pages() {
    [#{ id: "composer", title: "Composer", group: "Layouts and patterns" },
     #{ id: "selectable_card_group", title: "SelectableCardGroup", group: "Fields" }]
}
fn view(ctx, h, v, id) {
    if id == "composer" { composer_page(h, v) }
    else if id == "selectable_card_group" { cards_page(h, v) }
    else { () }
}
fn demo_put(ctx, key, value) {
    let values = ctx.get_state("values");
    values[key] = value;
    ctx.set_state("values", values);
}
fn composer_send(ctx, value) {
    demo_put(ctx, "composer.sent", value);
    demo_put(ctx, "composer.draft", "");
    demo_put(ctx, "composer.busy", true);
}
fn composer_stop(ctx, payload) { demo_put(ctx, "composer.busy", false); }
fn inspect_region(value, ctx, payload) { demo_put(ctx, "cards.inspected", value); }
fn composer_page(h, v) {
    let draft = kit::val(v, "composer.draft", "");
    let busy = kit::val(v, "composer.busy", false);
    let leading = button::Button(#{ key: "composer-attach", text: "Attach", variant: "ghost",
        on_click: kit::put(h, "composer.attachment", "notes.txt") });
    let field = composer::Composer(#{ key: "composer", label: "Message", value: draft,
        placeholder: "Ask a question · 输入中文", min_rows: 2, max_rows: 6, max_length: 2000,
        busy: busy, can_submit: grapheme_count(draft) > 0, leading: leading,
        on_change: kit::store(h, "composer.draft"), on_submit: h.composer_submit, on_stop: h.composer_stop });
    let form = composer::Composer(#{ key: "composer-comment", label: "Comment", value: kit::val(v, "composer.comment", ""),
        placeholder: "Enter inserts a newline; modifier+Enter accepts", submit_key: "mod_enter",
        on_change: kit::store(h, "composer.comment"), on_submit: kit::store(h, "composer.comment_sent") });
    let feedback = column([
        kit::note(if busy { "Working — Stop ends the demo request. Your next draft stays editable." } else { "Ready to send." }),
        kit::note(`Attachment: ${kit::val(v, "composer.attachment", "none")}`),
        text(`Last accepted: ${kit::val(v, "composer.sent", "none")}`),
    ]).with_style(style().gap(theme_length("space.unit")));
    kit::page(h, v, "composer", "Composer", "Enter sends · Shift+Enter adds a native newline. Sending clears the accepted draft; busy still lets you draft the next message.", [
        kit::sec("Draft and send", column([field, feedback])
            .with_style(style().gap(theme_length("space.group")).width(px(560)))),
        kit::sec("Comment form", form.with_style(style().width(px(560)))),
        kit::sec("Read only", composer::Composer(#{ key: "composer-readonly", label: "Archived draft", value: "Reference only · 可复制",
            read_only: true, on_submit: h.noop, on_change: h.noop }).with_style(style().width(px(560)))),
    ])
}
fn region_items(h, prefix) {
    [
        #{ value: "sg", label: "Singapore", content: column([
            tag::Tag(#{ text: "v2.8.1" }).with_style(style().self_start()), text("12 instances · healthy"), kit::note("Deployed 12 minutes ago"),
        ]).with_style(style().gap(theme_length("space.unit"))),
            actions: button::Button(#{ key: `${prefix}-sg-inspect`, text: "Inspect Singapore", variant: "ghost",
                on_click: h.inspect_region.curry("Singapore") }) },
        #{ value: "va", label: "Virginia", content: column([
            tag::Tag(#{ text: "v2.8.0" }).with_style(style().self_start()), text("8 instances · healthy"), kit::note("Deployed 1 hour ago"),
        ]).with_style(style().gap(theme_length("space.unit"))),
            actions: button::Button(#{ key: `${prefix}-va-inspect`, text: "Inspect Virginia", variant: "ghost",
                on_click: h.inspect_region.curry("Virginia") }) },
        #{ value: "fr", label: "Frankfurt", disabled: true, content: kit::note("Maintenance · selection unavailable") },
    ]
}
fn cards_page(h, v) {
    let selected = kit::val(v, "cards.region", ["sg"]);
    let comparison = kit::val(v, "cards.compare", []);
    let region = cards::SelectableCardGroup(#{ key: "regions", label: "Active region", values: selected,
        items: region_items(h, "regions"), select_on_focus: false,
        on_change: kit::store(h, "cards.region") });
    let compare = cards::SelectableCardGroup(#{ key: "compare", label: "Compare regions", values: comparison,
        items: region_items(h, "compare"), mode: "multiple", layout: "grid", columns: 2,
        allow_empty: true, indicator: "check", on_change: kit::store(h, "cards.compare") });
    let feedback = column([text(`Selected region: ${selected}`),
        text(`Inspected separately: ${kit::val(v, "cards.inspected", "none")}`)])
        .with_style(style().gap(theme_length("space.unit")));
    kit::page(h, v, "selectable_card_group", "SelectableCardGroup", "Rich presentational choices with independent action areas. Arrows move focus; Enter or Space accepts an expensive choice.", [
        kit::sec("Region subject", column([region, feedback]).with_style(style().gap(theme_length("space.group")))),
        kit::sec("Compare several", column([compare, text(`Compared regions: ${comparison}`)])
            .with_style(style().gap(theme_length("space.group")))),
    ])
}
