# Pagination specification

Pagination is a stateless Rhai composition over Button, Icon, and Select. It
does not require a native Rust node. Its controlled state is the pair
`{ current_page, page_size }`.

## Public contract

`registry/components/pagination.rhai` exports `Pagination(props)` with:

- required `key: string`;
- `total_items: integer >= 0`;
- `page_size: integer > 0`;
- `current_page: integer >= 1`;
- optional unique positive `page_size_options`;
- `boundary_count` and `sibling_count`, default `1` and bounded to a small
  documented range;
- `show_summary: bool`, default `true`;
- `size: "xs" | "sm" | "md" | "lg"`;
- optional `on_change`, receiving
  `#{ current_page: integer, page_size: integer }`.

Page numbers are one-based. `total_items == 0` has one logical page, so
`current_page` must be 1 and all navigation is disabled. Invalid page size or an
out-of-range controlled page is a component error; Pagination never silently
clamps caller state. When `page_size_options` is non-empty it must contain the
controlled page_size.

Changing page emits the new page and current size. Changing page size emits the
new size and page 1 atomically. Separate page/page-size callbacks are not
provided.

Style parts include at least `root`, `summary`, `controls`, `previous`, `next`,
`page`, `page_current`, `ellipsis`, `page_size`, and `page_size_label`.

## Page-window algorithm

- Boundary pages are shown at both edges.
- Siblings surround the current page.
- A one-page gap is rendered as that page; a gap of two or more pages becomes
  one noninteractive ellipsis.
- The algorithm never duplicates a page and remains stable for one, five, and
  hundreds of pages.

Previous/next and numbered controls use curried Rhai `FnPtr` callbacks to emit
their concrete next state through ordinary Button events. Pagination must not
add a Rust event adapter merely to bind page numbers.

## Locale and composition

When `page_size_options` is non-empty, Pagination renders Select; otherwise the
page-size control is absent. The built-in summary uses locale-formatted numbers
and neutral localized labels rather than attempting a partial plural-rules
engine. Applications may replace or hide the summary through parts/composition.

Directional navigation uses asset-backed Icon sources with explicit LTR/RTL
pairs. Every icon-only button has a localized accessible label.

Pagination is independent of Table. The caller supplies total count and slices
or fetches rows after every emitted state change.

## Exclusions

- direct page-number text input;
- infinite scrolling;
- implicit data slicing or fetching.

## Required evidence

- exhaustive page-window tests around every ellipsis transition;
- zero/one/few/hundreds-of-pages schema and node snapshots;
- atomic page-size reset and disabled-navigation tests;
- keyboard/focus, RTL icon, size, and theme assertions;
- integration in the `data_table` example.
