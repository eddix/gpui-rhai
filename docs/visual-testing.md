# macOS visual and interaction testing

Visual baselines supplement logic and semantic tests; they never replace
keyboard, focus, or accessibility assertions. Theme Studio owns one dense,
canonical matrix for every official component: actions, form controls, choices,
status/loading, navigation, data/virtualization, assets, and overlays. The
default metrics and state vocabulary are defined in the
[design specification](design/).

## Shots of an application

`gpui-rhai shot` renders an application's view offscreen and saves PNGs
(macOS only). The window opens off the screen and the frame is read back from
the GPU, so nothing appears and no screen-recording permission is involved:

```sh
gpui-rhai shot --size 1280x800 --theme Nord/Dark --click Settings --out settings.png
gpui-rhai shot --locale ar --out ar.png --key cmd-k --wait 300 --out palette.png
```

The steps run in order: `--size WxH` (1280x800 by default, before the view
mounts), `--theme Family/Variant` or `--theme Variant` in the current family,
`--locale ID`, `--click NAME` (the node whose accessible name is NAME, else the
first whose name starts with it), `--key CHORD` (GPUI syntax, space-separated
for several), `--wait MS` (advances the view's clock, so timers fire), `--out
PATH` and `--steps FILE`. A steps file holds one JSON request per line in the
[automation protocol](automation.md#json-lines-protocol), plus the shot
commands `click` (`name`, or an automation `locator`), `keys`, `wait`
(`millis`), `capture` (`path`), `theme` and `locale`:

```json
{"id":"open","command":"click","locator":{"kind":"role_name","role":"button","name":"Settings"}}
{"command":"capture","path":"settings.png"}
{"id":"q1","command":"query","locator":{"kind":"test_id","id":"theme-select"}}
```

Each line prints one response; the first failure stops the run. Images are in
device pixels, so a 1280x800 window gives a 2560x1600 PNG on a Retina display.

The command runs the application's `examples/shot.rs`, which `gpui-rhai init`
writes. It mounts the view as `src/main.rs` does, so register the same
extensions there, or fakes of the ones that need a network:

```rust
fn main() {
    if let Err(error) = gpui_rhai::offscreen::run(|| gpui_rhai::script_view!().prepare()) {
        eprintln!("shot: {error}");
        std::process::exit(1);
    }
}
```

`gpui_rhai::offscreen` needs GPUI's `test-support`, which gpui-rhai's
`offscreen-pre` feature (`offscreen-fast` on gpui-fast) turns on. Enable it in
`[dev-dependencies]` only, as `init` does; Cargo builds examples with it and
leaves it out of `cargo build` and `cargo build --release`:

```toml
[dev-dependencies]
gpui-rhai = { version = "0.4", default-features = false, features = ["offscreen-pre"] }
```

## Tabs, Button, and Badge control baseline

The [component contracts](design/atoms.md) define control appearance; the [Tabs contract](components/catalog.md#tabs) defines
its public props, semantics, and style parts.

Maintain one contextual specimen containing:

- Tabs with the first, middle, and last item selected; the track inset remains
  the resolved `spacing.xxs` at every outer edge, with no dividers or selected rail;
- content-width and equal-width Tabs, unequal English/CJK labels, icons with
  text and icon-only headers, horizontal/vertical and LTR/RTL layouts;
- underline Tabs with and without a panel and inside a Card: the hairline
  spans the strip, the 2px bar covers it under the selected label only, and
  the keyboard focus frame does not touch the label;
- bar Tabs over a panel: the selected tab is square and covers the strip's
  line; closable and dirty tabs keep their width as the close button appears;
- count capsules and tone lamps in every look: the capsule is
  exactly the label's line height, its figures sit centred, and `99+` fits;
  an add button after the last tab and slots on the content edge;
- normal, focused, hovered, disabled, scrolling, interrupted transition, and
  Reduced/None states; focus must not obscure persistent selection;
- Button and Badge using the same text and color treatment, including a
  filled Badge; their density difference must survive removing color cues.

Capture Default Light/Dark, Tokyo Night/Storm, and Catppuccin Latte/Mocha at
settled frames. Inspect the remaining bundled themes in the same specimen.
Record logical bounds as well as 1x/2x screenshots so text/line-height and
track-inset assertions do not depend on screenshot scaling. Mounted tests must
cover controlled-value rejection, keyboard/disabled behavior, local overflow
scrolling, and an indicator that owns neither input nor accessibility focus.
Also check rapid interrupted transitions, first mount, resize, theme/locale
changes, and Reduced/None presentation without spurious change callbacks.
Button disabled/loading must suppress actions without changing its height;
Badge adds no action focus stop. The permanent geometry and controlled-state
assertions live in `tests/native-keyboard/tests/native/components/control_visuals.rs`.
Also mount the specimen with deliberately different valid Host spacing/radius
overrides and larger typography. Verify all four track insets, text containment,
theme-responsive Button/Badge density, square and rounded rectangular surfaces,
native/virtualized token parity, and theme switching with preserved state.
Testing only bundled themes with identical spacing and zero radii cannot detect
literal-token substitution. Color checks must use the actual foreground/surface
pair for every state, including enabled but unselected Tabs.

Third-party reference screenshots are not project assets. Keep requirement
documents text-only; any product captures used during an audit remain outside
the repository unless their inclusion is explicitly approved for baselines.
Tabs reveal an offscreen selected tab (asserted in
`tests/native-keyboard/tests/native/components/tabs_collection.rs`). The count
capsule keeps `metrics.counter` when a Host enlarges the control typography,
so it can be shorter than an enlarged label; that is not certified behavior.
Do not replace those assertions with snapshot-only tests.

## Deterministic matrix

Capture `settings_panel`, `dashboard_layout`, and `form_showcase` at their
declared viewport sizes across the main matrix:

- Default Light and Default Dark;
- Tokyo Night Night/Storm;
- Catppuccin Latte/Mocha;
- English LTR and Arabic RTL;
- normal and reduced motion at a settled frame.

Capture Theme Studio across all 15 bundled themes, plus its fixed open-Menu
state, Arabic RTL directional Icon state, reduced-motion pointer-triggered
Tooltip state, and New/Open/Import/Save editing paths. Theme Studio has no
checked-in baselines yet.
Capture `embedded_views` once with the small view's Combobox open and duplicate
local Toast IDs visible in the shared Host queue.
The Gallery baselines are rendered offscreen: `scripts/capture-macos-gallery-baselines.sh`
runs `tests/native-keyboard/src/bin/gallery_baselines.rs`, which mounts the
Gallery in a GPUI `VisualTestAppContext` window at (-10000, -10000) and reads
the frame back from the Metal texture with `Window::render_to_image`. Window
managers, other windows and the screen-recording permission therefore never
enter a baseline, and the window keeps its 1280 × 860-point size even under a
tiling window manager. Images are stored in device pixels (2560 × 1720 on a
Retina display). The matrix is four pages (Button, `scene.operations`,
`scene.form`, `scene.settings`) in comfortable and compact, Default Light and
Dark, plus `scene.operations` in Arabic RTL and `table` and `description_list`
in Simplified Chinese. `--pages [density] [theme] [locale]` renders every page
for a review sweep without writing baselines.

Stories remain available for manual checks in a standalone window:

```sh
bash scripts/build-macos-gallery-app.sh components/catalog forms default-light en
bash scripts/build-macos-gallery-app.sh components/catalog forms catppuccin-mocha ar
```

Capture `charts/catalog` after every built-in series has installed its prepared
scene, including Default Light/Dark, one community theme, Arabic RTL, normal,
reduced, and disabled motion, narrow/wide resize, tooltip/crosshair, Cartesian
brush, linked zoom, annotations, Host GeoJSON, and the 100,000-row native line
series. Theme and locale controls must update existing Chart entities without
recompiling Rhai or losing controlled selection/viewport state. The checked-in
matrix now covers Default Light/Dark, Catppuccin Mocha Arabic RTL, Reduced,
None, and the invalid-diagnostics story. Automated mounted tests and the
benchmark continue to guard interaction, resize, streaming, and the complete
off-screen series matrix rather than treating snapshots as functional proof.

The same pass records Operations dashboard Light/Dark, configuration diff,
loading, empty, 1,000-row large data, the partial-failure fixture, its terminal
48% failure/Toast state, an open CommandDialog, HostSlot composition, and the
Motion catalog under None policy.

The complex-control expansion adds:

- `form_showcase` states for searchable/grouped Select, fixed-Clock DatePicker
  in English and Simplified Chinese, and Textarea empty/multiline/error/limit/
  auto-grow states;
- `data_table` states for scalar cells, sorting, single/multiple
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
- Exercise Combobox search, arrows, Home/End, type-ahead, Enter, Escape, outside
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
- Exercise Table row navigation, sorting, select-all semantics, synchronized
  horizontal scrolling, draggable overflow scrollbar,
  vertical-wheel axis isolation, grouped counts/collapse, group-header sticky
  push-off, and bounded visible rows.
  Exercise every Pagination ellipsis/edge transition and page-size reset.
- Exercise Slider pointer preview/one-shot commit and RTL keyboard stepping;
  drag both ScrollArea thumbs and verify auto/always/hidden visibility.
- Open ContextMenu by secondary click at several coordinates; verify Menu
  navigation reuse, Sheet start/end placement, and AlertDialog cancellation.
- Type, fuzzy-filter, rove, and execute Array- and NativeCollection-backed
  Command data; verify CommandDialog registers no global shortcut.
- Open/focus/confirm-close a secondary window; verify app-store propagation and
  per-window theme/locale/state cleanup.

## Offscreen contexts

An offscreen `VisualTestAppContext` (screenshots, scripted captures) runs on
GPUI's test scheduler, whose clock moves only when the test moves it. A mounted
view's timers, animation and its 16ms delivery poll all follow that clock
(0.4.2), so between captures advance it and let the work run:

```rust
visual.executor().advance_clock(std::time::Duration::from_millis(1000));
visual.run_until_parked();
visual.update(|window, cx| window.simulate_next_frame(cx));
```

A `timeout` fires once the advanced time reaches its deadline, and a capture
taken right after an interaction shows an animation at its start. Waiting on real
time does nothing here. Before 0.4.2 timers read the system clock while the
poll waited on the test clock, so timeouts fired only when some other delivery
woke the view.

## Automation split

`scripts/release-smoke.sh` is the unattended local macOS gate: it verifies that
all examples enter an event loop without panic. The hosted workflow runs its
portable checks on a standard Linux runner and cancels superseded runs.
Screenshot and input certification requires an unlocked interactive Mac
session. Computer Use must inspect fresh accessibility state
after every action and must not bypass the lock screen.

`tests/native-keyboard` separately uses GPUI's non-release `test-support` window
to synthesize Tab, Shift-Tab, Enter, Unicode text input, and clipboard
copy/cut/paste plus read-only selection against real renderer tab stops,
official Rhai components, native text editing plus generic Overlay/Layer,
declarative timers, and transactional
`ScriptLifecycle` callbacks. The suite also paints an open child Popover inside
an open parent Dialog and checks that the window survives the paint; GPUI 0.2
panicked on a nested `defer_draw`.
Three-view embedding cases cover automatic bounds, runtime isolation, shared
Host overlays, duplicate local IDs, click-through dismissal, key conflicts,
and dispose/remount.
The independent workspace spans GPUI integration, native document and motion
behavior, and embedded example preparation guards. It contains no
deleted Table/choice/date/toast native constructor and also guards that window-level
pointer-capture listeners register during paint rather than GPUI layout.
The launch additions cover CommandDialog autofocus/filter/Enter execution,
RTL Slider pointer and arrow semantics, and live Component Gallery category and
theme switching.
One case drives a mounted view through test-ID query, retained event dispatch,
semantic action, and deterministic time advance.
Another verifies that explicit Style occlusion blocks pointer hits to painted
sibling content behind the retained node.
Another compares `ctx.event_target_bounds()` from a real click against the
same node's committed automation bounds without creating a render dependency.
The grouped-Table case scrolls through two sections and verifies that the
runtime-owned sticky layer changes to the next retained header at the viewport
edge. Runtime-error cases verify both native selection/copy and Host opt-out.
It guards keyboard/clipboard dispatch mechanics but does not replace platform
IME candidate-window, focus-ring, or accessibility certification.

Run its independent workspace explicitly:

```text
(cd tests/native-keyboard && cargo test -- --test-threads=1)
(cd tests/native-keyboard && cargo clippy --all-targets --features dev-reload -- -D warnings)
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
bash scripts/build-macos-theme-studio-app.sh
bash scripts/build-macos-test-app.sh data_table default-dark en selected
bash scripts/build-macos-test-app.sh form_showcase default-light zh-CN date-picker
bash scripts/build-macos-gallery-app.sh components/split-pane basic default-dark en normal
```

The command prints the unique temporary bundle path. It never replaces an
existing application or baseline. The optional theme, locale, state, and motion
arguments are consumed only by examples that opt into deterministic
visual-test startup; normal runs preserve their documented defaults.
Baselines need no bundle: `bash scripts/capture-macos-gallery-baselines.sh`
and `bash scripts/capture-macos-example-baselines.sh` render them offscreen as
described above. The example tool compiles the five examples in as modules and
calls their `view(...)` builders, so a capture is exactly what the example runs;
focus states are scripted keystrokes and normal-motion cases use a manual clock.
The app wrappers above remain for manual interaction checks.
