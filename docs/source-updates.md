# Updating copied source

Installed components belong to the application. The CLI records the exact
upstream source under `.gpui-rhai/baselines` and its version/hash in the local
manifest.

```text
gpui-rhai diff
gpui-rhai update --dry-run
gpui-rhai update
gpui-rhai check
```

`diff` reports local changes relative to the installed baseline. `update`
performs an offline three-way merge of baseline, application source, and bundled
registry source. Clean updates replace source and baseline atomically;
independent edits merge line-wise.

The token base (`ui/tokens.rhai`) and the bundled palettes (`ui/themes/`) are
registry copies too: `init` records their baselines and `update` merges them
the same way, so a component that needs a new token gets it with the update.
`ui/theme.rhai` is the application's own theme and is never updated.

Projects created by 0.2.0 to 0.2.2 have no baselines for these files. `update`
records one for a copy equal to the bundled source, replaces a copy that is
still byte-for-byte what one of those releases installed, and leaves any other
copy alone with a `note:` line; merge that one by hand, or replace it and let
`update` record its baseline.

The CLI stages every planned file before replacing any target. An ordinary I/O
error during commit restores targets already replaced in that invocation.
Process termination can still leave uniquely named staging/backup artifacts;
the CLI does not claim filesystem-wide crash transactions. It also leaves the
project apply lock in place so a later invocation fails closed until the
artifacts are inspected and the lock is deliberately removed.

Conflicts never overwrite application source. The CLI writes conflict artifacts
under `.gpui-rhai/conflicts/` and exits unsuccessfully. Resolve the application
file deliberately, rerun `check`, and update only after reviewing component
metadata, dependency, schema, and runtime API changes. Commit application source,
the local manifest, and baselines together for reproducible reviews.
