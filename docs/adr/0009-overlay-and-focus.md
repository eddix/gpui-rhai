# ADR 0009: Shared overlay and focus hierarchy

Status: Accepted

All Popover, Dropdown, Menu, Dialog, Tooltip, and Toast implementations use one
Rust OverlayManager per `ScriptViewHost` interaction domain. The normal mapping
is one Host per GPUI window, while an advanced host may create explicit isolated
domains. It owns portal layers, placement, dismissal, modal focus, restoration,
and queue regions; Rhai owns content and policy.

Several isolated ScriptViews may share that Host. Their local IDs are
namespaced by `view_id`; automatically measured content bounds drive responsive
layout, while an independent absolute Host viewport drives overlays. The Host
root element initializes the coordinator exactly once per GPUI frame and owns
capture-phase outside-click routing.

This mechanism is scheduled for M1 and must not be reimplemented independently
inside component scripts.
