# Temporal controls

Time is a core service, available without `charts`. `WallClock` is independent
from the monotonic animation/timer clock. Inject `WallClock::fixed(epoch_ms)`
or a `WallClockSource` with `FileScriptView::wall_clock` or
`EmbeddedScriptView::wall_clock` before preparation. `ctx.now()` returns epoch
milliseconds. By default `ctx.today()` derives the local date from this same
wall clock. An explicitly injected `CalendarClock::fixed/from_source` overrides
only legacy date-only `today()` behavior. Temporal range controls always derive
their calendar's today from the wall instant in their presentation zone.

The common timezone grammar is `local`, `UTC`, `offset:<minutes>` (east of UTC
is positive), or an available IANA name such as `America/New_York`. Jiff owns
instant/civil conversion and DST rules. `time_parts(ms, zone)` returns
`#{date, time, offset_minutes, zone}` with ISO date and 24-hour seconds text.
`time_local_candidates(date, time, zone)` returns zero candidates for a gap,
one for ordinary time, and two ordered epoch instants for a fold. No API
silently chooses one repeated time or normalizes a nonexistent local time.

`ctx.format_time(ms, style, zone)` accepts `short`, `medium`, or `long` (date,
seconds, UTC offset). `ctx.format_duration(ms)` uses compact localized units
and digits. `ctx.format_relative(ms)` uses the injected now. These are
presentation formatters, not a full pluralized CLDR prose formatter.
`ctx.time()` returns detached time metadata; `ctx.parse_time(text, precision)`
returns canonical `HH:MM:SS` or `()` for incomplete/invalid localized input.

## Controlled ranges

```rhai
TimeRangePicker(#{key:"query",label:"Time range",
    value:#{kind:"relative",duration_ms:86400000},time_zone:"UTC",
    on_change:Fn("range_changed")})
```

Relative values remain durations. At query time the application calls
`time_range_resolve(value, now_ms)` and receives `#{start,end}`. Absolute values
are `#{kind:"absolute",start:epoch_ms,end:epoch_ms}`. Resolution uses checked
integer arithmetic; it never fetches, polls, clamps to now or shifts a stored
absolute range. `time_range_error(value,min,max,max_span_ms)` returns an empty
string when valid or an explanation; optional bounds use `()`.

Preset activation applies a relative value immediately. The custom calendar,
date fields, time fields and optional local/UTC switch edit only the draft.
Apply emits a complete valid absolute value. Escape/outside dismissal discards
the draft, and reopening begins from the current controlled value. End must be
after start and within explicit epoch bounds and span cap. Future instants
are valid unless `max` forbids them. The shift buttons, when requested, emit an
absolute snapshot shifted by its own duration.

Time fields use the locale's hour cycle, day periods and digits, and Up/Down
step a minute or second. Gaps show an inline error. Repeated local times show
two explicit instant buttons, distinguished by UTC offset; Apply stays
disabled until each ambiguous endpoint has a selected candidate. Changing its
date/time clears the endpoint's old choice.

## Shared Calendar and TimeInput

`Calendar` is the public engine used by DatePicker and TimeRangePicker. Its
controlled `value` and optional `end` are strict ISO dates, `mode` is `single`
or `range`, and `months` is one or two. `on_change` proposes one activated date;
the caller owns how it becomes a range. `min_date`/`max_date` are inclusive.
Arrow keys use logical horizontal direction and locale week boundaries;
Home/End and Page Up/Down share the same bounds policy. Range interiors are
selected for accessibility, and endpoints additionally announce start/end.
Hover previews an unfinished range. An optional ISO `today` supplies the
presentation-zone highlight. DatePicker retains its existing ISO value and
change contract and forwards its calendar part styles into this shared grid.

`TimeInput` wraps the native editor and keeps its controlled value as localized
text, including partial drafts. It emits raw/stepped text and deliberately
does not invent a date, timezone or instant. Parse with `ctx.parse_time`.

## Timeline

`Timeline` accepts nested records with unique keys, title, status, optional
start/end epoch milliseconds, current, description, meta and children. Status
is pending/running/success/warning/failed/skipped/canceled. `details` is a
separate map of item-key to Node, so arbitrary nested detail content crosses
the structural component boundary rather than being serialized as business
data. Expansion and single selection are controlled arrays; duplicate titles
are harmless. `order` preserves or reverses sibling input order. Callers own
chronological sorting and any business-specific stage order.

Keys are globally unique across the entire nested input, including folded
groups. Supplied detail Nodes retain caller ownership and are eagerly
constructed by that caller. Virtualization limits realized row presentation;
it does not make constructing thousands of supplied detail Nodes lazy. Keep
large business data as plain records and construct rich detail only where the
application needs it.

Rows are measured/virtualized for long descriptions and histories. The current
step is independent of selection, announces `current:step`, and is initially
revealed when present in the visible projection. Selectable timelines are one
tab stop: Up/Down/Home/End navigate, Enter activates and Left/Right folds or
opens groups. List/listitem position and level metadata remain available to
assistive technology. Relative accessible labels also include absolute time.

Missing endpoints display an em dash. Running durations use now; a skewed end
or future relative timestamp falls back to absolute presentation. Running lamps
use native motion and settle under reduced-motion policy. Relative/duration
labels use a component-owned minute timer; pass `visible:false` when retaining
a hidden timeline. This is a visual clock, never automatic data refresh.

The Gallery module `gallery/pages/temporal` contains interactive Calendar,
TimeInput, Timeline and TimeRangePicker pages, including a virtualized history
and a New York DST example.
