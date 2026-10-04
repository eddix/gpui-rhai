# Design decision log

Dated decisions that refine the design documents. Grilling decisions from
2026-10-04 are summarized first; later entries record details decided during
implementation. Runtime mechanism decisions also get an ADR.

## 2026-10-04 — grilling consensus

| ID | Decision |
|---|---|
| G1 | Visual sources: cassette futurism (instrument panels), Swiss typography, Rams-era industrial design; lo-fi only for warm neutrals; vaporwave and mid-century graphic design excluded. |
| G2 | Recognizability through five structural signatures (label voice, indicator bar, square lamps, visible shortcuts, keyed tags), each removable through parts. |
| G3 | Default palette: paper, ink and one cobalt spot color. |
| G4 | Normal is quiet: healthy states are neutral; success confirms completed actions and recovery. |
| G5 | Density: size and density are independent axes; density is a declared, inherited environment value; comfortable (32px controls, 14/22 body) is the default for 27-inch 4K displays; compact shrinks heights and outer spacing but not type. |
| G6 | Radius: square by default; roles `sm`/`md`/`lg`; a profile may use a micro radius up to 4px. |
| G7 | The specification governs defaults and choice, not existence: Button keeps seven variants, Tag keeps color variants, the motion pack stays (shown in an Effects group). |
| G8 | Marker system "four silhouettes": Button block, Tag tape, Badge lamp (strong emphasis is a solid block), Kbd keycap. |
| G9 | Focus: 2px reserved border in `focus_ring`, default ink. |
| G10 | Layers: neutral runtime, L0 tokens, L1 components, L2 `layouts/` (neutral) and `patterns/` (opinionated), profiles via `gpui-rhai init --profile`. |
| G11 | Composite components designed from scratch with semantic slots; violations reported by an audit rather than rejected. |
| G12 | Spacing: density-aware scale usable everywhere; relationships expressed mainly by L2 structure; `unit`/`related`/`group`/`section` are optional aliases; the audit checks nesting on resolved geometry. |
| G13 | Content inset contract: one text edge for rows and panels. |
| G14 | Displayed shortcuts derive from actions; the runtime exposes structured bindings. |
| G15 | Composition audit at runtime plus static checks; rule sets come from a profile; no profile means no design rules. |
| G16 | Runtime neutrality: open token registry with component-declared requirements; open typography roles; declarative environment values; token reads and color derivation; Rust-side component derivations move to L0. |
| G17 | Environment values resolve during native rendering (final model for presentation); logic-level environment through deferred construction is a separate proposal (ADR 0023). |
| G18 | Monospace family: system fonts (`Menlo`, then `DejaVu Sans Mono`), no bundled font. |
| G19 | Gallery is rebuilt as a real application from AppShell and L2 with three gates: zero audit findings, keyboard-only scenes, baselines in both densities and modes. |
| G20 | Scope: all 62 components; 0.2.0 with runtime API 3; no compatibility aliases. |

## Implementation decisions

| ID | Date | Decision | Reason |
|---|---|---|---|
| D1 | 2026-10-04 | Dark default accent stays a deep cobalt (`#3d5fe0`) carrying white text; accent-as-text uses the derived `text.accent`. | White text on a blue fill and blue text on graphite cannot both reach 4.5:1; fill identity matters more, and `readable` derives the text color. |
| D2 | 2026-10-04 | Fields have a 2px `border` frame on a `surface_raised` well; pressable controls are blocks. | Tonal fields and tonal buttons would be indistinguishable; the frame marks "you can type here", and a 2px low-contrast frame reads like a 1px line while reserving the focus border. |
| D3 | 2026-10-04 | Outline treatments draw their frame with the reserved 2px border in the `border` role. | GPUI paints one uniform border per quad; a 2px low-contrast line has the visual weight of a 1px line and avoids nested focus nodes. |
| D4 | 2026-10-04 | Tabs labels keep weight 400 in both states. | Changing weight on selection makes slot widths jitter. |
| D5 | 2026-10-04 | The label voice uses the mono family because GPUI 0.3.7 has no letter spacing. | Uppercase proportional text without tracking is cramped. |
| D6 | 2026-10-04 | `SF Mono` is not used as a family name. | CoreText does not resolve it by name and silently falls back to Helvetica, which is not monospaced. |
| D7 | 2026-10-04 | Markers keep 20px height and their padding in both densities. | Markers sit inside rows; shrinking them in compact rows harms CJK legibility without saving space. |
| D8 | 2026-10-04 | Button's default variant is `secondary`. | "Normal is quiet": an unqualified Button must not compete with the one primary action of a region. |
| D9 | 2026-10-04 | Control text uses `control_small` (13/16) and `control_regular` (14/20) instead of the body roles. | Specimen measurement: a 22px body line plus the reserved 2px focus border made compact `sm` controls 26px instead of 24 and `xs` 24 instead of 20. Font sizes are unchanged, so density still never changes type size. |
| D10 | 2026-10-04 | The Tag segment prop is `facet`, the derived color `tag.facet`. | `key` is the reserved component instance key. |
| D11 | 2026-10-04 | Runtime gains `group_focus` and `focus_within` styles; focus-styled nodes now honor their tab stop policy. | A compound control (Checkbox, Switch, Tabs, list rows) must show focus on its mark without moving the content edge; field groups must frame a native input's focus. GPUI applied the element tab policy only to handles it creates, so persistent handles were never tab stops. |
| D12 | 2026-10-04 | ToggleGroup uses roving focus over its segments. | The previous single-tab-stop group had no visible keyboard cursor. |
| D13 | 2026-10-04 | Overlays gain a cross-axis `align`; menus and field panels open start-aligned. | Centered dropdowns drift away from their trigger and get clamped at window edges. |
| D14 | 2026-10-04 | Table, ScrollArea, CodeViewer and DiffViewer draw no outer frame. | Regions provide edges; framed content inside framed regions doubles the lines. |
| D15 | 2026-10-04 | Button shows its shortcut legend at every size. | Rhai cannot read the resolved size; the caller omits the legend for `xs`. |
| D16 | 2026-10-04 | `ctx.action_enabled(id)` is read at render and is not dependency-tracked. | Components re-render with their parent; a host that changes enablement without other state change must trigger a render. |
| D17 | 2026-10-04 | Chart palettes 5 and 8 derive from accent/success and accent/danger mixes. | Hue separation rules leave too few independent hues in most palettes. |
| D18 | 2026-10-04 | Fill-height virtual lists defer their first reveal until the viewport is measured. | Without a configured height there is no estimate; revealing early top-aligned the target and hid preceding group headers. |
| D19 | 2026-10-04 | `ScriptApplication` writes fatal startup errors to stderr before quitting. | On macOS quitting terminates the process, so `run` never returned the error. |
