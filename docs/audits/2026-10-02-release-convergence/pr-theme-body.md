## D3: lightweight resolved theme and native startup environment

Stacked on #105 (and verified #104 core); preserves #99's original author
commit, then corrects the projection and startup boundary. No merge, publish
or tag approval is implied.

- Rhai theme_variant returns exactly family/name/mode or unit. Rust receives
  lightweight ThemeVariantInfo and explicitly resolved_theme_selection, not a
  preference. One borrowed resolver also serves motion without copying colors,
  typography or the complete token snapshot.
- Render reads track theme dependencies. Effects restart only via explicitly
  declared deps, not a read buried inside an effect body.
- Primary and secondary lifecycle init/effects/render start after actual native
  appearance is registered. Secondary startup failures clean their exact mount,
  queued work and temporary window; same-ID retry remains valid.
- First real Light appearance invalidates existing Dark-fallback readers;
  unchanged appearance does not repeat invalidation. Headless System resolution
  explicitly defaults to Dark until actual native information exists.

Validation: five public core fixtures, six actual native fixtures, private
appearance-ingestion/invalidation regression and strict checks. Initial native
SystemLight is verified, including init/effect mode and no init replay. Fixed,
Host/Rhai, window/local/sibling, no-theme, token override and effect-deps controls
pass. TestPlatform's appearance setter is a no-op, so true native SystemDark /
system change stays in final real-machine acceptance; it is not counted green.

Original review/probes remain untouched. The final combined RC must rerun its
whole gate and manual matrix. No theme permission/storage system was added.
