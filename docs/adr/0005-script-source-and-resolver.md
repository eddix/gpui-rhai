# ADR 0005: ScriptSource and restricted resolver

Status: Accepted

File and embedded builds implement one `ScriptSource` contract and resolve only
validated logical module IDs. Absolute paths, traversal, platform path syntax,
dynamic imports, and cycles are rejected.

Production embedding uses generated `include_str!`/`include_bytes!` resources
but preserves the same IDs and resolution behavior as development files.
