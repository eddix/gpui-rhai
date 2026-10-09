# Real-window checks on macOS

These scripts post CGEvent input to a real Gallery or `multi_window` window and
take window screenshots. They cover the release checklist's real-window gate:
keyboard, focus, clipboard, multi-window, the Rhai title bar's drag and double
press, and direct manipulation that must settle without further input. IME
preedit and VoiceOver are checked by hand.

## Running a step

1. Build the release CLI and example:

   ```text
   cargo build --release -p gpui-rhai-cli
   cargo build --release -p gpui-rhai --example multi_window
   ```

2. Allow the terminal to control the computer (Accessibility) and to record
   the screen (Screen Recording) in System Settings.
3. Open the window the step expects, for example
   `target/release/gpui-rhai gallery --page input`.
4. Run the step with the shared library in front of it:

   ```text
   mkdir -p /tmp/rw
   cat lib.swift keys.swift > /tmp/rw/run.swift && swift /tmp/rw/run.swift
   ```

Screenshots go to `/tmp/rw/`; compare them in RGB. The steps find the window
by owner name and press at offsets from its top-left corner, measured in the
window size of the run that wrote them. A window of another size moves the
targets: check a step's offsets against a screenshot before trusting its
result.

## Steps

| Step | Window | Checks | Expected |
|---|---|---|---|
| `titlebar.swift` | Gallery | drag the title bar's empty strip; a double press | the window moves while dragged; the double press runs the platform action (zoom) |
| `control.swift` | Gallery | press and drag starting on the `Compact` segment | the window stays; the click selects compact density |
| `keys.swift` | Gallery, Button page | Tab three times; Cmd+K, letters, Escape, Escape | the focus frame moves Deploy, Export, Duplicate; the palette opens with its input focused; the second Escape closes it and the window is pixel-identical to before Cmd+K |
| `type.swift` | Gallery | Cmd+K and typing | the palette input takes the text; with a Chinese input source letters become marked text and the first Escape cancels the composition |
| `clip.swift` | Gallery, Input page | Cmd+A Cmd+C, Cmd+V, Cmd+A Cmd+X | the pasteboard and the fields follow each copy, paste and cut |
| `ctx.swift` | Gallery, Table page | right press on a row; click another row, then Shift+F10 | the row is selected and the menu opens at the pointer; then under the clicked row |
| `mw.swift` to `mw6.swift` | `multi_window` | shared and per-window stores, the close request Dialog, keyboard close, reopen, a per-window theme | a shared count in both windows and a draft in one; the Dialog shows its focus frame on open; Tab, Tab, Return closes the window; the theme changes one window only |
| `spdrag.swift`, `pzdrag.swift`, `rotdrag.swift`, `dragdrag.swift` | Gallery, SplitPane, PanZoom, Rotatable and drag-and-drop pages | drag, release, then no input | right after release each shows the committed position, and nothing moves when the pointer leaves the area |

Record the source SHA, the commands, the screenshots compared and the result
of each step with the release candidate.
