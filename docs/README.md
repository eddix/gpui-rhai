# Documentation

Start with the [User Guide](../USER_GUIDE.md). It is the end-to-end contract for
application authors and coding agents: project setup, Rhai lifecycle, component
reuse, callback context, event geometry, Rust bridges, embedding, performance,
security, testing, and troubleshooting. The [简体中文快速开始](quick-start.zh-CN.md)
is the shortest path to a running project.

## Design with the official components

- Design specification: [principles](design/principles.md) (character,
  layers, density, spacing, type, color, focus),
  [component contracts](design/atoms.md), [composition](design/composition.md)
  (layouts, patterns and the audit rules), [themes](design/themes.md), the
  [decision log](design/decisions.md) and the
  [Gallery wireframes](design/gallery-wireframes.md)
- [Official component catalog](components/catalog.md)
- [Gallery](gallery.md), the acceptance application, and
  [Theme Studio](theme-studio.md)
- [Component authoring](component-authoring-guide.md)

Complex component contracts: [Table](components/table.md),
[DatePicker](components/date-picker.md), [Combobox](components/combobox.md),
[Select](components/select.md), [Textarea](components/textarea.md),
[Pagination](components/pagination.md), [Toast](components/toast.md),
[CodeViewer](components/code-viewer.md), [DiffViewer](components/diff-viewer.md),
[Command and CommandDialog](components/catalog.md#command-and-commanddialog),
[Tabs](components/catalog.md#tabs), and
[direct manipulation and layout behaviors](components/interaction-behaviors.md)
(SplitPane, Resizable, drag and drop, PanZoom and more).

## Build the interface

- [Typed Style surface](style.md) and [component stylesheets](component-styles.md)
- [Theming](theming.md) and [bundled themes](design/themes.md#6-bundled-themes)
- [Locale and RTL](locale-and-rtl.md)
- [Assets and fonts](assets.md) and [refreshing Host-owned images](asset-refresh-host.md)
- [Retained Canvas scenes](canvas.md)
- [Motion](motion.md)
- [Native charts](charts.md)
- [Native virtual collections](virtual-list.md) and [Rust-owned collection data](native-collections.md)
- [Native text documents and read-only viewers](document-viewers.md)
- [Accessibility](accessibility.md)

## Host the views in Rust

- [Embedding script views](embedding.md) and [multi-window applications](multi-window.md)
- [Hot reload and production embedding](hot-reload-and-production.md)
- [Actions and keybindings](actions-and-keybindings.md)
- [Capabilities](capabilities.md) and [custom Rust primitives](custom-primitives.md)
- [Retained automation and the JSON-lines protocol](automation.md)
- [Security boundary](security-boundary.md)

## Develop the framework

- [Architecture](architecture.md) and the [architecture decisions](adr/)
- [Runtime contract tests](runtime-contract-tests.md)
- [Performance budgets and baselines](performance.md)
- [Development inspector](devtools.md)
- [macOS visual and interaction testing](visual-testing.md)
- [Updating copied source](source-updates.md)
- [Rhai execution backends](rhai-execution-backends.md)
- [Release checklist](release-checklist.md)

## Releases and history

- [Release notes and upgrade index](releases/README.md): [0.2.0](releases/0.2.0.md),
  [0.1.8](releases/0.1.8.md) and every earlier version
- [CHANGELOG](../CHANGELOG.md)
- [History](history.md): audits, plans and research of earlier rounds

## Maintaining these documents

`INTENT.md` is the product contract. Keep visual rules in [design/](design/),
public component contracts in [components/](components/), runtime decisions in
[adr/](adr/), and acceptance procedures in `visual-testing.md`. A working round
(an audit, a plan, an investigation) does not stay in the tree: merge what
lasts into these documents, record unfinished work as an explicit gap, and list
the round in [History](history.md). Third-party reference screenshots are not
specification assets; product-generated visual evidence follows the baseline
process in `visual-testing.md`.
