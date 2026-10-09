# SharedLayoutCards

`motion/shared_layout_cards` · export `SharedLayoutCards` · version 0.1.6. Generated from
[`registry/motion/shared_layout_cards.rhai`](../../../registry/motion/shared_layout_cards.rhai); do not edit.

SharedLayoutCards moves the selected card from the list to a detail area with a shared-layout animation. State: caller-owned selection. Emits no events.

```rhai
import "motion/shared_layout_cards" as shared_layout_cards;

shared_layout_cards::SharedLayoutCards(#{ key: "cards", selected: "a", cards: cards })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `cards` | array of object (at most 128) | required | Cards in list order; every card except the selected one stays in the list. |
| `key` | string | required | Motion identity: names the motion group the cards' shared-layout ids belong to. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected` | string | required | Key of the card shown in the detail area; changing it animates the cards to their new places. |
| `style` | style | — | Style merged over the root part. |

### `cards[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | required | Card identity and shared-layout id; unique among the cards. |
| `title` | string | required | Card text. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `card`, `detail`, `list`, `root`.
