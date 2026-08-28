# Locale and RTL

Locale bundles are application-owned Rhai source under `ui/locales`. Every
bundle declares an ID, logical direction, and the required internal phrases.
English, Simplified Chinese, and the Arabic RTL validation bundle ship in the
registry.

```rhai
fn locale() {
    #{
        locale: "ar",
        direction: "right_to_left",
        messages: #{ /* required keys */ },
    }
}
```

Use `ctx.t("message.key")` for internal phrases. Application content normally
arrives through component props. `ctx.set_locale(id)` changes the app selection;
`ctx.set_window_locale(id)` changes only the current window, and
`ctx.set_local_locale(id)` overrides the current component subtree.
`ctx.text_direction()` returns `"ltr"` or `"rtl"` when policy must differ
explicitly.

The renderer automatically reverses row ordering under RTL, maps logical
start/end alignment, resolves `padding_start/end` and `margin_start/end`, and
maps physical Left/Right keys to the correct logical handlers. Prefer those
logical spacing methods in reusable components.

Directional icons use explicit resource pairs because GPUI's ordinary image
element has no public horizontal-mirror API:

```rhai
icon::Icon(#{ handle: forward_ltr, rtl_handle: forward_rtl, label: "Forward" })
```

Do not mirror checks, logos, text, or other non-directional imagery. Mixed-bidi
text shaping is delegated to the platform text system; the pinned GPUI release
does not expose a per-element base-direction or bidi-isolation API.
