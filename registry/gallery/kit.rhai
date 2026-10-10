// Shared building blocks for Gallery pages. Pages are plain functions; the Gallery
// root owns all state and passes `h` (handlers created in the root) and `v` (values).

import "layouts/region" as region;
import "layouts/stack" as stack;
import "layouts/inline" as inline;
import "patterns/section" as section;
import "components/button" as button;

// A stored demo value, or the fallback before the first change.
fn val(v, key, fallback) { if key in v { v[key] } else { fallback } }

// A handler that stores the event payload under `key`.
fn store(h, key) { h.set.curry(key) }

// A handler that stores a fixed `value` under `key` when pressed.
fn put(h, key, value) { h.put.curry(key).curry(value) }

// A Checkbox change handler that stores the new checked state under `key`.
fn checked(h, key) { h.checked.curry(key) }

// Page header: the title and one line of description, as a single heading block.
fn heading(title, description) {
    column([
        text(title).with_style(style().typography("title").text_color(theme_color("text_primary")))
            .accessibility_role("heading").accessibility_level(1),
        text(description).with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ]).with_style(style().flex_col().gap(theme_spacing("xxs")).min_width(px(0)))
}

// A spec page: header with the Source toggle, then sections a `section` apart, scrolling.
fn page(h, v, id, title, description, sections) {
    let source_open = val(v, "gallery.source_open", false);
    let toggle = button::Button(#{ key: `${id}-source`, text: if source_open { "Hide source" } else { "Source" },
        variant: "ghost", size: "sm", action: "gallery.toggle_source" });
    let body = stack::Stack(#{ gap: "section", children: sections });
    let scroller = column([body]).with_key(`${id}-scroll`)
        .with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll());
    region::Region(#{ label: title, title: heading(title, description), actions: [toggle], body: scroller })
        .with_key(`page-${id}`)
}

// A label-voice section of a spec page.
fn sec(title, content) {
    section::Section(#{ title: title, voice: "label", content: content })
}

// Specimens in one row, `related` apart, wrapping when narrow.
fn line(items) { inline::Inline(#{ children: items, wrap: true }) }
// The same, top-aligned, for specimens of different heights (a field with a counter).
fn line_top(items) { inline::Inline(#{ children: items, wrap: true, align: "start" }) }

// Specimen rows that compare sizes or variants side by side do so on purpose; they
// opt out of the rules that would otherwise flag exactly that comparison.
fn sizes(items) { line(items).audit_allow(["row-height-mismatch", "mixed-type-in-row"]) }
fn variants(items) { line(items).audit_allow(["multiple-solid-actions"]) }

// Specimens stacked `related` apart.
fn pile(items) { stack::Stack(#{ children: items, align: "start" }) }

// The same specimen in both densities, side by side. Callers build it twice (with
// distinct keys) because one node cannot appear twice in a view.
fn densities(comfortable, compact) {
    row([density_column("comfortable", comfortable), density_column("compact", compact)])
        .with_style(style().flex_row().flex_wrap().items_start().gap(theme_length("space.section")))
        .audit_allow(["row-height-mismatch", "mixed-type-in-row"])
}
fn density_column(density, content) {
    column([
        text(density.to_upper()).with_style(style().typography("label").text_color(theme_color("text_muted")))
            .accessibility_role("heading").accessibility_level(3),
        content,
    ]).with_style(style().flex_col().gap(theme_spacing("sm")).min_width(px(0))).env(#{ density: density })
}

// A muted caption under a specimen.
fn note(content) { text(content).with_style(style().typography("caption").text_color(theme_color("text_muted"))) }
