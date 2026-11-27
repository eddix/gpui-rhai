# IntegrationSettings

`patterns/integration_settings` · export `IntegrationSettings` · version 0.1.0. Generated from
[`registry/patterns/integration_settings.rhai`](../../../registry/patterns/integration_settings.rhai); do not edit.

IntegrationSettings edits versioned site configuration and requests Host credential actions. State: local field edits; the Host owns accepted values, sync and credentials.

```rhai
import "patterns/integration_settings" as integration_settings;

integration_settings::IntegrationSettings(#{key:"settings",integration:"store",name:"Store",revision:0,value:#{enabled:true,site:"",zone:""}})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `count` | integer ≥ 0 or `()` | — | Optional localized activity count. |
| `credential_busy` | bool | `false` | Accepted generation or rotation in progress. |
| `credential_message` | string or `()` | — | Nonsecret outcome or status for generation, copy or reveal. |
| `credential_present` | bool | `false` | Whether the Host has an API key. |
| `credential_preview` | node or `()` | — | Optional Host-projected display. For real secrets, use native paint or a native reveal surface; ordinary Rhai text is not a secret container. Absent unless explicitly revealed. |
| `credential_version` | integer ≥ 0 | `0` | Nonsecret credential revision echoed in key actions. |
| `credential_visible` | bool | `false` | Whether the Host authorizes mounting credential_preview now. |
| `disabled` | bool | `false` | Keeps accepted values visible but blocks edits and actions. |
| `integration` | string | required | Current integration ID, echoed in every proposal. Switching IDs isolates local drafts. |
| `key` | string | required | Stable settings surface identity. |
| `logo` | node or `()` | — | Optional decorative logo; otherwise an initial is shown. |
| `meta` | string or `()` | — | Subtitle while sync is enabled. |
| `name` | string | required | Integration display name. |
| `on_copy_key` | callback or `()` | — | Receives integration ID and credential version; only the Host handles actual copy. |
| `on_generate_key` | callback or `()` | — | Receives integration ID and credential version for a generation request. |
| `on_reveal_key` | callback or `()` | — | Receives integration ID, credential version and requested visible state. |
| `on_save` | callback or `()` | — | Receives #{integration,revision,site,zone}; the Host may reject it. |
| `on_toggle` | callback or `()` | — | Receives #{integration,enabled}; never toggles sync on its own. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Opaque accepted version echoed in Save requests for compare-and-set. |
| `save_message` | string or `()` | — | Safe Host feedback for a save result. Do not supply raw secrets. |
| `saving` | bool | `false` | Accepted in-flight save state; preserves and temporarily locks current edits. |
| `site_hint` | string or `()` | — | Optional valid-field hint. |
| `style` | style | — | Style merged over the root part. |
| `value` | object | required | Accepted ordinary settings; never includes a credential. |
| `zones` | array of object (at most 1024) | `[]` | Caller-supplied time zones; labels are not computed UTC offsets. |

### `value` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `enabled` | bool | required | Accepted sync state. |
| `site` | string | required | Accepted site domain or empty. |
| `zone` | string | required | Accepted zone ID. |

### `zones[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Application zone identifier. |
| `label` | string | required | Searchable zone label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `copy_key` | `on_copy_key` | object | Explicit user request to copy a key at a specific revision. |
| `generate_key` | `on_generate_key` | object | Ask the Host to generate or rotate a key; no raw key goes through this event. |
| `reveal_key` | `on_reveal_key` | object | Explicit user request to show or hide the key. |
| `save` | `on_save` | object | Versioned nonsecret configuration proposal. |
| `toggle` | `on_toggle` | object | Sync state proposal. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `address`, `body`, `count`, `credential`, `credential_label`, `discard`, `footer`, `generate`, `generate_full`, `header`, `hint`, `icon`, `invalid`, `key_action`, `key_frame`, `key_note`, `key_text`, `label`, `logo`, `meta`, `name`, `prefix`, `resolve`, `root`, `save`, `save_note`, `section`, `site`, `site_invalid`, `sync`, `valid`, `zone`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.control`, `typography.title`
- Environment: `density`

## Dependencies

[`components/accordion`](../components/accordion.md), [`components/button`](../components/button.md), [`components/icon_button`](../components/icon_button.md), [`components/input`](../components/input.md), [`components/select`](../components/select.md), [`components/switch`](../components/switch.md)
