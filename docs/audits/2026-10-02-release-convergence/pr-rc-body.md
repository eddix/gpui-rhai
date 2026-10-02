## Integrated 0.1.8 candidate — do not merge/publish/tag

Frozen product source: 53a62e3f2be05e5ae7a961936648195bc4fe9477.
Test/documentation source: be7a8eb83f5c97b9932661d74e5c3b7c684be4ce;
later evidence archive commits do not change the production trees.
Integrates the independently reviewed #104 core, #105 Host execution policy,
#106 key grammar/phases, #107 resolved-theme startup and the bounded #98 bridge.
Original #96/#97/#99 author commits and source mapping remain traceable.

Gallery now contains interactive wide Table and loading/empty/projection cases,
with LTR/RTL locale and resolved metadata. Source/registry metadata agree on
Table0.1.8. Release notes describe capabilities, Rust/key breaking changes and
explicit deferred issues, rather than claiming every green historical gate is
current acceptance.

Final product checks passed: workspace656/default473, native208, MSRV1.95 and
stable1.99, strict Clippy/rustdoc, core+registry package verification, release
launch/artifact gates and three release performance probes. CLI's no-verify
candidate boundary remains explicit; empty/nonempty generated consumers compile
with -Dwarnings and prepare using the exact local SDK. Source53 Linux CI passed,
including X11/Wayland. Final archive/test head CI is also checked before handoff.

Real idle MacPlatform Light/Dark revealed a missing notification ingress after
the earlier non-idle tests passed. The common weak Window subscription now
ingests environment before deferred normal work, preserves suspension and
cancels on disposal. Independent idle, multiView/fixed, resume/remount and
token-only native repaint probes passed; old failures remain in evidence.

Mandatory outstanding manual gates are not green by inference: native
OS IME/VoiceOver/focus/clipboard as applicable, physical120Hz and five Table
screenshots. Actual app-only native appearance tests do not alter OS preferences
or certify these manual gates. Temporary floating permission remains unconfirmed;
the old candidates are not new baselines. disktree-rhai stays excluded.

This is a draft candidate for review, not an approval to merge. See the complete
plan, contract index, dispositions and convergence report/verification under docs.
