# Pagination

`components/pagination` · export `Pagination` · version 0.2.0. Generated from
[`registry/components/pagination.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/pagination.rhai); do not edit.

Pagination is a stateless Rhai composition for one-based controlled page state.

```rhai
import "components/pagination" as pagination;

pagination::Pagination(#{ key: "users-pages", label: "Users pagination", total_items: 128, current_page: 3, page_size: 10, on_change: Fn("page_changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `boundary_count` | integer 1–3 | `1` | Pages always shown at each end of the range. |
| `current_page` | integer ≥ 1 | required | One-based page shown, at most the page count; the caller stores it from the `change` payload. |
| `key` | string | required | Stable identity of this instance; must not be empty, and keys the per-page Select as `<key>-page-size`. |
| `label` | string | required | Accessible name of the navigation landmark. |
| `on_change` | callback or `()` | — | Called with the next `#{ current_page, page_size }` when a page, Previous, Next or a page size is picked. |
| `page_size` | integer ≥ 1 | required | Items per page; the caller stores it from the `change` payload together with `current_page`. |
| `page_size_options` | array of integer ≥ 1 (at most 32) | `[]` | Choices of the per-page Select, without duplicates and including `page_size`; empty hides the Select. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `show_summary` | bool | `true` | Shows the item count and the current page out of the page count before the controls. |
| `sibling_count` | integer 0–3 | `1` | Pages shown on each side of the current page; a gap of more than one page becomes an ellipsis. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Size of the buttons and the per-page Select; `()` follows the `size` environment. |
| `style` | style | — | Style merged over the root part. |
| `total_items` | integer ≥ 0 | required | Number of items across all pages; the page count is this over `page_size` rounded up, at least 1. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Emitted when a page, Previous, Next or a page size is picked; the payload carries the next page and page size together. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `controls`, `ellipsis`, `next`, `page`, `page_current`, `page_size`, `page_size_label`, `previous`, `root`, `summary`.

## Theme

- Tokens: `selection`, `space.group`, `spacing.sm`, `spacing.xs`, `spacing.xxs`, `text_muted`, `typography.body`
- Environment: `density`, `size`

## Dependencies

[`components/button`](button.md), [`components/icon`](icon.md), [`components/select`](select.md)
