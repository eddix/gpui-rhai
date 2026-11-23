# Pagination

`components/pagination` · export `Pagination` · version 0.4.3. Generated from
[`registry/components/pagination.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/pagination.rhai); do not edit.

Pagination composes one-based controlled page state; only popup state is local.

```rhai
import "components/pagination" as pagination;

pagination::Pagination(#{ key: "users-pages", label: "Users pagination", total_items: 128, current_page: 3, page_size: 10, on_change: Fn("page_changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `boundary_count` | integer 1–3 | `1` | Pages always shown at each end of the range. |
| `compact` | bool | `false` | Shows only Previous/Next beside the range and optional page-size field, omitting page numbers and ellipses. Independent of the compact density preference. |
| `current_page` | integer ≥ 1 | required | One-based page shown, at most the page count; the caller stores it from the `change` payload. |
| `disabled` | bool | `false` | Disables page and page-size interaction without dimming the range text; closes the size popup. |
| `key` | string | required | Stable identity of this instance; must not be empty, and keys the per-page Select as `<key>-page-size`. |
| `label` | string | required | Accessible name of the navigation landmark. |
| `on_change` | callback or `()` | — | Called with the next `#{ current_page, page_size }` when a page, Previous, Next or a page size is picked. |
| `page_size` | integer ≥ 1 | required | Items per page; the caller stores it from the `change` payload together with `current_page`. |
| `page_size_options` | array of integer ≥ 1 (at most 32) | `[]` | Choices of the per-page Select, without duplicates and including `page_size`; empty hides the Select. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `show_summary` | bool | `true` | Shows the visible item range and total, e.g. 21–30 of 128; an empty collection shows 0 of 0. |
| `sibling_count` | integer 0–3 | `1` | Pages beside the current page. Near an end, spare slots show adjacent pages; the page/ellipsis slot count is stable. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Size of the buttons and the per-page Select; `()` follows the `size` environment. |
| `style` | style | — | Style merged over the root part. |
| `total_items` | integer ≥ 0 | required | Number of items across all pages; the page count is this over `page_size` rounded up, at least 1. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Emitted when a page, Previous, Next or a page size is picked; the payload carries the next page and page size together. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `controls`, `ellipsis`, `info`, `next`, `next_icon`, `page`, `page_current`, `page_size`, `page_size_control`, `page_size_indicator`, `page_size_label`, `page_size_trigger`, `page_size_value`, `page_slot`, `previous`, `previous_icon`, `root`, `summary`.

## Theme

- Tokens: `metrics.pagination_h`, `metrics.pagination_icon`, `metrics.pagination_pad`, `pagination.active`, `pagination.ink`, `pagination.muted`, `space.group`, `spacing.sm`, `spacing.xs`, `spacing.xxs`, `text_muted`, `typography.body`, `typography.pagination`
- Environment: `density`, `size`

## Dependencies

[`components/button`](button.md), [`components/icon`](icon.md), [`components/icon_button`](icon_button.md), [`components/select`](select.md)
