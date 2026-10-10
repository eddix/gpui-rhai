# AppShell

`patterns/app_shell` · export `AppShell` · version 0.2.0. Generated from
[`registry/patterns/app_shell.rhai`](../../../registry/patterns/app_shell.rhai); do not edit.

AppShell is the window frame of a productivity tool: title bar, sidebar, main area, optional inspector, status bar, and keyboard regions.

The sidebar and inspector are raised color blocks beside the main surface; no rules. F6 and Shift+F6 move focus from the region that holds it to the next or previous present region; a region shows the 2px ink frame while it holds focus itself, and Tab then enters it. With focus outside every region they go on from the last region F6 moved to, which the shell keeps as its only state.

```rhai
import "patterns/app_shell" as app_shell;

app_shell::AppShell(#{ key: "app", label: "Workbench", title_bar: #{ title: "Workbench" },
    sidebar: nav_node, main: region_node, status: #{ start: [text("Ready")] } })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `inspector` | node or `()` | — | Raised region after the main area, and an F6 stop. |
| `inspector_width` | length or `()` | — | Width of the inspector; omitted, 320 logical pixels. |
| `key` | string | required | Identity of the shell; the keys of its regions, which F6 focuses, derive from it. |
| `label` | string | required | Accessible name of the shell; it also names the title bar (`<label> title bar`) and the status bar (`<label> status`). |
| `main` | node | required | The main working area; it takes the free width and is always an F6 stop. |
| `overlays` | array of node (at most 8) | `[]` | Nodes placed after the bars, for overlays such as dialogs, sheets and toasts. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `sidebar` | node or `()` | — | Raised region before the main area, and an F6 stop. |
| `sidebar_width` | length or `()` | — | Width of the sidebar; omitted, twice `metrics.label_column`. |
| `status` | object or `()` | — | StatusBar props without `label`, for the bar along the bottom; omitted, the shell has no status bar. |
| `style` | style | — | Style merged over the root part. |
| `title_bar` | object or `()` | — | TitleBar props without `label`, for the bar across the top; omitted, the shell has no title bar. |

### `status` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `center` | array of node (at most 16) | `[]` | Fields centered in the bar. |
| `end` | array of node (at most 16) | `[]` | Fields at the end edge; they keep their width. |
| `start` | array of node (at most 16) | `[]` | Fields at the start edge; they truncate when space runs out. |

### `title_bar` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `center` | array of node (at most 8) | `[]` | Nodes centered in the bar. |
| `end` | array of node (at most 8) | `[]` | Nodes at the end edge; they keep their width while the title truncates. |
| `inset_start` | integer 0–256 | `0` | Logical pixels kept free at the start edge, room for the macOS window buttons. |
| `start` | array of node (at most 8) | `[]` | Nodes before the title, at the start edge. |
| `subtitle` | string or node or `()` | — | Muted text or node on the title's line; it truncates first. |
| `title` | string or node | required | Title text, or a node such as a breadcrumb; a string is set semibold and the subtitle truncates before it. |
| `window_drag` | bool | `false` | Whether pressing the bar's background moves the window and a double press zooms it. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `inspector` | no | no | Raised region after the main area. |
| `main` | yes | no | The main working area. |
| `overlays` | no | yes | Overlays such as dialogs, sheets and toasts. |
| `sidebar` | no | no | Raised region before the main area. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `inspector`, `main`, `region_frame`, `root`, `sidebar`.

## Theme

- Tokens: `focus_ring`, `metrics.label_column`, `surface`, `surface_raised`

## Dependencies

[`components/status_bar`](../components/status_bar.md), [`components/title_bar`](../components/title_bar.md)
