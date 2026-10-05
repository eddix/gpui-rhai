# macOS visual baselines

Baselines follow the deterministic matrix in `docs/visual-testing.md`. They are
evidence tied to an explicit environment, not portable pixel-perfect promises.

## Capture environment

- Original capture: 2026-08-28; every file was recaptured for 0.2.0 on
  2026-10-05 (examples) and 2026-10-04 (Gallery)
- Environment of the current files: macOS 27.0, gpui-pre 0.3.7, Rust 1.95.0,
  rendered offscreen and read back from the GPU texture (no window chrome, no
  screen capture), stored in device pixels (2x)
- Settings: 640 × 520 points / 1280 × 1040 pixels
- Dashboard: 760 × 560 points / 1520 × 1120 pixels
- Form: 760 × 720 points / 1520 × 1440 pixels
- Data Table: 980 × 720 points / 1960 × 1440 pixels
- Embedded Views: 900 × 420 points / 1800 × 840 pixels
- Gallery: 1280 × 860 points / 2560 × 1720 pixels

Examples: `scripts/capture-macos-example-baselines.sh [example] [case]`.
Gallery: `scripts/capture-macos-gallery-baselines.sh [case]`.

## Recorded cases

- `settings_panel/default-light.en.ltr.png`
- `settings_panel/default-dark.en.ltr.png`
- `settings_panel/default-dark.zh-cn.button-focus.png`
- `settings_panel/default-dark.zh-cn.switch-focus.png`
- `settings_panel/tokyo-night.en.ltr.png`
- `settings_panel/tokyo-storm.en.ltr.png`
- `settings_panel/catppuccin-latte.en.ltr.png`
- `settings_panel/catppuccin-mocha.en.ltr.png`
- `settings_panel/catppuccin-mocha.ar.rtl.png`

Dashboard records the same six LTR themes plus Catppuccin Mocha Arabic RTL in
both `normal` and `reduced` motion. The normal indeterminate Progress position
is runtime evidence rather than a stable pixel oracle; the matching `reduced`
case is the deterministic regression baseline and keeps a static midpoint
indicator visible without scheduling frames.

The refreshed Form files record the six LTR themes, Catppuccin Mocha Arabic RTL,
and fixed Default Light Dialog and Toast states with Select, DatePicker, and
Textarea in the 760 × 720-point layout. The Dialog/Toast startup state and toast
duration come from the visual-test environment, not pointer automation.

Data Table records Default Light populated, loading, and empty states, Default
Dark selected rows, and Catppuccin Mocha Arabic RTL. These cases cover scalar
cells, horizontal overflow, controlled sorting/selection, selection indicators,
bounded skeleton rows, empty copy, and Pagination.
The new deterministic `grouped` state exercises counted/collapsible sticky
sections; its refreshed screenshot is pending the next unlocked visual pass.

### Data Table 0.1.8 candidate refresh

The five Table files were recaptured on 2026-10-04 from frozen local-fix source
`43732b89`, following the 2026-10-03 `53a62e3f` capture, using gpui-pre 0.3.7 on
macOS 27.0.1 (26A434). Their environment supersedes the older environment
above for these five files only. See [capture.json](data_table/capture.json)
for binary/raw/baseline hashes and per-case state.

Native AX and CoreGraphics measurements agree before and after every capture:
integer origin `(100,100)`, window `980×752`, content `980×720`, scale 2.
The native `screencapture -x -o -l` output is a real `1960×1504` PNG,
normalized only from 2x to 1x. No width-to-fit squeeze, crop, compositing,
or screen-control-marker retouching was performed. Raw images and frame
measurements are archived in the linked audit evidence.

Only the five exact test bundles were temporarily excluded from Rift
management for native placement, with explicit user permission. The original
11 runtime rules and all other runtime settings were restored afterward;
owned test processes were closed and the pre-existing test instance retained.
The native title bar is included: the system chrome is Light on 2026-10-04
(Dark on 2026-10-03), independently of the Rhai content theme. No OS appearance
setting was changed for capture. Decoded comparison against the previous five
PNGs confines differences to rows 0–33: the title bar and its adjacent boundary;
rows 34–751, containing the controls, are pixel-identical. This is a frozen-source
recapture, not a new control-design change.

