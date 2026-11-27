# PromptLibrary

`patterns/prompt_library` · export `PromptLibrary` · version 0.1.0. Generated from
[`registry/patterns/prompt_library.rhai`](../../../registry/patterns/prompt_library.rhai); do not edit.

PromptLibrary lets the user find templates, edit variables and propose a prompt. State: accepted selection can be controlled; query and drafts are component-local.

```rhai
import "patterns/prompt_library" as prompt_library;

prompt_library::PromptLibrary(#{key:"prompts",templates:[#{id:"summary",title:"Summarize",body:"Write for {audience}."}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `default_value` | string or `()` | — | Initial local selection; defaults to the first catalog entry. |
| `disabled` | bool | `false` | Blocks input and proposals while keeping the preview visible. |
| `key` | string | required | Stable library identity. |
| `on_copy` | callback or `()` | — | Template ID copied in the input transaction; does not duplicate raw text in the event. |
| `on_select` | callback or `()` | — | Template ID proposal. |
| `on_use` | callback or `()` | — | Receives #{id,text} with the filled prompt. No evaluation or submission is performed. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `templates` | array of object (at most 128) | required | Ordered templates to search by title, category or literal body. |
| `title` | string or `()` | — | Accessible and visible library name. |
| `use_label` | string or `()` | — | Optional label for the proposed-use action. |
| `value` | string or `()` | — | Accepted template ID; empty means no accepted selection. Omit for local selection. |

### `templates[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `body` | string | required | Literal template, up to 32 KiB; {name} placeholders are not code. |
| `category` | string or `()` | — | Short badge and additional search term. |
| `id` | string | required | Unique, nonempty template ID. |
| `title` | string | required | Template title and primary search term. |
| `tone` | `"blue"` or `"violet"` or `"amber"` or `"green"` or `"red"` or `"gray"` or `()` | — | Optional category color; otherwise assigned by category order. |
| `variables` | map of object | — | Optional details keyed by placeholder name. |

### `templates[].variables.<key>` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `default` | string or `()` | — | Initial variable text, at most 512 graphemes and 4096 UTF-8 bytes. |
| `description` | string or `()` | — | Hint while the editor has focus. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `copy` | `on_copy` | string | The template ID for a requested clipboard write. |
| `select` | `on_select` | string | Accept or reject the requested template ID. |
| `use` | `on_use` | object | A fully filled prompt, proposed as literal text. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `badge`, `body`, `caption`, `card`, `copy`, `detail`, `detail_title`, `empty`, `field`, `field_editor`, `field_label`, `fields`, `foot`, `list`, `root`, `row`, `row_label`, `search`, `search_icon`, `search_input`, `selected`, `side_head`, `sidebar`, `title`, `total`, `use`, `variable`, `variable_active`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `radius.sm`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.accent`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.control`, `typography.title`
- Environment: `density`

## Dependencies

[`components/badge`](../components/badge.md), [`components/button`](../components/button.md), [`components/input`](../components/input.md), [`components/markdown_viewer`](../components/markdown_viewer.md), [`components/textarea`](../components/textarea.md), [`patterns/list_detail`](../patterns/list_detail.md)
