# ADR 0002: Foreground engine and callback generations

Status: Accepted

Rhai view/lifecycle/event evaluation is serialized on the GPUI foreground
thread. Every compiled candidate has a generation; it becomes current only
after `view` succeeds. Callbacks from any other generation are rejected.

This avoids concurrent scope semantics and prevents obsolete hot-reloaded ASTs
from handling events. Heavy work belongs in Rust tasks.

Protected by callback invalidation and failed-candidate tests.
