# Kbd

`components/kbd` · export `Kbd` · version 0.4.3. Generated from
[`registry/components/kbd.rhai`](../../../registry/components/kbd.rhai); do not edit.

Kbd presents one framed key or combination, or a bare inline key legend.

Named keys have platform-specific visual and spoken names. A combination uses one keycap, keeps its input order in RTL, and does not register a key binding.

```rhai
import "components/kbd" as kbd;

kbd::Kbd(#{ keys: ["mod", "shift", "p"], size: "md" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `appearance` | `"keycap"` or `"inline"` or `"menu"` | `"keycap"` | keycap frames the whole combination; inline shows a bare legend; menu uses a 20px contextual keycap, inheriting its surrounding menu/command ink. None install bindings. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keys` | string or array of string (at most 8) | `[]` | One named key or a combination in one chip, in input order. Whitespace-only keys are ignored. Supports mod, cmd, ctrl, alt, shift, enter, esc, tab, space, backspace, delete, arrows, pageup, pagedown, home and end; other names stay literal. |
| `label` | string or `()` | — | Accessible name; otherwise uses spelled-out key names, or the literal text. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `platform` | `"mac"` or `"other"` or `()` | — | Explicit key naming convention, or current platform when omitted. Does not change binding semantics. |
| `size` | `"sm"` or `"md"` | `"sm"` | Keycap size, independent of surrounding control size: Modern 18/22px, Clean 16/20px, before reading scale. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | `""` | A literal legend used when keys is empty; not parsed as key names or bindings. Empty content produces no chip. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `glyph`, `inline`, `key`, `keys`, `keys_other`, `menu`, `menu_key`, `menu_keys`, `root`.

## Theme

- Tokens: `metrics.kbd_glyph`, `metrics.kbd_h`, `space.unit`, `typography.key_legend`, `typography.key_legend_cap`, `typography.key_legend_menu`
- Environment: `density`, `size`
