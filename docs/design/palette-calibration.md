# Bundled palette calibration

The source palettes keep their identity. Calibration changes the relationships
between neutral layers and text, not every theme into the same grey skin.
`registry/themes/` is authoritative; application copies remain editable and the
CLI's three-way update must preserve their local changes.

## Method

- Preserve all 15 page-background and accent anchors, status fills and their
  paired foregrounds. Preserve each neutral ramp's cool/warm/chromatic character.
- Adjust tonal lightness in Oklab/OKLCH, retaining hue and chroma. Gamut mapping
  reduces chroma rather than clipping RGB channels and shifting the hue. The
  final rounded sRGB values, not floating-point candidates, are tested.
- Dark surfaces step upward by about 0.05 perceptual L per level. Light paper
  lifts only a small amount toward white; hover/selection step downward. Hover
  and selection differ by at least 0.04 L, so hue is not their only distinction.
- On page, raised, hover and selection surfaces, primary text targets 7:1 and
  secondary text at least 5.2:1, with at least 0.075 L between the text roles.
  This leaves margin above 4.5:1 rather than tuning every caption to its edge.
- Decorative borders reach 2:1 on page/well. Functional control outlines and
  scrollbar thumbs reach 3:1 on their tested adjacent surfaces. Focus keeps its
  original palette role and must also clear 3:1.
- Check actual foreground/fill pairs in both idle and hover states. A light
  pastel suitable as a dark-theme fill may need a different derived color when
  used as small syntax/link text.
- Follow color roles through the actual native renderer. Single-line Input and
  document search placeholders use `text_muted`, with an overrideable Input
  `placeholder` part / native `placeholder_style`; no fixed 55% opacity is
  applied. Textarea and Input share the same part-color resolver.

The perceptual method follows [Oklab's original definition](https://bottosson.github.io/posts/oklab/).
Contrast minima follow [WCAG text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)
and [meaningful non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html).
The 2:1 decorative-border target and stronger bundled reading targets are our
design choices, not additional WCAG requirements. Nor does WCAG require 3:1
between every pair of hover/idle fills or decorative surfaces.

## Theme-specific intent

| Variant | Preserved identity | Calibration emphasis |
|---|---|---|
| Default Dark | Warm graphite/paper, cobalt, white-on-blue action | Separate the field from the page; deepen primary hover rather than washing out white text. |
| Default Light | Warm paper, ink and cobalt | Stronger readable control edges without turning panel dividers into dark frames. |
| Tokyo Night | Blue-black canvas, blue accent and frost | Even navy surface steps; keep auxiliary text quieter than primary and make scrollbar thumbs visible. |
| Tokyo Storm | Slate-blue canvas and shared Tokyo accent | Remove the shallow first/large second surface step; give text margin on the raised layers. |
| Catppuccin Latte | Cool paper, blue accent, violet-grey ink | Darken readable ink roles while preserving their tint and distinct hierarchy. |
| Catppuccin Mocha | Mauve-grey canvas and pastel accents | Reduce large tonal jumps that previously forced secondary text up toward the main text. |
| Ethereal | Deep indigo, lavender and peach | Restrain strongly tinted raised layers; keep peach headings and lavender action identity. |
| Everforest | Green-grey canvas, warm parchment and teal | Preserve the forest neutrals while separating reading roles and control outlines. |
| Gruvbox | Warm charcoal, parchment and muted teal | Restore a useful primary/secondary gap; text previously occupied almost the same lightness. |
| Hackerman | Near-black navy, phosphor green and cold highlights | Restrain near-white glare while retaining the vivid accent; clarify the neutral ramp. |
| Nord | Arctic slate, frost blue and pale text | Improve muted text and functional outlines without replacing frost with saturated blue. |
| Retro 82 | Deep blue, amber and aged cream | Remove the large blue-to-teal hover jump; keep amber/cream character in the foreground. |
| Hermarchy | Near-black graphite, pale ink and cyan | Even the abrupt hover step and reduce the peak white while preserving cyan emphasis. |
| Futurism | Blue-black canvas, pink accent and cyan focus | Keep neon accents as accents; make surfaces a coherent navy ramp and soften the body white. |
| Aetheria | Purple-black canvas, cyan text and coral action | Preserve the unusual cyan text; deepen coral hover so its pale foreground stays readable. |

## Verification and visual review

```sh
cargo test -p gpui-rhai --test theme_readability --all-features -- --nocapture
cargo test --manifest-path tests/native-keyboard/Cargo.toml --test native theme::derived_color_contrast
cargo test -p gpui-rhai-cli --lib theme_studio
```

The first test prints the final per-theme contrast/lightness table and checks
reading surfaces, outline/focus/scrollbar roles, hover foregrounds and syntax.
Mounted tests check real controls, selected data rows and facet tags in every
palette. Theme Studio warns on the broader pair matrix for application drafts.

For a real macOS/Metal review of every palette, without changing committed
baselines or opening multiple persistent applications:

```sh
cargo run --release --manifest-path tests/native-keyboard/Cargo.toml \
  --bin gallery_baselines -- /tmp/palette-review --themes foundations.color
cargo run --release --manifest-path tests/native-keyboard/Cargo.toml \
  --bin gallery_baselines -- /tmp/control-review --themes scene.form
```

Inspect fields, secondary labels, selection and status on the same real Gallery
pages, not only swatch cards. Keep the actual source and capture parameters with
any proposed baseline refresh. Numerical checks cannot guarantee recognition on
an arbitrary uncalibrated display, nor replace keyboard/focus and colour-vision
review. Existing baseline images are reference artifacts, not proof of current
contrast; refresh them through the documented capture workflow when accepting a
new visual baseline.
