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
- [Typed Style surface](style.md)
- [Theming](theming.md), [bundled themes](bundled-themes.md), and [Theme Studio](theme-studio.md)
- [Locale and RTL](locale-and-rtl.md)
- [Assets and fonts](assets.md)
- [Retained Canvas scenes](canvas.md)
- [Native virtual collections](virtual-list.md) and [Rust-owned collection data](native-collections.md)
- [Accessibility status](accessibility.md)
- [Official component catalog](components/catalog.md)

Complex component contracts:

- [DatePicker](components/date-picker.md)
- [Combobox](components/combobox.md)
- [Select](components/select.md)
- [Textarea](components/textarea.md)
- [Table](components/table.md)
- [Pagination](components/pagination.md)
- [Toast](components/toast.md)
- [Command and CommandDialog](components/catalog.md#command-and-commanddialog)
- [Slider and ScrollArea](components/catalog.md#native-interaction-foundations)

## Develop the framework

- [Architecture](architecture.md)
- [Performance budgets and baselines](performance.md)
- [Development inspector](devtools.md)
- [macOS visual and interaction test matrix](visual-testing.md)
- [Updating copied source](source-updates.md)
- [Rhai execution backends](rhai-execution-backends.md)
- [Release checklist](release-checklist.md)
- [Core Runtime v2 evidence ledger](core-runtime-v2-audit.md)

Architecture decisions and their test evidence are recorded under
[`docs/adr`](adr/). `INTENT.md` is the product contract;
`IMPLEMENTATION_PLAN.md` records planned and incomplete work rather than the
current user-facing API.
