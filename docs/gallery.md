# Gallery and acceptance application

`gpui-rhai gallery` is the first-party acceptance application and the
authoritative interactive catalog for the public gpui-rhai surface. It replaces
the former Component, Motion, and Chart Cargo gallery examples.

```sh
gpui-rhai gallery
gpui-rhai gallery --list
gpui-rhai gallery --story components/tabs
gpui-rhai gallery --story apps/operations --case config-diff
gpui-rhai gallery --theme default-dark --locale zh-CN
```

When running from the repository, substitute
`cargo run --release -p gpui-rhai-cli --` for `gpui-rhai`. `--list` prints the
stable story ID, category, cases, and title without initializing GPUI or opening
a window. Unknown story, case, theme, and locale values return an explicit
error. Gallery uses only bundled sources and deterministic fixtures: it does not
read or write the launch directory, require network access, inspect a real
machine, or execute commands.

## Explore

Explore combines a searchable, registry-driven story list with the actual
running preview and a read-only CodeViewer for the same bundled Rhai source.
Search matches titles, stable IDs, keywords, and public module IDs. Category
filtering never changes the selected story.

Every story exposes:

- a stable ID, title, purpose, category, keywords, and public module coverage;
- one or more deterministic cases and the observation goal for the selected
  case;
- the running Rhai source module, Host fixture boundary, and maintained
  documentation path;
- real pointer/keyboard interaction and observable controlled state;
- Reset, which replaces only that story instance with a fresh generation.

The shell provides Default Light/Dark and `en`/`zh-CN`/`ar` controls. Theme and
locale changes are applied through public `ScriptViewHandle` Host APIs to every
retained story without recompiling or clearing component state.

Viewport controls select Auto, Compact (520 logical pixels), Regular (800), or
Wide (1,120). Fixed presets constrain the actual mounted story surface, so
`ctx.viewport_class()` and responsive dependencies follow normal Runtime
behavior. Fixed previews place the exact source pane below the preview; Auto
uses a side-by-side layout.

Motion controls select `MotionPreference::Normal`, `Reduced`, or `None` through
the public Host motion policy. The policy applies to cached views and to later
mount/reset generations; script requests cannot relax a stricter Host choice.

The catalog is not a coverage spreadsheet. Registry tests compute coverage
from `BUNDLED_COMPONENT_SOURCES_BY_ID`, `BUNDLED_MOTION_SOURCES_BY_ID`, and
`BUNDLED_CHART_SOURCES_BY_ID`; adding a public module without a covering story
fails the registry test. An independent native test prepares, mounts, draws,
and snapshots nonzero semantic geometry for every story/case.

`apps/host-embedding` is the HostSlot acceptance story. The visible outer
source declares only the opaque slot. Gallery mounts a second resident Rhai
view under an independent `ScriptViewHost`, supplies it through
`HostSlotRegistry::with_script_view`, and treats parent plus resident as one
cache/lifecycle group. The resident form exercises Input/Textarea, theme and
locale propagation, Reset, and the native Chinese IME regression.

## Operations Workbench

`apps/operations` is a connected local application rather than a specimen
grid. Its pages share one formal-component state model:

- Dashboard consumes Rust-owned `NativeChartData` and a recent-events
  `NativeCollection`.
- Hosts consumes a Rust-owned, virtualized `NativeCollection`, including
  semantic Badge adornments, Rust-side search/sort/page/group projection,
  Pagination, and a controlled Host details Sheet.
- Configurations combines Input, CodeViewer, DiffViewer, Host-owned revisioned
  `NativeTextDocument` configuration snapshots, and deployment staging.
- Deployments presents bounded Host progress, failure, Progress, Badge, and
  Toast feedback.
- Settings demonstrates state-preserving theme selection.
- A source-owned CommandDialog and navigation reach the same pages.

The normal task is:

1. inspect/select hosts;
2. compare the target configuration;
3. edit the release channel;
4. stage a deployment;
5. cancel without changing the model, or confirm;
6. observe Rust subscription progress and the committed result.

Rust owns the window, lifecycle, fixtures, native collections/chart data, and
the versioned `gallery.operations` subscription capability. Rhai owns page
composition, navigation, business state, confirmation, and result rendering.
The Workbench is itself a normal formal Rhai component: its capability and
effects are declared in metadata, so suspend, reset, unmount, and generation
replacement use the same cleanup rules as application components.

Cases are intentionally deterministic:

| Case | Initial surface | Contract |
| --- | --- | --- |
| `basic` | Dashboard | Normal three-host session |
| `config-diff` | Configurations | Complete cancel/confirm workflow |
| `theme-overrides` | Settings | Host radii override across theme changes |
| `loading` | Dashboard | Loading UI without unbounded background work |
| `empty` | Dashboard | Actionable empty application state |
| `failure` | Configurations | One unreachable host and failed deployment; old configuration remains active |
| `streaming` | Dashboard | Four bounded Rust subscription revisions, no Rhai polling |
| `large` | Hosts | 1,000 Rust-owned rows with virtual realization |

Deployment progress is sent as `15 → 48 → 76 → 100`; the failure fixture sends
`15 → 48 → failed`. The default story creates neither the 1,000-row collection
nor a continuing stream.

## State and lifecycle

Each Explore story owns an independent `ScriptViewHandle`. Switching stories
suspends the old handle and resumes a retained handle. The shell keeps the most
recent eight story/case generations and disposes older entries. Reset disposes
and replaces only the selected key. Theme, locale, viewport, and Motion changes
do not replace business state.

Disposal and suspension remain Runtime operations—not UI hiding. Declarative
effects, capability tasks/subscriptions, timers, Motion, primitive resources,
and late generation deliveries follow the normal view lifecycle. Gallery does
not promise persistence after process exit.

## Verification

The source-backed automated gates are:

```sh
cargo test -p gpui-rhai-cli --all-features
cargo test --manifest-path tests/native-keyboard/Cargo.toml --test gallery
cargo test --manifest-path tests/native-keyboard/Cargo.toml --test keyboard component_catalog_story
cargo test --manifest-path tests/native-keyboard/Cargo.toml --test keyboard motion_catalog_story
cargo test --manifest-path tests/native-keyboard/Cargo.toml --test charts chart_catalog_story
cargo test --manifest-path tests/performance/Cargo.toml --test e2e --no-run
```

`scripts/release-smoke.sh` launches an Operations large-data case from a fresh
temporary current directory and verifies that Gallery creates no files there.
Manual platform matrices and screenshot policy are maintained in
[visual-testing.md](visual-testing.md).

## Non-goals

Gallery does not add an Omarchy adapter, arbitrary UI zoom, third-party story
plugins, a browser/WASM build, CodeEditor, terminal/SSH, real command execution,
network services, or persistence. Those capabilities belong in independent
work or extension crates rather than the core acceptance surface.
