# Locale, formatting, and RTL

Locale bundles are application-owned Rhai source under `ui/locales`. Every
bundle declares an ID, logical direction, required internal phrases, Gregorian
calendar metadata, and decimal-number metadata. English, Simplified Chinese,
and the Arabic RTL validation bundle ship in the registry.

```rhai
fn locale() {
    #{
        locale: "en",
        direction: "left_to_right",
        messages: #{ /* required component labels */ },
        calendar: #{
            first_weekday: "sunday",
            months: #{ short: [/* 12 */], long: [/* 12 */] },
            weekdays: #{ short: [/* 7, Sunday first */], long: [/* 7 */] },
            date_patterns: #{
                month_year: "MMMM yyyy",
                short: "MM/dd/yyyy",
                medium: "MMM d, yyyy",
                long: "MMMM d, yyyy",
            },
        },
        number: #{
            digits: ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
            decimal_separator: ".",
            grouping_separator: ",",
            primary_group_size: 3,
            secondary_group_size: 3,
            minus_sign: "-",
        },
    }
}
```

Calendar arrays use stable Gregorian order regardless of first_weekday.
`month_year` controls the DatePicker header without locale-ID branches. The
loader validates exact lengths, non-empty unique display strings, weekday enum,
and supported date-pattern tokens. Initial patterns support numeric and named
year/month/day/weekday tokens needed by short, medium, and long presentation;
unknown ASCII pattern letters are errors rather than literals.

Number metadata is deliberately a decimal formatter contract, not an incomplete
currency or CLDR plural-rules implementation. Formatter options control grouping
and fraction digits. Component summary text uses neutral localized labels and
formatted numbers. Currency, percentages with business-specific rounding, and
pluralized prose belong to caller policy or a future complete formatter.

## Runtime API

- `ctx.t(key)` resolves required internal phrases.
- `ctx.format_date(iso_date, style)` accepts strict Gregorian `YYYY-MM-DD` and
  `short`, `medium`, or `long`.
- `ctx.format_month_year(iso_date)` applies the validated locale month/year
  pattern used by calendar headers.
- `ctx.format_number(value, options)` applies the selected bundle's digits,
  separators, grouping, sign, and fraction policy.
- read-only calendar metadata is exposed only through a validated value shape
  needed by source components; scripts cannot mutate the selected bundle.
- `ctx.number()` returns corresponding detached read-only number metadata.
- `ctx.set_locale`, `ctx.set_window_locale`, and `ctx.set_local_locale` retain
  app/window/subtree selection precedence.
- `ctx.text_direction()` returns `ltr` or `rtl` for explicit policy branches.

Pure global helpers keep calendar policy available to every Rhai author without
a DatePicker-specific native node: `date_info`, `date_month_start`,
`date_checked_add_days`, `date_checked_add_months`, `date_week_edge`,
`date_month_grid`, `date_clamp`, and `date_month_intersects`. They validate the
strict four-digit Gregorian range; checked shifts return `()` at the absolute
boundary, and month grids always contain 42 detached data maps.

Date values never change with locale. DatePicker values, date column data,
min/max constraints, and callbacks use ISO strings; only their visible text is
formatted. Today's date comes from the injected Runtime Clock in the host's
local time zone.

Locale switches increment locale generation, rerender affected views, and keep
component state, retained node identity, scroll state, and compiled ASTs.

## RTL

The renderer reverses row ordering under RTL, maps logical start/end alignment,
resolves `padding_start/end` and `margin_start/end`, and maps logical horizontal
navigation. Reusable components prefer logical alignment and spacing.

Directional icons provide explicit `source` and `rtl_source` resources because
GPUI exposes no safe public horizontal-mirror API. Do not mirror checks, logos,
text, or other non-directional imagery. DatePicker logical day navigation,
Table start/end alignment, Select disclosure, and Pagination arrows are part of
the RTL interaction matrix.

Mixed-bidi text shaping remains delegated to the platform text system; pinned
GPUI exposes no per-element base-direction or bidi-isolation API.
