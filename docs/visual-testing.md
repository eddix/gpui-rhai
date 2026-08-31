# macOS visual and interaction testing

Visual baselines supplement logic and semantic tests; they never replace
keyboard, focus, or accessibility assertions. `component_gallery` supplements
the four product examples with Collapsible, Icon, Menu, Skeleton, and Tooltip
coverage.

## Deterministic matrix

Capture `settings_panel`, `dashboard_layout`, and `form_showcase` at their
declared viewport sizes across the main matrix:

- Default Light and Default Dark;
- Tokyo Night Night/Storm;
- Catppuccin Latte/Mocha;
- English LTR and Arabic RTL;
- normal and reduced motion at a settled frame.

Capture `component_gallery` for its fixed open-Menu state, Arabic RTL
directional Icon state, and reduced-motion pointer-triggered Tooltip state.
Capture `embedded_views` once with the small view's Dropdown open and duplicate
local Toast IDs visible in the shared Host queue.

The complex-control expansion adds:

- `form_showcase` states for searchable/grouped Select, fixed-Clock DatePicker
  in English and Simplified Chinese, and Textarea empty/multiline/error/limit/
  auto-grow states;
- `data_table` states for scalar/custom cells, sorting, single/multiple
  selection, loading, empty, striped rows, horizontal overflow, Pagination, and
  RTL logical alignment;
- a fixed Clock, fixed generated row data, and settled scroll offsets recorded
  in each new baseline manifest.

Record macOS version, GPUI version, display scale, viewport, theme, locale,
component state, and the reason for every accepted baseline change. Store PNGs
under `tests/visual/macos/<example>/<case>.png` once captured.

## Interaction matrix

- Traverse every interactive element with Tab and Shift-Tab; focus must remain
  visible and modal dialogs must trap then restore it.
- Activate controls with Enter/Space and validate disabled/loading suppression.
- Exercise RadioGroup and Tabs in horizontal LTR/RTL plus vertical modes.
- Exercise Dropdown search, arrows, Home/End, type-ahead, Enter, Escape, outside
  click, and 5,000-item virtualization.
- Exercise nested Popover/Dialog/Menu dismissal and submenu parent ownership.
- Hover Tooltip delays and Toast pause/resume/automatic/manual dismissal.
- Enter Latin, Simplified Chinese IME composition, emoji, combining text,
  selection replacement, copy/cut/paste, and read-only selection in Input.
- Repeat the input matrix in Textarea, including visual-line Up/Down, multiline
  selection, internal caret scrolling, max-length paste/IME commit, autofocus,
  fixed rows, and auto-grow under width changes.
- Exercise Select groups/search/clear, DatePicker day/week/month/year-boundary
  navigation and min/max, and locale switching while their panels are open.
- Exercise Table row navigation, sorting, select-all semantics, custom-cell Tab
  order, synchronized horizontal scrolling, draggable overflow scrollbar,
  vertical-wheel axis isolation, sticky header, and bounded visible rows.
  Exercise every Pagination ellipsis/edge transition and page-size reset.
- Open/focus/confirm-close a secondary window; verify app-store propagation and
  per-window theme/locale/state cleanup.

## Automation split

`scripts/release-smoke.sh` is safe for unattended macOS CI: it verifies that all
examples enter an event loop without panic. Screenshot and input certification
requires an unlocked interactive Mac session. Computer Use must inspect fresh
accessibility state after every action and must not bypass the lock screen.

`tests/native-keyboard` separately uses GPUI's non-release `test-support` window
to synthesize Tab, Shift-Tab, Enter, Unicode text input, and clipboard
copy/cut/paste plus read-only selection against real renderer tab stops,
official Rhai components, native text editing plus generic Overlay/Layer,
declarative timers, and transactional
`ScriptLifecycle` callbacks. The suite also paints an open child Popover inside
an open parent Dialog, guarding GPUI 0.2.x against nested `defer_draw` panics.
Three-view embedding cases cover automatic bounds, runtime isolation, shared
Host overlays, duplicate local IDs, click-through dismissal, key conflicts,
and dispose/remount.
The suite currently has 14 production-renderer cases and contains no deleted
Table/choice/date/toast native constructor. It also guards that window-level
pointer-capture listeners register during paint rather than GPUI layout.
It guards keyboard/clipboard dispatch mechanics but does not replace platform
IME candidate-window, focus-ring, or accessibility certification.

Run its independent workspace explicitly:

```text
(cd tests/native-keyboard && cargo test -- --test-threads=1)
(cd tests/native-keyboard && cargo clippy --tests -- -D warnings)
```

Runtime-only automation injects `ManualRuntimeClock` through the file or
embedded view builder, advances it explicitly, and polls a frame. This gives
timers and animation a single deterministic timeline without process sleeps;
platform screenshot tests still require an unlocked interactive session.

Command-line example binaries do not have a macOS application bundle identity,
so accessibility automation may not be able to address them by application.
Build a temporary, non-destructive `.app` wrapper around an already-built
release example when updating visual or interaction baselines:

```text
bash scripts/build-macos-test-app.sh settings_panel
bash scripts/build-macos-test-app.sh settings_panel catppuccin-mocha ar
bash scripts/build-macos-test-app.sh form_showcase default-light en dialog
bash scripts/build-macos-test-app.sh dashboard_layout tokyo-night en default reduced
bash scripts/build-macos-test-app.sh component_gallery default-light en menu
bash scripts/build-macos-test-app.sh data_table default-dark en selected
bash scripts/build-macos-test-app.sh form_showcase default-light zh-CN date-picker
```

The command prints the unique temporary bundle path. It never replaces an
existing application or baseline. The optional theme, locale, state, and motion
arguments are consumed only by examples that opt into deterministic
visual-test startup; normal runs preserve their documented defaults.
