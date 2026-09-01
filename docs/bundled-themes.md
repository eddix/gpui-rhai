# Bundled themes

`gpui-rhai init` installs every bundled variant as editable Rhai source. The
primary `ui/theme.rhai` starts as Default Dark; the remaining variants live in
`ui/themes/` and are embedded only as ordinary project-owned source files.

## Project themes

- Default Light and Default Dark
- Tokyo Night and Tokyo Storm
- Catppuccin Latte and Catppuccin Mocha

## Omarchy adaptations

The following palettes are semantic adaptations of the corresponding themes in
the MIT-licensed [Omarchy repository](https://github.com/basecamp/omarchy/tree/master/themes):

- Ethereal
- Everforest
- Gruvbox
- Hackerman
- Nord
- Retro 82

The Omarchy source roles are translated into gpui-rhai's component roles rather
than copied mechanically. In particular, error, warning, success, focus, and
their foreground pairs remain readable and keep their UI meaning.

## Community adaptations

- Hermarchy is adapted from
  [Archer Clawbot's Hermarchy](https://github.com/archer-clawbot/omarchy-hermarchy-theme),
  licensed MIT.
- Aetheria is adapted from
  [Dizziee's Aetheria](https://github.com/JJDizz1L/aetheria), licensed MIT.
- Futurism is an original gpui-rhai semantic palette inspired by
  [Bjarne Øverli's Futurism](https://github.com/bjarneo/omarchy-futurism-theme).
  No source code or artwork from that repository is redistributed.

Each corresponding `.rhai` file repeats the short attribution at its top so it
survives copying into an application. Theme identity and runtime tokens remain
free of provenance metadata.

## Quality contract

Bundled variants use the shared `2 / 4 / 6px` radius scale and pass automated
contrast checks for primary/muted text, filled semantic states, and focus. The
Theme Studio renders the canonical component specimen under the active variant;
product examples remain contextual checks rather than the component catalog.
