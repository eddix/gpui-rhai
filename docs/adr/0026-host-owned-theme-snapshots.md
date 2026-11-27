# ADR 0026: Theme snapshots for Host-owned trees

Status: Accepted. Extends ADR 0012 without introducing a Host runtime.

## Context

`StaticUiView` accepted retained Rust-built trees and direct Host callbacks but
always rendered with `LiteralColorResolver`. A Host could express symbolic
colors, lengths and typography, yet they could not resolve through that view.
Adding a script view solely to select the design would violate the Host-owned
UI boundary.

## Decision

An optional resolved `ThemeVariant` supplies the existing `ColorResolver` to
the existing retained renderer. `with_theme` validates an initial snapshot;
`set_theme` validates a replacement before mutating it, notifies only on an
actual change, and does not reconcile the tree. `None` selects the existing
literal-only resolver. Theme replacement and `set_root` remain independent,
explicit Host operations, not an invented cross-operation transaction.

Theme identity, token vocabulary, inheritance and primitive snapshots follow
the same path as other native rendering. No parallel resolver, theme manager,
system-appearance subscription, script engine, scheduler or font loader is
added. Hosts choose and own the snapshot and install application font resources.
Parsing a theme source once is optional startup configuration, not scripted UI.

## Verification

Native tests exercise all four bundled designs through real layout and paint,
invalid and equal replacements, literal-only restoration, and input focus,
selection and undo across snapshot switches. These checks run separately on
the two published backend pins. They do not certify backend-dependent effects
or OS accessibility/input behavior.
