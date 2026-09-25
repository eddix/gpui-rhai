# Documentation

Start with the [User Guide](../USER_GUIDE.md). It is the end-to-end contract for
application authors and coding agents: project setup, Rhai lifecycle, component
reuse, callback context, event geometry, Rust bridges, embedding, performance,
security, testing, and troubleshooting.

## Build an application

- [简体中文快速开始](quick-start.zh-CN.md)
- [Embedding script views](embedding.md) and [multi-window applications](multi-window.md)
- [Hot reload and production embedding](hot-reload-and-production.md)
- [Actions and keybindings](actions-and-keybindings.md)
- [Capabilities](capabilities.md) and [custom Rust primitives](custom-primitives.md)
- [Retained automation and JSON-lines protocol](automation.md)
- [Security boundary](security-boundary.md)

## Build the interface

- [Component authoring](component-authoring-guide.md)
- [Registry design system](registry-design-system.md)
- [Typed Style surface](style.md) and [component stylesheets](component-styles.md)
- [Theming](theming.md), [bundled themes](bundled-themes.md), and [Theme Studio](theme-studio.md)
- [Locale and RTL](locale-and-rtl.md)
- [Assets and fonts](assets.md)
- [Retained Canvas scenes](canvas.md)
- [Motion Runtime 2](motion.md)
- [Chart Runtime architecture decision](adr/0021-chart-runtime.md)
- [Native Chart Runtime](charts.md)
- [Native virtual collections](virtual-list.md) and [Rust-owned collection data](native-collections.md)
- [Native text documents and read-only viewers](document-viewers.md)
- [Accessibility status](accessibility.md)
- [Official component catalog](components/catalog.md)

Complex component contracts:

- [Tabs](components/catalog.md#tabs) and [Button/Badge density](registry-design-system.md#button-and-badge-density)
- [DatePicker](components/date-picker.md)
- [Combobox](components/combobox.md)
- [Select](components/select.md)
- [Textarea](components/textarea.md)
- [Table](components/table.md)
- [Pagination](components/pagination.md)
- [Toast](components/toast.md)
- [Command and CommandDialog](components/catalog.md#command-and-commanddialog)
- [CodeViewer](components/code-viewer.md) and [DiffViewer](components/diff-viewer.md)
- [Slider and ScrollArea](components/catalog.md#native-interaction-foundations)

## Develop the framework

- [Architecture](architecture.md)
- [gpui-pre 升级实施计划（待实施）](plans/2026-09-25-gpui-pre-upgrade.zh-CN.md)
- [Performance budgets and baselines](performance.md)
- [Development inspector](devtools.md)
- [macOS visual and interaction test matrix](visual-testing.md)
- [Updating copied source](source-updates.md)
- [Rhai execution backends](rhai-execution-backends.md)
- [Release checklist](release-checklist.md)
- [Release notes and upgrade index](releases/README.md)
- [0.1.0](releases/0.1.0.md), [0.1.1](releases/0.1.1.md),
  [0.1.2](releases/0.1.2.md), [0.1.3](releases/0.1.3.md),
  [0.1.4](releases/0.1.4.md), and [0.1.5](releases/0.1.5.md)
- [Core Runtime v2 evidence ledger](core-runtime-v2-audit.md)

Architecture decisions and their test evidence are recorded under
[`docs/adr`](adr/). `INTENT.md` is the product contract;
`IMPLEMENTATION_PLAN.md` records planned and incomplete work rather than the
current user-facing API.

Maintain current visual rules in `registry-design-system.md`, public component
contracts in `components/`, and acceptance procedures in `visual-testing.md`.
After an iteration lands, merge durable requirements into these documents and
remove the temporary implementation brief; preserve unfinished work as an
explicit gap. Third-party reference screenshots are not checked-in component
specification assets. Product-generated visual test evidence follows the
separate baseline process in `visual-testing.md`.
