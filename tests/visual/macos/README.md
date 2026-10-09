# macOS visual baselines

Baselines follow the deterministic matrix in `docs/visual-testing.md`. They are
evidence tied to an explicit environment, not portable pixel-perfect promises.

## Capture environment

- Original capture: 2026-08-28; every file was recaptured for 0.2.0 on
  2026-10-05 (examples) and 2026-10-04 (Gallery); on 2026-10-07 the 19 Gallery
  files and the Settings, Form, Data Table and Embedded Views files were
  recaptured for the field padding (`metrics.field_pad`, decision D61); on
  2026-10-08 `form_showcase/default-light.en.dialog` for the Dialog width fix
  and `gallery/table.comfortable.default-dark.zh-CN` for the TabBar entry in
  the Lists sidebar; on 2026-10-09 the 19 Gallery files and the Settings, Form,
  Data Table and Embedded Views files for the field padding of decision D65
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
The example's `grouped` state (counted, collapsible sticky sections) has no
baseline yet; `example_baselines` does not capture it.

Earlier capture sessions, including the 0.1.8 Data Table refresh and the
interaction evidence of the first capture pass, are listed under
[History](../../../docs/history.md); the Table capture manifest is
[capture.json](data_table/capture.json).