The refresh shows shared resolved column widths and resize separators,
contained loading/empty shells and pagination, genuine two-row accent
selection in Default Dark, and reversed logical columns/Arabic pagination in
Mocha RTL. The default fixture fits all six columns; these PNGs do not prove
horizontal scrolling. Overflow reachability and state transitions remain
covered by the formal native Table tests.

Theme Studio replaced the old Component Gallery on 2026-09-01. An unlocked
manual pass on 2026-09-05 verified its independent editor/specimen scrolling
and the shared specimen across the Gallery's 15-theme, category, compact,
regular, RTL, reduced-motion, Sheet, CommandDialog, and AlertDialog matrix.
The 2026-09-27 Acceptance Application pass recorded 29 story-shell Gallery
PNGs; with Resizable and SplitPane they reached 31 and the inventory 69.

On 2026-10-04 the 0.2 Gallery replaced them. The Rust story shell they showed no
longer exists, and the 0.2 components look different, so the 31 files were
removed rather than relabeled. The 19 new files (`gallery/*.png`) are four pages
(Button and the operations, form and settings scenes) in comfortable/compact ×
light/dark, `scene.operations` in Arabic RTL, and `table` and
`description_list` in Simplified Chinese. They are rendered offscreen by
`scripts/capture-macos-gallery-baselines.sh`. The inventory is now 57: 38
product-context and shared-Host captures plus 19 Gallery captures.

On 2026-10-05 the five examples were rewritten with the 0.2 layouts and
patterns (Region, FormLayout, DataView, Section, Stat, DescriptionList) and all
38 example files were recaptured offscreen with the same case matrix. The focus
cases come from scripted keyboard traversal in the capture tool (Tab, arrows,
Space), and normal-motion cases run on a manual clock, so the indeterminate
Progress is captured at a fixed moment.

`embedded_views/default-dark.shared-host.png` records three independent Rhai
views inside one host-owned GPUI layout. It proves compact responsive sizing for
the 200-point view, a 280-point Combobox escaping that view, two same-local-ID
Toasts stacked by the shared Host queue, and distinct `view_id` values sharing
one `window_id`.

The two Settings focus cases come from real Tab/Shift-Tab traversal. Return
switched the live locale to Simplified Chinese, Space toggled Switch state, and
focus survived the keyed rerenders. The Switch case also guards transparent
controls against a focus shadow filling their whole hit area.

The Arabic cases prove logical row reversal, text alignment, and start/end
placement across headers, Combobox, Tabs, tags, FormField, language buttons,
Switch, RadioGroup, Accordion, and progress content. These initial baselines
were recorded after fixing the host surface gutter, semantic default/on-fill
text colors, native Tab stops, theme-token focus rings, native Input placeholder,
Checkbox mark centering, and reduced-motion looping transitions.

The 38 retained product/Host baselines were refreshed on 2026-09-01 after applying the shared
registry design system across all six themes. The accepted change covers
square terminal-native geometry, compact control metrics, border-led state
hierarchy, flat tab rails, consistent icon sizing/tint, content-sized choice
focus rings, clipped and caret-following single-line Input, and corrected
trigger action grouping. The two Settings focus cases were recaptured through
real Tab traversal after the content-sized Switch focus fix.

The same-day alignment correction makes fixed-height Text nodes honor their
declared cross/main-axis centering, centers the native Input line box, removes
default Combobox chrome around custom triggers, and normalizes every bundled
component SVG to an explicit 24 × 24 coordinate space.

The platform CJK candidate/composition case listed in
`docs/visual-testing.md` passed manual certification on 2026-08-30, alongside
the native integration coverage described below.

## Interaction evidence from this capture session

Passed on the real macOS window:

- initial host focus enters the first control; Tab/Shift-Tab traverse keyed
  controls with visible rings;
- Enter changes locale, Space toggles Switch, and state survives rerender;
- searchable Combobox opens from the keyboard, accepts native text, filters,
  handles Escape, and restores its stable trigger focus; option click closes
  and commits the controlled theme selection;
