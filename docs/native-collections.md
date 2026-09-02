# Rust-owned collections

`NativeCollection` is the data-plane path for large Table and
`virtual_collection` inputs. Rhai retains control of component composition,
columns, themes, controlled sort/selection state, and semantic callbacks; Rust
owns the complete keyed row set, cached sort order, and lazy visible-row
projection.

This is deliberately not a native Table widget. The official Table remains
copied Rhai source built from public Text/Box/`virtual_collection` mechanisms.
It accepts either its original Rhai Array or a `NativeCollection`.

## Register host data

Construct rows as schema-safe `UiValue` maps. Every row must contain one unique,
non-empty scalar key in the declared key field:

```rust
use std::collections::BTreeMap;
use gpui_rhai::{
    NativeCollection, ScriptViewExtension, UiRuntimeState, UiValue,
};

struct AppData {
    accounts: NativeCollection,
}

impl ScriptViewExtension for AppData {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime.native_collections
            .register("accounts", self.accounts.clone())
            .map_err(|error| error.to_string())
    }
}

let accounts = NativeCollection::new("id", [
    BTreeMap::from([
        ("id".into(), UiValue::String("acct-1".into())),
        ("name".into(), UiValue::String("Ada".into())),
    ]),
])?;
```

Attach the extension before `prepare`. A render reads and subscribes through
`UiContext`:

```rhai
table::Table(#{
    key: "accounts",
    label: "Accounts",
    row_key: "id",
    rows: ctx.get_native_collection("accounts"),
    columns: columns,
    fill_height: true,
    sort: ctx.get_state("sort"),
    selected_keys: ctx.get_state("selected"),
    on_sort_change: Fn("set_sort"),
    on_selection_change: Fn("set_selection"),
})
```

Rhai can read `len` and `key_field`, but cannot index or enumerate the
collection. This prevents an innocent component loop from materializing every
row as `Dynamic`. The native Table path validates its projection configuration,
caches scalar-field sort orders in Rust, and constructs normalized cell payloads
only for indices requested by GPUI.

## Replace live data

Prepare a new immutable collection in Rust, then replace it on the GPUI
foreground thread:

```rust
view.replace_native_collection("accounts", next_accounts, cx)?;
```

Only components that called `ctx.get_native_collection("accounts")` are marked
dirty. Runtime transactions snapshot the cheap collection handles and reader
edges; they never clone all rows. Several mounted views may subscribe to the
same runtime collection and will invalidate independently through their normal
frame pumps.

## Behavioral boundary

- With Array rows, Table preserves the existing source-component behavior: the
  caller owns row ordering and Table normalizes the complete Array in Rhai.
- With `NativeCollection`, Table applies its controlled scalar-field sort in
  Rust and performs selection/striping/cell projection only for visible rows.
- Domain formatting should still happen before the Table boundary. Compound
  values are not implicitly serialized into cell strings.
- `NativeCollection` contains typed `UiValue`, not `Dynamic`, `FnPtr`, Engine,
  GPUI entities, or window contexts. It can therefore be prepared by trusted
  Rust code without moving the foreground Rhai runtime.

Use the Array path for small or highly custom script-owned data. Use
`NativeCollection` when a repeated O(n) Rhai pass would dominate interaction
latency.
