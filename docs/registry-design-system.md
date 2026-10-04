# Official registry visual system

Since 0.2.0 the visual system of the official registry is specified in
[docs/design/](design/). This page used to hold the 0.1.x contract (an
Omarchy-informed compact language with a 13px body); that contract is retired
and the documents below replace it.

| Document | Content |
|---|---|
| [design/principles.md](design/principles.md) | character, layers, density and size, spacing, typography, color, focus |
| [design/atoms.md](design/atoms.md) | component contracts (L1): geometry, markers, lists, fields, overlays, focus mechanics |
| [design/composition.md](design/composition.md) | combining components into screens; the audit rules |
| [design/themes.md](design/themes.md) | palettes, derived tokens and theme authoring |
| [design/gallery-wireframes.md](design/gallery-wireframes.md) | the Gallery as acceptance application and the L2 APIs |
| [design/decisions.md](design/decisions.md) | the dated decision log |

The [Gallery](gallery.md) shows every component in both densities and is the
acceptance test of these documents; [visual testing](visual-testing.md) defines
the baseline matrix.
