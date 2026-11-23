# IntegrationPicker

`patterns/integration_picker` · export `IntegrationPicker` · version 0.1.0. Generated from
[`registry/patterns/integration_picker.rhai`](../../../registry/patterns/integration_picker.rhai); do not edit.

IntegrationPicker browses application integrations and proposes changes. State: the Host owns connection state; selection may be controlled.

```rhai
import "patterns/integration_picker" as integration_picker;

integration_picker::IntegrationPicker(#{key:"tools",integrations:[#{id:"notes",name:"Notes",domain:"notes.local",description:"Find shared notes."}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `app_icon` | node or `()` | — | Decorative application mark. |
| `app_name` | string or `()` | — | Application name displayed alongside the selected integration. |
| `connected` | array of string (at most 128) | `[]` | Accepted connected IDs. Toggles request a change only. |
| `default_value` | string or `()` | — | Initial local selection. |
| `disabled` | bool | `false` | Disable all actions, keeping accepted state visible. |
| `integrations` | array of object (at most 128) | required | Ordered integration metadata; does not register a service or credential. |
| `key` | string | required | Stable integration browser identity. |
| `on_back` | callback or `()` | — | Back button or logical back arrow in an empty search. |
| `on_close` | callback or `()` | — | Escape after the search is already empty. |
| `on_explore` | callback or `()` | — | Receives the ID the user wants to inspect. |
| `on_select` | callback or `()` | — | Receives a selected ID proposal. |
| `on_toggle` | callback or `()` | — | Receives #{id,connected}; actual connection remains Host-owned. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `value` | string or `()` | — | Accepted selection; empty means none. Omit for local selection. |

### `integrations[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `category` | string or `()` | — | Secondary caption and search term. |
| `description` | string | required | Literal details of the integration. |
| `disabled` | bool | — | Skips the entry in selection and blocks modification. |
| `domain` | string | required | Displayed site or namespace. Never opened by this component. |
| `features` | array of string (at most 3) | — | Optional concise capabilities. |
| `icon` | node or `()` | — | Decorative small logo provided by caller. |
| `id` | string | required | Unique nonempty integration ID. |
| `name` | string | required | Display name and primary search term. |
| `tone` | `"blue"` or `"violet"` or `"green"` or `"amber"` or `"gray"` or `()` | — | Optional logo tile tone. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `back` | `on_back` | none | User requested navigation to the previous screen. |
| `close` | `on_close` | none | User requested closing the picker. |
| `explore` | `on_explore` | string | Inspection request, with the accepted selected ID. |
| `select` | `on_select` | string | Proposed integration ID. |
| `toggle` | `on_toggle` | object | Explicit request to change a connection. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `art`, `back`, `bridge`, `card`, `connected`, `description`, `detail`, `detail_title`, `domain`, `empty`, `explore`, `feature`, `features`, `group`, `icon`, `large_tile`, `list`, `rings`, `root`, `row`, `row_name`, `search`, `search_icon`, `search_input`, `selected`, `shortcut`, `tile`, `tile_mark`, `title`, `toggle`, `total`, `tray`.

## Theme

- Tokens: `accent`, `border`, `catalog.mark_amber`, `catalog.mark_app`, `catalog.mark_blue`, `catalog.mark_gray`, `catalog.mark_green`, `catalog.mark_violet`, `catalog.tile_amber`, `catalog.tile_app`, `catalog.tile_blue`, `catalog.tile_gray`, `catalog.tile_green`, `catalog.tile_violet`, `focus_ring`, `radius.lg`, `radius.md`, `radius.sm`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.catalog_initials`, `typography.catalog_initials_large`, `typography.control`, `typography.title`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/input`](../components/input.md), [`components/switch`](../components/switch.md), [`patterns/list_detail`](../patterns/list_detail.md)
