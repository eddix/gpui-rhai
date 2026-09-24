# Chart Runtime contract remediation

The original report remains unchanged evidence for baseline `e09b5b2f`. This
note records the implementation response on the same feature branch.

| Finding | Remediation |
|---|---|
| C01 | Compile one domain/scale per `(region, axis)`; category values use typed display identity, log axes exclude an implicit zero, and rejected Geo projection points become bounded diagnostics. |
| C02 | Build signed stack intervals before domain calculation; positive and negative totals are independent and stack groups receive distinct band slots. |
| C03 | Preserve column schema through selection/empty sets, validate encode after semantic transforms, keep typed aggregate groups, and separate semantic data from null-aware draw sampling. |
| C04 | Gauge consumes the radial-axis domain; Radar aligns stable indicator names and one cross-series domain. |
| C05 | Use committed Host viewport plus transient preview/acknowledgement; Cartesian viewport changes regenerate domains, scales, marks and ticks together. |
| C06 | Add `ChartMarkRole`, `ChartDatumRef`, region-scoped unique mark identity and spec-stable visual assignment. |
| C07 | Control roles win gesture arbitration; wheel proposals commit once per gesture; brush/select payloads carry structural datum identity and revision; linked viewport synchronization uses data-domain windows rather than pixels. |
| C08 | PNG reuses the system-font SVG environment; special point symbols are resolved into scene geometry; native/SVG clip data by plot region and use the same fill/stroke geometry. |
| C09 | Native primitives can contribute bounded semantics to the retained accessibility/automation tree. Chart focus is identity-based and active/selected semantic data survives draw sampling. |
| C10 | Cache parsed inline sources before conversion, ignore origin-only layout changes, run transform and geometry candidates in background jobs, reject stale candidates, enforce mark/vertex budgets, and make the benchmark wait for the target presented revision. |
| C11 | All extension registries use vacant-entry insertion, custom Polar dispatch no longer inherits Value requirements, and custom renderers receive typed Cartesian/Polar/Geo coordinate context plus durable options. |
| C12 | Full-spec adapters mutate their stored series entries, formal adapters make shorthand-only props optional, and axis title, color encode, shared tooltip, tooltip theme tokens, compact locale formatting, funnel order, and small viewports have observable behavior. |

Permanent regression coverage:

- `crates/gpui-rhai/tests/chart_contract.rs`: numerical, schema, identity,
  extension, Geo and export contracts.
- `tests/native-keyboard/tests/charts.rs`: the five original real-GPUI
  behavioral failures through official Rhai component entry points.
- `tests/performance/tests/e2e.rs`: `gpui-rhai-chart-e2e-v2`, including an
  assertion that the requested `NativeChartData` revision is presented.

The real VoiceOver pass, full theme/RTL screenshot matrix, and 30-sample release
performance run remain release-checklist activities; they are not substituted
by unit snapshots or the three-sample reference baseline.