- Dialog traps forward/reverse focus across actions, Escape closes it, and the
  next Tab reaches Input after restoring the prior host focus;
- pointer clicks outside concurrent Dialog/child Popover/Popover/Combobox
  dismiss exactly one ownership layer at a time, then restore trigger focus;
- Tooltip pointer enter shows after its 250 ms executor timer and pointer leave
  hides after its 100 ms timer, with placement verified beside the trigger;
- the automatically opened Settings window reaches the foreground, renders
  official Button components from its own engine, observes app-store updates,
  cancels a native close request, and closes after explicit confirmation while
  the main window remains alive;
- three independent ScriptViews render in one host-owned window; the 200-point
  view reports `compact`, its 280-point Combobox escapes into its neighbour,
  two same-local-ID Toasts stack in one Host queue, and clicking the neighbour
  both dismisses the Combobox and increments that view;
- explicit disposal removes the third view and its resources, and remounting
  the same `view_id` starts from fresh state.
- DatePicker opens with ArrowDown, navigates by day, commits with Return without
  reopening, closes with Escape, formats its fixed date in Simplified Chinese,
  and keeps disabled boundary cells blank;
- Select opens from the keyboard, filters `Germany` from the query `ger`,
  commits with Return, resets the query when reopened, and clears to `null`;
- Textarea accepts real newlines, auto-grows, updates its grapheme counter, and
  preserves combining input in the controlled value. With the host switched to
  `鼠须管`, a real candidate was committed as `候选窗已出现` and produced the
  expected `6 / 240` count; a second `khk` composition remained marked without
  changing that count, then Escape removed it without changing the controlled
  value;
- Table sorts ascending and descending, selects all visible rows, switches to
  single selection, pages to a fresh controlled slice, resets atomically after
  changing page size, and shows distinct bounded skeleton rows plus the empty
  state. Overflow now exposes a themed draggable horizontal scrollbar; dragging
  it moves header/body together, while a subsequent vertical wheel page changes
  visible rows without changing the horizontal offset;
- Arabic RTL reverses Table's logical column order and Pagination controls while
  keeping logical start/end alignment and the horizontal overflow origin.

Additionally, `tests/native-keyboard` passes synthesized GPUI integration for
disabled-node skipping, forward/reverse traversal, Enter activation, searchable
single-select query/change/close callbacks through a real `ScriptLifecycle`,
Unicode (`中文😀é`) controlled Input, Cmd-A/C/X/V clipboard behavior, and
read-only selection/copy with edit suppression.

The independent native workspace now has 42 tests: 40 GPUI integration cases
plus `table_1000` and Component Gallery preparation guards. It verifies
executor-clock Toast expiry, Menu trigger→panel focus, separator-skipping roving selection, Enter
action/close, nested parent/child overlay painting, event-time target bounds
from a real click, grouped Table sticky-header replacement, and selectable or
Host-suppressed runtime errors. It also verifies App installation and key conflicts, shared
Host mechanics with isolated runtime state, duplicate local Overlay ID
namespacing, automatic bounds, click-through dismissal, and dispose/remount.
Separate Host domains in one window are also proven not to dismiss each other's
overlays. Official source tests preserve nested submenu parent IDs, while
CommandDialog autofocus/filter/Enter execution, RTL Slider pointer and arrow
semantics, and live Component Gallery category/theme switching now run through
real GPUI windows as well.
OverlayManager tests prove top-down parent/child dismissal and topmost Escape
behavior.

This interaction pass found and fixed three release-only failures: a child
overlay attempting `defer_draw` during its parent's deferred prepaint, secondary
window engines starting without the compiled component export registry, and a
confirmed self-window close recursively updating the active GPUI window.

The CJK certification required one physical input-source switch because the
Computer Use transport cannot dispatch global shortcuts or address
`SystemUIServer` menu extras. Once `鼠须管` (default schema: Wubi 86) was active,
the same real macOS window provided candidate commit, marked-text, counter, and
Escape-cancellation evidence. GPUI 0.2.2 still cannot expose the custom controls
in the system AX tree, as documented in `docs/accessibility.md`.
