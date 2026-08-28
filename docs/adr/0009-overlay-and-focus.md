# ADR 0009: Shared overlay and focus hierarchy

Status: Accepted

All Popover, Dropdown, Menu, Dialog, Tooltip, and Toast implementations will use
one per-window Rust OverlayManager. It owns portal layers, placement, dismissal,
modal focus, restoration, and queue regions; Rhai owns content and policy.

This mechanism is scheduled for M1 and must not be reimplemented independently
inside component scripts.
