# ADR 0009: Shared overlay and focus hierarchy

Status: Accepted

All Popover, Combobox, Menu, Dialog, and Tooltip implementations use one generic
Rust OverlayManager per `ScriptViewHost` interaction domain. Toast uses the
same Host's generic Layer portal plus runtime timers. The normal mapping
is one Host per GPUI window, while an advanced host may create explicit isolated
domains. It owns portal layers, placement, dismissal, modal focus, and
restoration; Rhai owns content, item limits, regions, and policy.

Several isolated ScriptViews may share that Host. Their local IDs are
namespaced by `view_id`; automatically measured content bounds drive responsive
layout, while an independent absolute Host viewport drives overlays. The Host
root element initializes the coordinator exactly once per GPUI frame and owns
capture-phase outside-click routing.

An open modal enforces containment during each focus-driven frame. This covers
initially-open content and embedding Hosts that programmatically focus an
ancestor after mount; Escape and Tab therefore remain routed through the modal.
The check is frame-based because GPUI 0.2.x focus-out listeners do not expose the
old path for every programmatic focus transfer. Closing disables containment
before the coordinator restores the prior focus.

Official components consume these public mechanisms and receive no private
placement, queue, or timer privileges.
