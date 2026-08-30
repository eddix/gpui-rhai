# Textarea specification

Textarea is a controlled multiline text field backed by a keyed native editor
entity. It shares editing, selection, clipboard, and IME infrastructure with
Input while using a separate wrapped multiline layout and scrolling element.
A `multiline` flag on the existing single-line element is not sufficient.

## Public contract

`registry/components/textarea.rhai` exports `Textarea(props)` with:

- required `key: string` and controlled `value: string`;
- optional `placeholder`;
- `disabled`, `read_only`, and `error`;
- `size: "xs" | "sm" | "md" | "lg"`;
- auto-grow `min_rows: integer >= 1`, default 3;
- auto-grow `max_rows: integer >= min_rows`, default 8;
- optional fixed `rows: integer >= 1`;
- optional `max_length: integer >= 0`;
- `show_count: bool` and `autofocus: bool`;
- optional `on_change`, `on_focus`, and `on_blur`.

Row counts are capped at 1,000 and `max_length` at 1,000,000 graphemes, matching
the runtime's bounded UI/string resource policy.

When `rows` is present, fixed-height mode ignores min/max for layout and scrolls
internally. Otherwise the measured wrapped visual-line count is clamped between
min_rows and max_rows. Width changes, large paste, deletion, and controlled
value replacement must recompute height without oscillation.

Style parts include at least `root`, `editor`, `placeholder`, `selection`,
`caret`, `scroll`, `counter`, and `limit`.

## Text and limit semantics

Textarea preserves explicit newlines and soft-wraps long lines. Rows count
visual wrapped lines, not newline characters. The first contract has no
`wrap: false` or horizontal code-editor mode.

User-visible characters are Unicode extended grapheme clusters. Cursor boundary
logic, character count, and `max_length` use the same unit. Normal insertion is
limited at a grapheme boundary; oversized paste inserts the prefix that fits.
IME marked text may temporarily exceed the limit so composition is not
corrupted, then clamps safely when committed. Replacing a selection credits the
removed grapheme count before applying the limit.

With `show_count`, the component displays the localized current count, and
`current / max` when max_length exists. Reaching the limit activates the
styleable `limit` state; it does not emit a second hidden value.

## Native editing and focus

The shared editing core owns UTF-8/UTF-16 conversions, grapheme boundaries,
selection direction, marked ranges, clipboard operations, controlled-value
reconciliation, max-length insertion policy, and a bounded 100-revision native
undo/redo history. Consecutive single-grapheme typing coalesces for 750 ms; one
IME composition is one revision; selection replacements/paste remain atomic;
an authoritative external controlled replacement clears history. Cmd-Z and
Shift-Cmd-Z use the same core in Input and Textarea. Input retains a separate
single-line element. Textarea adds wrapped line layout, multi-rectangle
selection paint, vertical hit testing, preferred-x Up/Down movement, caret
visibility scrolling, auto-grow measurement, and multiline IME bounds.

Tab and Shift-Tab move form focus. Enter inserts a newline. Textarea does not
invent Tab insertion or Cmd-Enter submission. Autofocus is a one-shot request on
the keyed entity's first mount, is ignored when disabled, and never repeats on
rerender or hot reload.

The controlled value remains authoritative. Native selection, marked text,
scroll position, preferred x coordinate, and measured layout are transient
entity state.

## Accessibility and exclusions

The normalized node retains multiline text-field, value, placeholder,
read-only, disabled, invalid, label, description, and count semantics. It must
remain selectable in read-only mode.

Rich text, attributed runs, Markdown preview, code-editor Tab behavior, syntax
highlighting, and horizontal no-wrap scrolling are separate products.

## Required evidence

- shared editing-core unit tests for grapheme, UTF-16, selection, paste, limit,
  IME commit, undo/redo/coalescing, and controlled reset behavior;
- single-line Input regression coverage after the refactor;
- wrapped layout, mouse hit, vertical movement, selection paint, auto-grow, and
  scroll-to-caret native tests;
- Latin, CJK IME, emoji, combining-mark, read-only, disabled, and autofocus
  interaction tests;
- visual baselines for empty, multiline, error, limit, fixed, and auto-grow
  states;
- a feedback/notes field in `form_showcase`.
