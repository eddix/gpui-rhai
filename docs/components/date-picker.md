# DatePicker specification

DatePicker is a controlled, single-date Rhai source component over public date
data helpers, Box/Text/Image atoms, semantic events, formal component state,
effects, and generic Overlay. It presents a read-only Input-like trigger and an
anchored month panel. The public date ABI is strict Gregorian ISO `YYYY-MM-DD`;
locale affects presentation only.

## Public contract

`registry/components/date_picker.rhai` exports `DatePicker(props)` with:

- required `key: string`;
- `value: optional<string>`;
- optional `min_date` and `max_date`, both strict ISO dates;
- `placeholder: string` for an empty value;
- `display_style: "short" | "medium" | "long"`, default `"short"`;
- `clearable: bool`, default `false`;
- optional `presets: array<{ label, value, disabled? }>`;
- `disabled`, `error`, `size`, `width`, `placement`, and optional
  `parent_overlay`;
- optional `on_change`, whose payload is `optional<string>`.

An empty value is Rhai `()`. Empty strings are never date sentinels. Invalid
calendar dates, an inverted min/max interval, or a selected value outside the
interval are construction errors. A syntactically valid preset outside the
interval is displayed disabled rather than failing the whole component.

`DatePicker` declares style parts for at least `root`, `trigger`,
`trigger_value`, `trigger_icon`, `panel`, `header`, `previous`, `next`,
`weekdays`, `grid`, `day`, `day_outside`, `day_today`, `day_selected`,
`day_disabled`, `footer`, `preset`, and `clear`.

## Calendar and state behavior

- The panel is a fixed six-row by seven-column grid. Adjacent-month dates fill
  the grid, use a muted part, and remain selectable when inside min/max.
  At the absolute `0001-01`/`9999-12` ISO boundary, unavailable adjacent-year
  positions remain disabled blank cells so the 42-cell geometry stays stable.
- The runtime Clock supplies today's date in the host system's local time zone.
  Tests and deterministic examples inject a fixed Clock.
- On first open, the visible month is the selected date's month, or today's
  month when no value is selected. An external value change resynchronizes it.
- Uncommitted month navigation is transient. Closing and reopening resets the
  visible month from the controlled value or today.
- Selecting a date emits the ISO value, closes the panel, and restores focus to
  the trigger. Clearing emits `()` and follows the same close/focus behavior.
- Presets contain caller-owned labels and concrete ISO values. The component
  does not hard-code relative rules such as “tomorrow” or “next Monday”.

Open state, visible month, and focused date are declared keyed component state.
A named effect synchronizes those transients when the controlled/range-derived
anchor changes; it retains only UiValue dependencies and named callbacks.
Hover/focus visuals remain native GPUI interaction states. DatePicker does not
expose controlled `open` or `visible_month` props.

## Keyboard and accessibility

- Enter, Space, or ArrowDown opens the panel.
- Arrows move by one logical day or one week. Left/Right respect RTL.
- Home/End move to the beginning/end of the locale-defined week.
- PageUp/PageDown move by month; the modified platform convention may move by
  year when supported by the native key event.
- Enter or Space commits the focused enabled date.
- Escape closes without changing the value and restores trigger focus.
- Month navigation controls disable when their target month contains no date
  inside min/max.

The normalized tree retains combobox, dialog, calendar-grid, selected, current,
disabled, label, value, and invalid semantics even where pinned GPUI cannot yet
forward every value to AccessKit.

## Runtime and locale boundary

The Rhai component owns validation policy, state, keyboard mapping, selection,
layout, style parts, and asset IDs. Rust exposes pure checked Gregorian data
operations (`date_info`, checked day/month shifts, week edges, month grids,
range clamp/intersection) and owns generic focus/input and Overlay placement.
There is no DatePicker UiNodeKind, renderer branch, Entity, or private callback
adapter.

Month and weekday names, first weekday, and display patterns come exclusively
from the selected locale bundle. Generic Runtime APIs expose date formatting
and validated calendar metadata for DatePicker, Table, and application scripts.
No locale ID is special-cased in DatePicker.

## Exclusions

- editable or locale-dependent date text parsing;
- date ranges or a `mode: "range"` compatibility surface;
- time selection;
- non-Gregorian calendars.

A future DateRangePicker is a separate component that may reuse the same date,
locale, overlay, and grid mechanisms.

## Required evidence

- Gregorian arithmetic and invalid-date unit tests, including leap years;
- December/January navigation and min/max boundary tests;
- English, Simplified Chinese, and RTL locale contract tests;
- source/runtime keyboard, generic Overlay dismissal/focus, and controlled-value tests;
- fixed-Clock visual baselines across representative themes;
- a localized appointment-date scenario in `form_showcase`.
