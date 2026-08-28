# macOS visual baselines

Baselines follow the deterministic matrix in `docs/visual-testing.md`. They are
evidence tied to an explicit environment, not portable pixel-perfect promises.

## Capture environment

- Date: 2026-08-28
- macOS: 26.6.2 (25G83)
- GPUI: 0.2.2
- Rust: 1.94.1
- Capture service: Codex Computer Use, one screenshot pixel per logical point
- Settings viewport/capture: 640 × 520 points / 640 × 552 pixels
- Dashboard viewport/capture: 760 × 560 points / 760 × 592 pixels
- Form viewport/capture: 680 × 680 points / 680 × 712 pixels
- Component Gallery viewport/capture: 720 × 520 points / 720 × 552 pixels

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

Form records the six LTR themes, Catppuccin Mocha Arabic RTL, and fixed
Default Light Dialog and Toast states. The Dialog/Toast startup state and toast
duration come from the visual-test environment, not pointer automation.

Component Gallery closes the remaining product-line screenshot gaps with a
Default Light open Menu, Catppuccin Mocha Arabic RTL directional Icon, and a
Default Dark reduced-motion Skeleton plus pointer-triggered Tooltip. Together
the four product examples and this mechanism gallery render every official
component in at least one recorded macOS state.

The two Settings focus cases come from real Tab/Shift-Tab traversal. Return
switched the live locale to Simplified Chinese, Space toggled Switch state, and
focus survived the keyed rerenders. The Switch case also guards transparent
controls against a focus shadow filling their whole hit area.

The Arabic cases prove logical row reversal, text alignment, and start/end
placement across headers, Dropdown, Tabs, tags, FormField, language buttons,
Switch, RadioGroup, Accordion, and progress content. These initial baselines
were recorded after fixing the host surface gutter, semantic default/on-fill
text colors, native Tab stops, theme-token focus rings, native Input placeholder,
Checkbox mark centering, and reduced-motion looping transitions.

The platform CJK candidate/composition case listed in
`docs/visual-testing.md` still requires a physical input-source switch before
the complete interaction gate is declared passed.

## Interaction evidence from this capture session

Passed on the real macOS window:

- initial host focus enters the first control; Tab/Shift-Tab traverse keyed
  controls with visible rings;
- Enter changes locale, Space toggles Switch, and state survives rerender;
- searchable Dropdown opens from the keyboard, accepts native text, filters,
  handles Escape, and restores its stable trigger focus; option click closes
  and commits the controlled theme selection;
- Dialog traps forward/reverse focus across actions, Escape closes it, and the
  next Tab reaches Input after restoring the prior host focus;
- pointer clicks outside concurrent Dialog/child Popover/Popover/Dropdown
  dismiss exactly one ownership layer at a time, then restore trigger focus;
- Tooltip pointer enter shows after its 250 ms executor timer and pointer leave
  hides after its 100 ms timer, with placement verified beside the trigger;
- the automatically opened Settings window reaches the foreground, renders
  official Button components from its own engine, observes app-store updates,
  cancels a native close request, and closes after explicit confirmation while
  the main window remains alive.

Additionally, `tests/native-keyboard` passes synthesized GPUI integration for
disabled-node skipping, forward/reverse traversal, Enter activation, searchable
single-select query/change/close callbacks through a real `ScriptLifecycle`,
Unicode (`中文😀é`) controlled Input, Cmd-A/C/X/V clipboard behavior, and
read-only selection/copy with edit suppression.

The same eight-case native suite now verifies executor-clock Toast expiry, Menu
trigger→panel focus, separator-skipping roving selection, Enter action/close,
and nested parent/child overlay painting. Official source tests preserve nested
submenu parent IDs, while OverlayManager tests prove top-down parent/child
dismissal and topmost Escape behavior.

This interaction pass found and fixed three release-only failures: a child
overlay attempting `defer_draw` during its parent's deferred prepaint, secondary
window engines starting without the compiled component export registry, and a
confirmed self-window close recursively updating the active GPUI window.

Still manual: a real CJK input-source candidate/composition window. The host
has `美国` and `鼠须管` (default schema: Wubi 86), but the Computer Use transport
cannot dispatch global input-source shortcuts or address `SystemUIServer` menu
extras. The original `美国` source was verified in a native Finder text field
after the attempt. GPUI 0.2.2 also cannot expose the custom controls in the
system AX tree, as documented in `docs/accessibility.md`.
