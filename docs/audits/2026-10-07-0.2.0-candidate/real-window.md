# 0.2.0 candidate: real-window checks

Platform: macOS 27.0.1 (26A434), arm64, a Retina display with the rift tiling
window manager. Builds: release `gpui-rhai` CLI and the release `multi_window`
example from `feat/design-system-0.2` at `8371a50`, plus `3b30144` for the
re-run after the fix below. Input was posted with CGEvent from the Swift scripts
in [`harness/`](harness/) (`swift lib.swift <step>.swift` after concatenating the
two); results were read from window screenshots (`screencapture -l`, compared
in RGB), the window list and `NSPasteboard`. Coordinates in the step scripts
are for the window sizes rift gave that session.

| Check | What was posted | Result |
|---|---|---|
| Title bar drag | press on the Gallery title bar's empty strip, drag 100x60 | the window moved while dragging, `(524,42)` to `(567,50)`; rift put it back after release |
| Title bar double press | two presses with click states 1 and 2 | the platform action zoomed the window, width 1196 to 1540 |
| Press on a bar control | press and drag starting on the `Compact` segment | the window stayed at `(524,42)`; the click selected compact density (status bar `DENSITY compact`) |
| Keyboard focus | Tab three times on the Button page | the focus frame moved Deploy, Export, Duplicate (screenshot differences move right) |
| Command palette | Cmd+K, then letters, Escape, Escape | the palette opened with its input focused; with the system's Chinese input source the letters became marked text; the first Escape cancelled the composition, the second closed the palette, and the window was pixel-identical to before Cmd+K |
| Clipboard | Input page: Cmd+A Cmd+C in `eu-west`; Cmd+V of `clip-123` into Service name; Cmd+A Cmd+C; Cmd+A Cmd+X | the pasteboard read `eu-west`; the field showed `clip-123`; the pasteboard read `clip-123`; the field emptied and the pasteboard kept `clip-123` |
| Shared and window stores | `multi_window`: Increment shared counter and Edit this window's draft in the settings window | shared count 1 in both windows; draft `edited in settings` in settings, `unchanged` in main |
| Close request | the settings window's close button | the close handler's Dialog opened; see the finding below |
| Keyboard close | Tab, Tab, Return in that Dialog | the window closed; one window left |
| Reopen | Open settings window in main | two windows |
| Window theme | Light in the settings window | settings background `rgb(243,241,235)`, main stays `rgb(21,20,18)` |
| Table context menu | Gallery Table page: right press on `api-01`; click `api-02`, then Shift+F10 | `api-01` selected and the menu opened at the pointer; the menu opened under the `api-02` row |

## Finding, fixed

The Dialog opened from the close request showed its 1px hairline instead of
the 2px focus frame, although its panel held focus: Tab went to Cancel and
Shift+Tab back to the panel, which then showed the frame. The panel takes
focus while its first frame is laid out, and GPUI ignores the refresh that
`focus()` asks for during a draw, so nothing redrew. `3b30144` asks for the
next frame after the one that focused the panel; the re-run showed the frame
on open, and `panel_focus_frame.rs` now opens a Dialog by a click and lets only
the window's own requests draw (it fails without the fix).

## Finding, fixed after the candidate

Reported by the maintainer: in the Gallery a PanZoom drag jumped back to where
it started on release and stayed there until another input redrew the window
(moving the pointer out of the area). Rotatable did the same; Draggable and
SplitPane did not. On release the primitive put the old transform back and
proposed the new one; the new source reached the signals while the next frame
was drawn, after the content had read them, and GPUI drops the redraw a write
asks for during a draw. Both now keep the proposed value until the Host answers
and ask for a frame after a draw-time write. Re-checked on real windows with
`pzdrag.swift`, `rotdrag.swift`, `dragdrag.swift` and `spdrag.swift`: right
after release, with no further input, each shows the committed position, and
nothing moves when the pointer then leaves the area (the screenshots are equal,
except SplitPane's separator line, which loses its hover color).

## Observation, open

An Input keeps its selection highlighted in the accent tint after focus moves to
another field (`eu-west` above). Not changed in this candidate.

## For the maintainer

IME preedit (candidate window, commit, cancel inside fields and the palette)
and VoiceOver are not covered here.
