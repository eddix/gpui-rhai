# Community skins

This directory collects skins shipped with the repo. A skin is a plain `ui/`
directory: `app.toml`, `main.rhai`, and whatever modules, themes, and fonts it
wants to bring along. Install one by copying or symlinking it under
`$XDG_CONFIG_HOME/disktree-rhai/skins/`, then run `disktree-rhai --skin <name>`.

Planned:

- `minimal/` — the default skin minus the sidebar and controls: keyboard-only
  mosaic, the structural-reshaping demo (M4).
- `radial/` — a DaisyDisk-style radial layout, replacing the squarified
  geometry with a polar one via the same `core.rhai` contract (fast-follow).
