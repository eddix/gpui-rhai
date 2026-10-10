# Script API

Generated from the native functions and primitives the runtime registers and
their documentation in `crates/gpui-rhai/src/script_docs/` and the primitive
descriptors; do not edit. Rhai's own standard library (strings, arrays, maps,
math) is in the [Rhai book](https://rhai.rs/book/); the language as views use it
is in [Rhai for gpui-rhai](../rhai.md). Components are in the
[module reference](README.md).

## Node constructors

Functions that build `UiNode` values.

| Call | Description |
|---|---|
| `box(children: Array) -> UiNode` | Creates a box node, the general container for layout, paint and interaction; every child must be a `UiNode`. |
| `canvas(scene: CanvasScene) -> UiNode` | Creates a canvas node that paints a retained `canvas_scene` in canvas-local logical pixels. |
| `column(children: Array) -> UiNode` | Creates a box that lays its children out top to bottom (`flex_col`); every child must be a `UiNode`. |
| `directional_image(left_to_right: AssetId, right_to_left: AssetId) -> UiNode` | Creates an image node that shows the first asset in left-to-right layout and the second in right-to-left layout. |
| `directional_image(left_to_right: OpaqueHandle, right_to_left: OpaqueHandle) -> UiNode` | Creates an image node that shows the first image handle in left-to-right layout and the second in right-to-left layout. |
| `directional_image_source(left_to_right: ?, right_to_left: ?) -> UiNode` | Creates an image node chosen by layout direction; each source is an `AssetId` or an image handle, and other values are rejected. |
| `error_boundary(child: UiNode, fallback: UiNode) -> UiNode` | Wraps `child` so that a native rendering failure in its subtree shows `fallback` instead. |
| `error_boundary_lazy(child: FnPtr, fallback: FnPtr) -> UiNode` | Builds `fallback()` then `child()` into an error boundary; when `child()` throws, returns the fallback with a `boundary_error` attribute. |
| `fragment(children: Array) -> UiNode` | Groups children without a layout box; a fragment carries only children, a key and its source, so style, handlers and refs are rejected. |
| `image(asset: AssetId) -> UiNode` | Creates an image node for a component-declared, preloaded asset; an undeclared asset renders an image error. |
| `image(handle: OpaqueHandle) -> UiNode` | Creates an image node for an image handle from `ctx.load_image` or a capability; a handle of another kind fails when rendered. |
| `image_source(source: ?) -> UiNode` | Creates an image node from `source`, an `AssetId` or an image handle; other values are rejected. |
| `layer(content: UiNode, config: Map) -> UiNode` | Places `content` in a window-level layer; `config` needs `id` and may set `placement` (default `top_right`), `inset` (12 px) and `priority`. |
| `motion_group(id: String, children: Array) -> UiNode` | Groups children in a layout-transparent fragment whose `shared_layout` nodes join group `id` (1 to 128 letters, digits, `_`, `-`). |
| `overlay(trigger: UiNode, content: UiNode, config: Map) -> UiNode` | Anchors `content` to `trigger` in a window-level overlay; `config` needs `id` and sets `kind`, `open`, `placement`, `align`, `gap` and dismissal. |
| `render_component(id: String, props: Map) -> UiNode` | Renders the formal component registered as `id` with `props`, validated and defaulted; only during view render. |
| `row(children: Array) -> UiNode` | Creates a box that lays its children out side by side (`flex_row`); every child must be a `UiNode`. |
| `stack(children: Array) -> UiNode` | Creates a relatively positioned box, so children can be placed with `style().absolute()` and edge offsets. |
| `svg(markup: String) -> UiNode` | Creates an inline SVG node from self-contained markup of at most 64 KiB and 2,048 elements; scripts and external references are rejected. |
| `text(spans: Array) -> UiNode` | Creates a rich text node from `span` values; no span may split a grapheme cluster. |
| `text(content: String) -> UiNode` | Creates a plain text node. |
| `virtual_collection(config: Map, renderer: FnPtr) -> UiNode` | Creates a virtual list of `config.data` rendering visible items via named `renderer(ctx, payload)`; needs `key`, `estimated_height`, `height` or `fill_height`. |

## Styles, lengths and colors

Values a style is built from; read lengths and colors from theme tokens.

| Call | Description |
|---|---|
| `alpha(color: ColorValue, factor: float) -> ColorValue` | Returns `color` with its alpha multiplied by `factor` (0 to 1); a token color resolves when the node renders. |
| `color(value: String) -> ColorValue` | Parses a color literal: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, a CSS basic name, `transparent`, or an `rgb()`, `hsl()` or `hwb()` form. |
| `mix(base: ColorValue, other: ColorValue, weight: float) -> ColorValue` | Mixes `base` toward `other` by `weight` (0 to 1) in every channel, alpha included; token colors resolve when the node renders. |
| `offset_px(value: float) -> SignedLength` | Creates a signed offset in logical pixels for margins and insets; any finite value, negative included. |
| `offset_px(value: int) -> SignedLength` | Creates a signed offset in logical pixels for margins and insets; negative values are allowed. |
| `offset_relative(fraction: float) -> SignedLength` | Creates a signed offset as a fraction of the parent's size, from -10 to 10, for margins and insets. |
| `offset_relative(fraction: int) -> SignedLength` | Creates a signed offset as a whole multiple of the parent's size, from -10 to 10, for margins and insets. |
| `offset_rem(value: float) -> SignedLength` | Creates a signed offset in rems for margins and insets; any finite value, negative included. |
| `offset_rem(value: int) -> SignedLength` | Creates a signed offset in rems for margins and insets; negative values are allowed. |
| `px(value: float) -> Length` | Creates a length in logical pixels; `value` must be finite and non-negative. |
| `px(value: int) -> Length` | Creates a length in logical pixels; `value` must be non-negative. |
| `readable(color: ColorValue, toward: ColorValue, background: ColorValue, ratio: float) -> ColorValue` | Moves `color` toward `toward` just far enough to reach WCAG contrast `ratio` (1 to 21) against `background`, when the node renders. |
| `relative(fraction: float) -> Length` | Creates a length as a fraction of the parent's size, from 0 to 1: `relative(0.5)` is half. |
| `relative(fraction: int) -> Length` | Creates a length as a fraction of the parent's size; as an integer only 0 or 1, and `relative(1)` fills the parent. |
| `rem(value: float) -> Length` | Creates a length in rems, multiples of the window's rem size; `value` must be finite and non-negative. |
| `rem(value: int) -> Length` | Creates a length in rems, multiples of the window's rem size; `value` must be non-negative. |
| `rgb(hex: int) -> ColorValue` | Creates an opaque color from `0xRRGGBB` (0 to 0xffffff). |
| `rgba(hex: int) -> ColorValue` | Creates a color from `0xRRGGBBAA`, alpha in the low byte (0 to 0xffffffff). |
| `style() -> Style` | Returns an empty `Style` to build with chained setters. |
| `theme_color(token: String) -> ColorValue` | References theme color `token`, such as `"accent"` or `"charts.series_a"`, resolved against the active theme when the node renders. |
| `theme_length(path: String) -> Length` | References length token `path` (`namespace.name`, such as `"metrics.row"`), resolved against theme and environment when rendered. |
| `theme_radius(name: String) -> Length` | References radius token `radius.<name>`, such as `theme_radius("md")`, resolved when the node renders. |
| `theme_spacing(name: String) -> Length` | References spacing token `spacing.<name>`, such as `theme_spacing("sm")`, resolved when the node renders. |
| `theme_typography(role: String) -> Style` | Returns a `Style` with typography `role`, like `style().typography(role)`; the theme decides at render whether the role exists. |
| `times(length: Length, factor: float) -> Length` | Scales `length` by a non-negative `factor`, like `length * factor`; a relative result must stay within 0 to 1. |
| `times(length: Length, factor: int) -> Length` | Scales `length` by a non-negative integer `factor`, like `length * factor`; a relative result must stay within 0 to 1. |

## Other global functions

Components, references, collections, timers, motion and canvas commands.

| Call | Description |
|---|---|
| `asset(id: String) -> AssetId` | Parses a logical asset ID of two or more `/`-separated segments of letters, digits, `_` or `-`, such as `"app/icons/check"`. |
| `auto() -> AutoLength` | Returns the `auto` layout length, accepted by the size, margin and inset setters: `style().width(auto())`. |
| `by_env(keys: ?, table: Map) -> EnvTableSource` | Varies a theme token with the environment: `keys` is an axis name or an array of names, and `table` maps their values, nested per axis. |
| `canvas_circle(key: String, center_x: float, center_y: float, radius: float, fill: ColorValue) -> CanvasCommand` | Creates a filled circle command centered at (`center_x`, `center_y`); `radius` must be positive and `key` unique in the scene. |
| `canvas_circle(key: String, center_x: ?, center_y: ?, radius: ?, fill: ColorValue) -> CanvasCommand` | Creates a filled circle command centered at (`center_x`, `center_y`); `radius` must be positive and `key` unique in the scene. Numbers may be integers or floats. |
| `canvas_fill_path(key: String, segments: Array, fill: ColorValue) -> CanvasCommand` | Creates a path command filled with a solid color; `segments` start with `path_move` and hold 2 to 10,000 segments. |
| `canvas_fill_path(key: String, segments: Array, gradient: LinearGradientSpec) -> CanvasCommand` | Creates a path command filled with a two-stop `linear_gradient`; `segments` start with `path_move` and hold 2 to 10,000 segments. |
| `canvas_line(key: String, from_x: float, from_y: float, to_x: float, to_y: float, width: float, color: ColorValue) -> CanvasCommand` | Creates a line command from (`from_x`, `from_y`) to (`to_x`, `to_y`), stroked `width` pixels wide (positive). |
| `canvas_line(key: String, from_x: ?, from_y: ?, to_x: ?, to_y: ?, width: ?, color: ColorValue) -> CanvasCommand` | Creates a line command from (`from_x`, `from_y`) to (`to_x`, `to_y`), stroked `width` pixels wide (positive). Numbers may be integers or floats. |
| `canvas_morph_stroke_path(key: String, from: Array, to: Array, width: float, color: ColorValue) -> CanvasCommand` | Creates a stroked path that morphs from `from` to `to` as its `path_progress` motion goes 0 to 1; both need the same segment kinds. |
| `canvas_morph_stroke_path(key: String, from: Array, to: Array, width: int, color: ColorValue) -> CanvasCommand` | Creates a stroked path that morphs from `from` to `to` as its `path_progress` motion goes 0 to 1; both need the same segment kinds. |
| `canvas_rect(key: String, x: float, y: float, width: float, height: float, fill: ColorValue) -> CanvasCommand` | Creates a filled rectangle command with its top-left corner at (`x`, `y`); `width` and `height` must be positive. |
| `canvas_rect(key: String, x: ?, y: ?, width: ?, height: ?, fill: ColorValue) -> CanvasCommand` | Creates a filled rectangle command with its top-left corner at (`x`, `y`); `width` and `height` must be positive. Numbers may be integers or floats. |
| `canvas_scene(commands: Array) -> CanvasScene` | Builds a canvas scene from canvas commands, painted in order; every command key must be unique. |
| `canvas_stroke_path(key: String, segments: Array, width: float, color: ColorValue) -> CanvasCommand` | Creates a path command stroked `width` pixels wide (positive); `segments` start with `path_move` and hold 2 to 10,000 segments. |
| `canvas_stroke_path(key: String, segments: Array, width: int, color: ColorValue) -> CanvasCommand` | Creates a path command stroked `width` pixels wide (positive); `segments` start with `path_move` and hold 2 to 10,000 segments. |
| `chart_validate(spec: Map, data: ?, key_dimension: ?)` | Checks that `spec` is a valid chart spec and `data` is `NativeChartData` or up to 10,000 row maps keyed by `key_dimension` or `()`. |
| `date_checked_add_days(date: String, days: int) -> ?` | Adds signed `days` to an ISO `YYYY-MM-DD` date and returns the new ISO date, or `()` when it leaves years 1 to 9999. |
| `date_checked_add_months(date: String, months: int) -> ?` | Adds signed `months` to an ISO date, clamping the day into the target month; returns `()` when it leaves years 1 to 9999. |
| `date_clamp(date: String, min: ?, max: ?) -> String` | Clamps an ISO date into `min` to `max`, each an ISO date or `()` for no bound, and returns the ISO date. |
| `date_info(date: String) -> Dynamic>` | Parses an ISO `YYYY-MM-DD` date into `#{ iso, year, month, day, weekday }`, `weekday` being a lowercase English name. |
| `date_month_grid(date: String, first_weekday: String) -> Dynamic>` | Returns the 42 cells (six weeks) of the month containing `date`, weeks starting on `first_weekday`; each is `#{ date, day, outside, weekday }`. |
| `date_month_intersects(date: String, min: ?, max: ?) -> bool` | Returns whether any day of the month containing `date` lies within `min` to `max`, each an ISO date or `()` for no bound. |
| `date_month_start(date: String) -> String` | Returns the ISO date of the first day of the month containing the ISO date `date`. |
| `date_week_edge(date: String, first_weekday: String, end: bool) -> ?` | Returns the ISO date that starts (or, when `end`, ends) the week containing `date`, weeks starting on `first_weekday`, or `()` out of range. |
| `define_component(definition: Map)` | Registers a formal component from `#{ metadata, schema, render }` at module top level; `render` must be a named, uncurried `Fn`. |
| `effect(key: String, dependencies: ?, start: FnPtr, cleanup: FnPtr)` | Declares effect `key` (listed in the schema) during formal render; `start(ctx, deps)` runs after commit, `cleanup` before a change or unmount. |
| `element_ref(key: String) -> ElementRef` | Declares a component-local element ref during formal render; attach it with `with_ref` and read it with `ctx.element_bounds`. |
| `event_response() -> EventResponse` | Returns a blank event response that lets the event continue, to refine with `prevent_default()`, `stop()` or `capture_pointer()`. |
| `grapheme_count(text: String) -> int` | Returns the number of extended grapheme clusters, the user-perceived characters, in `text`. |
| `handled() -> EventResponse` | Returns an event response that stops the event from reaching ancestor handlers. |
| `is_native_collection(value: ?) -> bool` | Returns whether `value` is a Host-owned `NativeCollection`. |
| `is_native_text_document(value: ?) -> bool` | Returns whether `value` is a Host-owned `NativeTextDocument`. |
| `key_shortcut(text: String) -> ?` | Formats `text` such as `"cmd-p"` as `#{ label, keystrokes, chords }` with platform key names; text that is not keystrokes stays a plain label. |
| `linear_gradient(spec: Map) -> LinearGradientSpec` | Creates a two-stop gradient from `#{ angle, from, to }`; `angle` is in degrees (default 180) and both colors are required. |
| `motion_delay(duration_ms: int) -> MotionTimelineStep` | Creates a timeline step that waits `duration_ms` milliseconds (non-negative). |
| `motion_focus(source: MotionSource) -> MotionProgressBinding` | Drives `source` by focus: forward while the node is focused, back after. Takes a transition or keyframes without delay or repeat. |
| `motion_hover(source: MotionSource) -> MotionProgressBinding` | Drives `source` by hover: forward while the pointer is over the node, back after. Takes a transition or keyframes without delay or repeat. |
| `motion_in_view(source: MotionSource) -> MotionProgressBinding` | Maps `source` to the visible fraction of the node in the viewport, 0 to 1. Takes a transition or keyframes without delay or repeat. |
| `motion_inertia(property: String, from: float, velocity: float, config: Map) -> MotionSource` | Creates an inertia source gliding `property` from `from` at `velocity` units/s; `config` sets `friction`, `min`, `max`, `bounce`, `snap_points`. |
| `motion_inertia(property: String, from: ?, velocity: ?, config: Map) -> MotionSource` | Creates an inertia source gliding `property` from `from` at `velocity` units/s; `config` sets `friction`, `min`, `max`, `bounce`, `snap_points`. Numbers may be integers or floats. |
| `motion_keyframes(property: String, frames: Array, config: Map) -> MotionSource` | Creates a keyframe source for `property` from `#{ offset, value, easing }` maps; `config` sets `duration_ms` (240), `delay_ms`, `iterations`. |
| `motion_parallel(steps: Array) -> MotionTimelineStep` | Creates a timeline step that starts all `steps` together and ends with the longest. |
| `motion_path_follow(scene: CanvasScene, key: String, property: String, config: Map) -> MotionSource` | Creates keyframes moving `translate_x`, `translate_y` or `rotate` along path `key` of `scene`; `config` sets `samples` (16 to 512), `duration_ms`. |
| `motion_press(source: MotionSource) -> MotionProgressBinding` | Drives `source` by press: forward while the node is held down, back on release. Takes a transition or keyframes without delay or repeat. |
| `motion_scroll(axis: String, source: MotionSource) -> MotionProgressBinding` | Maps `source` to the scroll position on `axis` (`"x"` or `"y"`) of the nearest scrollable node, 0 at the start to 1 at the end. |
| `motion_sequence(steps: Array) -> MotionTimelineStep` | Creates a timeline step that runs `steps` one after another. |
| `motion_spring(property: String, from: float, to: float, config: Map) -> MotionSource` | Creates a spring source from `from` to `to` on `property`; `config` sets `stiffness` (180), `damping` (24), `mass` (1), `initial_velocity`. |
| `motion_spring(property: String, from: ?, to: ?, config: Map) -> MotionSource` | Creates a spring source from `from` to `to` on `property`; `config` sets `stiffness` (180), `damping` (24), `mass` (1), `initial_velocity`. Numbers may be integers or floats. |
| `motion_stagger(steps: Array, interval_ms: int) -> MotionTimelineStep` | Creates a timeline step that starts each of `steps` `interval_ms` milliseconds after the previous one. |
| `motion_text_spans(text: String, config: Map) -> Dynamic>` | Splits `text` into one fading-in span per grapheme for `text(spans)`; `config` sets `duration_ms` (180), `stagger_ms` (24), `easing`. |
| `motion_timeline(name: String, root: MotionTimelineStep, config: Map) -> MotionTimeline` | Creates timeline `name` from a root step for `node.timeline`; `config` sets `autoplay` (true), `iterations`, `on_complete`, `on_cancel`. |
| `motion_track(target: String, source: MotionSource) -> MotionTimelineStep` | Creates a timeline step that plays `source` on `target`: `"."` for the timeline's own node, or a `/`-separated path of descendant keys. |
| `motion_transition(property: String, from: float, to: float, config: Map) -> MotionSource` | Creates a transition of `property` from `from` to `to`; `config` sets `duration_ms` (180), `delay_ms`, `easing`, `iterations`, `intent`. |
| `motion_transition(property: String, from: ?, to: ?, config: Map) -> MotionSource` | Creates a transition of `property` from `from` to `to`; `config` sets `duration_ms` (180), `delay_ms`, `easing`, `iterations`, `intent`. Numbers may be integers or floats. |
| `motion_viewport(source: MotionSource) -> MotionProgressBinding` | Maps `source` to the node's travel through the viewport: 0 as it enters at the bottom, 1 as it leaves at the top. |
| `native_fuzzy_adjacent(collection: NativeCollection, active: String, step: int) -> String` | Returns the key of the enabled row one before (`step` < 0) or after (`step` > 0) `active` in a fuzzy view, wrapping; `step` 0 returns `active` itself. An `active` that is not an enabled row counts as the first one; `""` when none is enabled. |
| `native_fuzzy_edge(collection: NativeCollection, last: bool) -> String` | Returns the key of the first (or, when `last`, the last) enabled row of a fuzzy view, or `""` when it has none. |
| `native_fuzzy_view(collection: NativeCollection, config: Map) -> NativeCollection` | Returns a fuzzy-filtered, ranked and grouped view of a native collection for Command; `config` sets `query` and the row field names. |
| `native_handler(id: String) -> NativeHandlerRef` | Resolves the Host-registered native handler `"namespace.name"` for use as a callback; fails when no handler has that ID. |
| `native_table_neighbors(collection: NativeCollection, selected_keys: Array) -> Map` | Returns the Table navigation keys `#{ first, last, previous, next, current, current_index }`; `current` is the first shown selected row. Non-string keys match no row. |
| `native_table_view(collection: NativeCollection, config: Map) -> NativeCollection` | Returns a sorted, filtered, paged and grouped view of a native collection, as the Table's normalized `config` describes. |
| `optional_float_signal(key: String) -> NativeSignal` | Declares a component-local optional-float signal that starts as `()` during formal render, for example to bind as `width_override`. |
| `outline_projection(items: Array, expanded_keys: Array) -> Dynamic>` | Flattens `#{ key, parent, label }` tree items into visible rows with `depth` and `has_children`, opening only `expanded_keys`. |
| `path_close() -> CanvasPathSegment` | Creates a path segment that closes the current subpath back to its start. |
| `path_cubic(x: float, y: float, control_a_x: float, control_a_y: float, control_b_x: float, control_b_y: float) -> CanvasPathSegment` | Creates a cubic curve segment to (`x`, `y`) with control points (`control_a_x`, `control_a_y`) and (`control_b_x`, `control_b_y`). |
| `path_cubic(x: ?, y: ?, control_a_x: ?, control_a_y: ?, control_b_x: ?, control_b_y: ?) -> CanvasPathSegment` | Creates a cubic curve segment to (`x`, `y`) with control points (`control_a_x`, `control_a_y`) and (`control_b_x`, `control_b_y`). Numbers may be integers or floats. |
| `path_line(x: float, y: float) -> CanvasPathSegment` | Creates a path segment that draws a straight line to (`x`, `y`). |
| `path_line(x: ?, y: ?) -> CanvasPathSegment` | Creates a path segment that draws a straight line to (`x`, `y`). Numbers may be integers or floats. |
| `path_move(x: float, y: float) -> CanvasPathSegment` | Creates a path segment that starts a new subpath at (`x`, `y`); every path begins with one. |
| `path_move(x: ?, y: ?) -> CanvasPathSegment` | Creates a path segment that starts a new subpath at (`x`, `y`); every path begins with one. Numbers may be integers or floats. |
| `path_quadratic(x: float, y: float, control_x: float, control_y: float) -> CanvasPathSegment` | Creates a quadratic curve segment to (`x`, `y`) with control point (`control_x`, `control_y`); the end point comes first. |
| `path_quadratic(x: ?, y: ?, control_x: ?, control_y: ?) -> CanvasPathSegment` | Creates a quadratic curve segment to (`x`, `y`) with control point (`control_x`, `control_y`); the end point comes first. Numbers may be integers or floats. |
| `propagate() -> EventResponse` | Returns an event response that lets the event continue to ancestor handlers; a handler returning any other value stops it. |
| `shadow(spec: Map) -> ShadowSpec` | Creates a box shadow from `#{ x, y, blur, spread, color }` in pixels; only `color` is required, and `blur` and `spread` are non-negative. |
| `signal(key: String, initial: ?) -> NativeSignal` | Declares a component-local native signal holding a bool, int, float, string or color during formal render; `initial` applies on first mount. |
| `span(text: String) -> Span` | Creates an inline run for `text([...])`, refined with `color`, `background`, `typography`, `bold` and `italic`. |
| `timeout(key: String, delay_ms: int, paused: bool, callback: FnPtr, payload: ?)` | Declares one-shot timer `key` during formal render that calls `callback(ctx, payload)` `delay_ms` (1 ms to 24 h) later, unless `paused`. |

## Context methods (`ctx`)

Methods of the context a view, callback or effect receives.

| Call | Description |
|---|---|
| `ctx.action_enabled(action: String) -> bool` | Returns whether the action `action` (a `namespace.name` id) is registered and enabled; `false` when it is unregistered. |
| `ctx.action_shortcut(action: String) -> ?` | Returns the key binding declared to this view for `action` as `#{ label, keystrokes, chords }`, or `()` when it has none. |
| `ctx.actions() -> Dynamic>` | Lists every registered action as `#{ id, enabled, shortcut }` for a command palette; `shortcut` is `()` when unbound. |
| `ctx.calendar() -> Dynamic>` | Returns the selected locale's calendar metadata: `first_weekday`, `months`, `weekdays` and `date_patterns`. Records a locale dependency in render. |
| `ctx.call_capability(capability: String, method: String, input: ?) -> ?` | Synchronously calls `method` of the declared capability `capability` with `input` and returns its output; not allowed during render. |
| `ctx.cancel_image_decode(handle: ImageDecodeHandle) -> bool` | Cancels a pending image decode so its callbacks never run; returns `false` when it is no longer pending. Raises for another component's handle. |
| `ctx.cancel_motion(handle: MotionHandle)` | Cancels the timeline behind `handle`; raises an error for a stale handle and is not allowed during render. |
| `ctx.cancel_subscription(handle: SubscriptionHandle) -> bool` | Cancels a subscription; returns `false` when it already closed, raises for another component's handle. The producer is told on commit. |
| `ctx.cancel_task(handle: TaskHandle) -> bool` | Cancels a task so its callbacks never run and signals its work on commit; returns `false` when it already finished, raises for another component's handle. |
| `ctx.cancel_timeout(key: String) -> bool` | Cancels this component's declared `timeout` `key` until its declaration changes; returns `false` when no such timer is active. |
| `ctx.clear_close_handler()` | Removes the current window's close handler, so a native close request closes the window again; not allowed during render. |
| `ctx.close_window(id: String)` | Queues a forced close of window `id` that skips its close handler; needs window-command authority and is not allowed during render. |
| `ctx.component_style(part: String, base: Style) -> Style` | Returns `base` merged with the stylesheet rule, the caller's `style` (root only) and `part_styles` for the declared part `part`. |
| `ctx.dispatch_action(action: String, payload: ?)` | Queues the action `action` with `payload` to run after the current callback; raises an error when it is unknown or disabled. |
| `ctx.element_bounds(reference: ElementRef) -> ?` | Returns a ref's last committed `#{ layout, visual, clip }` window bounds, or `()` before first layout; records a dependency in render. |
| `ctx.element_bounds(key: String) -> ?` | Returns the `#{ layout, visual, clip }` window bounds of a component-local ref key, or `()`; the form event callbacks use. |
| `ctx.emit(event: String, payload: ?)` | Emits the declared event `event` with a schema-checked `payload`; the caller's handler runs after this callback. Not allowed during render. |
| `ctx.event_target_bounds() -> ?` | Returns the `#{ x, y, width, height }` window bounds of the node running the current handler, or `()`; untracked, event callbacks only. |
| `ctx.focus(reference: ElementRef)` | Queues keyboard focus for a ref's node, applied after the transaction commits; not allowed during render. |
| `ctx.focus(key: String)` | Queues keyboard focus for the node of a component-local ref key, applied after the transaction commits; not allowed during render. |
| `ctx.focus_window(id: String)` | Queues activation of window `id`; needs window-command authority and is not allowed during render. |
| `ctx.format_date(date: String, style: String) -> String` | Formats an ISO `YYYY-MM-DD` date in the selected locale; `style` is `short`, `medium` or `long`. Records a locale dependency in render. |
| `ctx.format_month_year(date: String) -> String` | Formats the month and year of an ISO `YYYY-MM-DD` date with the selected locale's pattern. Records a locale dependency in render. |
| `ctx.format_number(value: float) -> String` | Formats a finite number with the selected locale's digits and separators. Records a locale dependency in render. |
| `ctx.format_number(value: float, options: Map) -> String` | Formats a finite number in the selected locale; `options` takes `min_fraction_digits`, `max_fraction_digits` (0 to 12) and `grouping`. |
| `ctx.format_number(value: int) -> String` | Formats an integer with the selected locale's digits and separators. Records a locale dependency in render. |
| `ctx.format_number(value: int, options: Map) -> String` | Formats an integer in the selected locale; `options` takes `min_fraction_digits`, `max_fraction_digits` (0 to 12) and `grouping`. |
| `ctx.get_app_store(store: String, field: String) -> ?` | Reads `field` of the app store `store`; in render it subscribes the component to the whole field. |
| `ctx.get_app_store_path(store: String, field: String, path: Array) -> ?` | Reads one nested path of an app store field, in render subscribing to that path only; segments are keys, indexes or `#{ by, key }`. |
| `ctx.get_native_chart_data(name: String) -> NativeChartData` | Returns the Host-registered chart data `name`; the handle notifies its own updates, so the read adds no rerender dependency. |
| `ctx.get_native_collection(name: String) -> NativeCollection` | Returns the Host collection `name` as an opaque view for collection-aware components; in render it subscribes the component to it. |
| `ctx.get_native_text_document(name: String) -> NativeTextDocument` | Returns the Host text document `name`; in render it subscribes the component to its replacement. |
| `ctx.get_signal(signal: NativeSignal) -> ?` | Reads a native signal's current value without recording a dependency; in render it disables reuse of the component's last render. |
| `ctx.get_signal(key: String) -> ?` | Reads this component's signal `key` without recording a dependency; raises an error when no such signal is mounted. |
| `ctx.get_state(field: String) -> ?` | Reads a declared state field of the current component. It needs no dependency: a change to local state rerenders its component. |
| `ctx.get_state_path(field: String, path: Array) -> ?` | Reads one existing nested path of a state field; segments are keys, indexes or `#{ by, key }`. The component stays the dependency. |
| `ctx.get_window_store(store: String, field: String) -> ?` | Reads `field` of the current window's store `store`, in render subscribing to the whole field; raises an error outside a window. |
| `ctx.get_window_store_path(store: String, field: String, path: Array) -> ?` | Reads one nested path of a field of the current window's store, in render subscribing to that path only. |
| `ctx.load_image(asset: AssetId) -> OpaqueHandle` | Loads and caches the logical image `asset` and returns its opaque handle; not allowed during render. |
| `ctx.motion_distance(role: String) -> float` | Returns the theme motion distance in pixels for `role`, such as `subtle`; raises an error for an unknown role. |
| `ctx.motion_duration(role: String) -> int` | Returns the theme motion duration in milliseconds for `role`, such as `fast`; tracked in render, an error for an unknown role. |
| `ctx.motion_easing(role: String) -> String` | Returns the theme easing for `role` as `linear`, `ease_in`, `ease_out` or `ease_in_out`; raises an error for an unknown role. |
| `ctx.motion_handle(name: String) -> MotionHandle` | Returns a handle to the timeline `name` this component declared in the current view; raises an error when missing or duplicated. |
| `ctx.motion_quality() -> String` | Returns the Host's motion quality tier: `low`, `medium` or `high`. |
| `ctx.motion_spring(role: String) -> Dynamic>` | Returns the theme spring preset for `role` as `#{ stiffness, damping, mass }`; raises an error for an unknown role. |
| `ctx.motion_stagger(role: String) -> int` | Returns the theme stagger interval in milliseconds for `role`, such as `tight`; raises an error for an unknown role. |
| `ctx.number() -> Dynamic>` | Returns the selected locale's number metadata: `digits`, separators, group sizes and `minus_sign`. Records a locale dependency in render. |
| `ctx.open_window(id: String, title: String, width: int, height: int, focus: bool)` | Queues a new window `id` that runs the same entry, sides 200 to 4096 pixels; needs window-command authority, not during render. |
| `ctx.pause_motion(handle: MotionHandle)` | Pauses the timeline behind `handle` at its current position; raises an error for a stale handle and is not allowed during render. |
| `ctx.pause_timeout(key: String) -> bool` | Pauses this component's declared `timeout` `key`; returns `false` when no such timer is active. Not allowed during render. |
| `ctx.play_motion(handle: MotionHandle)` | Plays the timeline behind `handle`, resuming a paused position; idempotent while playing and not allowed during render. |
| `ctx.register_action(action: String, callback: FnPtr)` | Registers or replaces the app-wide action `action` (`namespace.name`), enabled, running `callback`; removed when the component unmounts. |
| `ctx.restart_motion(handle: MotionHandle)` | Restarts the timeline behind `handle` from zero, the only way to rewind; not allowed during render. |
| `ctx.resume_motion(handle: MotionHandle)` | Resumes the timeline behind `handle`, the same as `play_motion`; not allowed during render. |
| `ctx.resume_timeout(key: String) -> bool` | Clears an explicit pause of this component's declared `timeout` `key`; returns `false` when no such timer is active. |
| `ctx.scroll_into_view(reference: ElementRef)` | Queues scrolling the nearest retained scroll ancestor so a ref's node shows on the next frame; not allowed during render. |
| `ctx.scroll_into_view(key: String)` | Queues scrolling the nearest retained scroll ancestor to reveal the node of a component-local ref key; not allowed during render. |
| `ctx.scroll_to(reference: ElementRef, x: float, y: float)` | Queues a scroll offset for a ref's scroll container, applied after commit; `x` and `y` must be finite and non-negative. |
| `ctx.scroll_to(reference: ElementRef, x: ?, y: ?)` | Queues a scroll offset for a ref's scroll container, applied after commit; `x` and `y` must be finite and non-negative. Numbers may be integers or floats. |
| `ctx.scroll_to(key: String, x: float, y: float)` | Queues a scroll offset for the scroll container of a component-local ref key; `x` and `y` must be finite and non-negative. |
| `ctx.scroll_to(key: String, x: ?, y: ?)` | Queues a scroll offset for the scroll container of a component-local ref key; `x` and `y` must be finite and non-negative. Numbers may be integers or floats. |
| `ctx.seek_motion(handle: MotionHandle, position_ms: int)` | Moves the timeline behind `handle` to an absolute position in milliseconds; raises an error for a negative position or stale handle. |
| `ctx.set_action_enabled(action: String, enabled: bool)` | Enables or disables the registered action `action`; raises an error when it is unknown and is not allowed during render. |
| `ctx.set_app_store(store: String, field: String, value: ?)` | Writes a schema-checked `value` to `field` of app store `store` and rerenders its readers; not during render, undone on failure. |
| `ctx.set_app_store_path(store: String, field: String, path: Array, value: ?)` | Replaces one existing nested path of an app store field, schema-checking the whole field; rerenders only readers of that path. |
| `ctx.set_close_handler(callback: FnPtr)` | Calls `callback` instead of closing when the user closes the current window; it may confirm, then call `close_window`. |
| `ctx.set_local_locale(locale: String)` | Selects `locale` for the current component subtree and rerenders its locale readers; not allowed during render. |
| `ctx.set_local_theme(family: String, variant: String)` | Selects the theme variant `variant` of `family` for the current component subtree; not allowed during render. |
| `ctx.set_local_theme_system(family: String)` | Makes the current component subtree follow the system light or dark appearance within theme `family`; not allowed during render. |
| `ctx.set_locale(locale: String)` | Selects the app-wide `locale` and rerenders locale readers, keeping state; not allowed during render. |
| `ctx.set_reduced_motion(reduced: bool)` | Requests reduced or normal motion; reduced settles active animation at once. A stricter Host preference still wins. |
| `ctx.set_signal(signal: NativeSignal, value: ?)` | Writes a native signal's value, which must match its type; bound properties update without rerunning Rhai. Not allowed during render. |
| `ctx.set_signal(key: String, value: ?)` | Writes this component's signal `key`, which must match its type; bound properties update without rerunning Rhai. Not allowed during render. |
| `ctx.set_state(field: String, value: ?)` | Writes a schema-checked value to a declared state field and rerenders the component; not during render, undone if the callback fails. |
| `ctx.set_state_path(field: String, path: Array, value: ?)` | Replaces one existing nested path of a state field, schema-checking the whole field, and rerenders the component; not allowed during render. |
| `ctx.set_theme(family: String, variant: String)` | Selects the app-wide theme variant `variant` of `family` without recompiling or resetting state; not allowed during render. |
| `ctx.set_theme_system(family: String)` | Makes the app follow the system light or dark appearance within theme `family`; not allowed during render. |
| `ctx.set_window_locale(locale: String)` | Selects `locale` for the current window and rerenders its locale readers; not allowed during render. |
| `ctx.set_window_store(store: String, field: String, value: ?)` | Writes a schema-checked `value` to `field` of the current window's store and rerenders its readers; not allowed during render. |
| `ctx.set_window_store_path(store: String, field: String, path: Array, value: ?)` | Replaces one existing nested path of a field of the current window's store; rerenders only readers of that path. |
| `ctx.set_window_theme(family: String, variant: String)` | Selects the theme variant `variant` of `family` for the current window; not allowed during render. |
| `ctx.set_window_theme_system(family: String)` | Makes the current window follow the system light or dark appearance within theme `family`; not allowed during render. |
| `ctx.start_image_decode(asset: AssetId, on_success: FnPtr, on_error: FnPtr) -> ImageDecodeHandle` | Decodes the image `asset` in the background, then calls `on_success` or `on_error`, and returns a handle; from `init`, callbacks, effects or `resume`. |
| `ctx.start_subscription(capability: String, method: String, input: ?, on_value: FnPtr, on_error: FnPtr, options: Map) -> SubscriptionHandle` | Starts a capability stream calling `on_value` per item, only from an effect start; `options` takes `delivery`, `capacity`, `throttle_ms`. |
| `ctx.start_task(capability: String, method: String, input: ?, on_success: FnPtr, on_error: FnPtr) -> TaskHandle` | Runs a capability `method` in the background, then calls `on_success` or `on_error`; from `init`, callbacks, effects or `resume`. |
| `ctx.t(key: String) -> String` | Returns the message `key` in the current locale, falling back to the default locale; records a locale dependency in render. |
| `ctx.text_direction() -> String` | Returns `ltr` or `rtl` for the locale of the current component scope; records a locale dependency in render. |
| `ctx.theme_variant() -> ?` | Returns the resolved theme variant as `#{ family, name, mode }`, or `()` without a theme; records a theme dependency in render. |
| `ctx.theme_variants() -> Array` | Lists every loaded theme variant as `#{ family, name, mode }`, ordered by family and name, for a theme picker. |
| `ctx.today() -> String` | Returns today's date as an ISO `YYYY-MM-DD` string from the Host calendar clock; available during render. |
| `ctx.view_id() -> String` | Returns the identity of the mounted script view; raises an error when the context has no view. |
| `ctx.viewport_class() -> String` | Returns `compact`, `regular` or `wide` for the current window's width; records a viewport dependency in render. |
| `ctx.virtual_item_bounds(collection: String, index: int) -> ?` | Returns the window bounds of the item at display `index` of this component's virtual collection `collection`, or `()`; untracked. |
| `ctx.window_id() -> String` | Returns the ID of the current window; raises an error outside a window. |

## Node methods

Methods of `UiNode`; each returns the node, so calls chain.

| Call | Description |
|---|---|
| `node.accessibility_checked(checked: ?) -> UiNode` | Sets the checked state, `true`, `false` or `"mixed"`: checkbox, radio and switch roles report it as toggled unless `accessibility_pressed` is set; tabs, options, rows, grid cells and menu items as selected, a combobox as expanded. |
| `node.accessibility_column_count(count: int) -> UiNode` | Sets the total number of columns of a table or grid, at least 1, including columns that are not rendered. |
| `node.accessibility_column_header(label: String, column: int) -> UiNode` | Makes the node a `columnheader` with the accessible name `label` at the 1-based column index `column`. |
| `node.accessibility_column_index(index: int) -> UiNode` | Sets the node's 1-based column index within its table or grid. |
| `node.accessibility_current(current: String) -> UiNode` | Marks the node as the current item of a set: `page`, `step`, `location`, `date`, `time` or `true`; other values raise an error. |
| `node.accessibility_described_by(ids: String) -> UiNode` | Takes the accessible description from the labels or text of the nodes with these space-separated `accessibility_id`s. |
| `node.accessibility_expanded(expanded: bool) -> UiNode` | Sets whether the content the node controls, such as a menu or a disclosure panel, is expanded. |
| `node.accessibility_id(id: String) -> UiNode` | Gives the node a semantic ID, unique in the view, for `accessibility_labelled_by`, `accessibility_described_by` and automation. |
| `node.accessibility_invalid(invalid: bool) -> UiNode` | Marks the node's value as invalid, as for a form field that fails validation. |
| `node.accessibility_key_shortcuts(shortcuts: String) -> UiNode` | Announces the shortcuts that activate the node, in ARIA `keyshortcuts` form such as `"Meta+S"`; it binds no keys. |
| `node.accessibility_label(label: String) -> UiNode` | Sets the node's accessible name, which takes precedence over `accessibility_labelled_by` and the node's own text. |
| `node.accessibility_labelled_by(ids: String) -> UiNode` | Names the node from the labels or text of the nodes with these space-separated `accessibility_id`s, unless it has a label. |
| `node.accessibility_level(level: int) -> UiNode` | Sets the node's hierarchical level, at least 1, such as a heading level or a tree item's depth. |
| `node.accessibility_option(label: String, selected: bool, position: int, size: int) -> UiNode` | Makes the node an `option` named `label`, with its selected state, at the 1-based `position` in a set of `size` options. |
| `node.accessibility_orientation(orientation: String) -> UiNode` | Sets the orientation of a slider, toolbar or list, `horizontal` or `vertical`; other values raise an error. |
| `node.accessibility_placeholder(placeholder: String) -> UiNode` | Sets the placeholder text assistive technology announces for an empty input. |
| `node.accessibility_position_in_set(position: int) -> UiNode` | Sets the node's 1-based position among the items of its set, such as the options of a listbox. |
| `node.accessibility_pressed(pressed: bool) -> UiNode` | Sets the pressed state of a toggle button, reported as toggled on or off; it takes precedence over `accessibility_checked`. |
| `node.accessibility_read_only(read_only: bool) -> UiNode` | Marks the node as read-only; accessibility actions can no longer set its value. |
| `node.accessibility_required(required: bool) -> UiNode` | Marks the node as a form field that must be filled in. |
| `node.accessibility_role(role: String) -> UiNode` | Sets the node's semantic role, such as `button`, `checkbox` or `presentation`; an unknown role rejects the render at commit. |
| `node.accessibility_row_count(count: int) -> UiNode` | Sets the total number of rows of a table or grid, at least 1, including rows that are not rendered. |
| `node.accessibility_row_index(index: int) -> UiNode` | Sets the node's 1-based row index within its table or grid. |
| `node.accessibility_selected(selected: bool) -> UiNode` | Sets whether the node, such as a tab, an option or a row, is selected. |
| `node.accessibility_size_of_set(size: int) -> UiNode` | Sets how many items, at least 1, are in the set the node belongs to. |
| `node.accessibility_table_cell(label: String, column: int) -> UiNode` | Makes the node a `gridcell` with the accessible name `label` at the 1-based column index `column`. |
| `node.accessibility_table_row(label: String, selected: bool, row: int, position: int, size: int) -> UiNode` | Makes the node a `row` named `label`, with its selected state, 1-based row index `row`, and `position` in a set of `size`. |
| `node.accessibility_value(value: ?) -> UiNode` | Sets the value assistive technology reads; it is announced as text, and a number is also reported as the numeric value. |
| `node.accessibility_value_max(max: ?) -> UiNode` | Sets the maximum of the node's numeric range, such as a slider's; `max` must be a finite number. |
| `node.accessibility_value_min(min: ?) -> UiNode` | Sets the minimum of the node's numeric range, such as a slider's; `min` must be a finite number. |
| `node.audit_allow(rules: Array) -> UiNode` | Exempts the node from the named composition audit rules, such as `"mixed-type-in-row"`; an unknown rule raises an error. |
| `node.bind_parent_signal(ctx: UiContext, property: String, key: String) -> UiNode` | Drives an optional-float `property` such as `width_override` from the signal `key` of the nearest ancestor that declares it; raises when none does. |
| `node.bind_signal(property: String, signal: NativeSignal) -> UiNode` | Drives `property` (`opacity`, `translate_x`, `width`, `background`, ...) from a native signal of its type, without a rerender. |
| `node.disabled(disabled: bool) -> UiNode` | Disables the node and its subtree: their handlers stop firing, `disabled` styles apply, and assistive technology reports it. |
| `node.enter_motion(source: MotionSource) -> UiNode` | Declares a motion source that plays when the keyed node mounts or its replay key changes; the same as `motion`. |
| `node.env(values: Map) -> UiNode` | Sets environment axes for the node and its subtree, such as `#{ density: "compact" }`, merged with earlier `env` values on the node. |
| `node.exit_motion(source: MotionSource) -> UiNode` | Declares a motion played on a paint-only copy of the keyed node after it is removed; replaces an exit source for that property. |
| `node.heading_elsewhere() -> UiNode` | Tells the composition audit that this container's heading is drawn elsewhere, such as in a host's panel header. |
| `node.layout_motion(duration_ms: int, easing: String) -> UiNode` | Animates the node between committed layout positions over `duration_ms`, with `linear`, `ease_in`, `ease_out` or `ease_in_out`. |
| `node.motion(source: MotionSource) -> UiNode` | Declares a motion source for one property of the keyed node, replacing an earlier source for it; rerenders keep its progress. |
| `node.motion_particle_count(count: int) -> UiNode` | Declares how many particles the node draws itself, such as Canvas circles, against the `motion_particles` budget; it draws nothing. At least 0. |
| `node.motion_replay_key(key: String) -> UiNode` | Replays the node's unchanged motion declarations whenever `key` changes, such as when a semantically new value arrives. |
| `node.on(event: String, handler: FnPtr) -> UiNode` | Appends a target-phase handler for `event` (`click`, `pointer_down`, `wheel`, `key:escape`, ...), called with `ctx` and the payload. |
| `node.on(event: String, handler: NativeHandlerRef) -> UiNode` | Appends a Rust native handler for `event` in the target phase; the handler's descriptor must declare `event`. |
| `node.on_bubble(event: String, handler: FnPtr) -> UiNode` | Appends a bubble-phase handler for a pointer, wheel or `key:` event, run after target handlers, from inner to outer nodes. |
| `node.on_bubble(event: String, handler: NativeHandlerRef) -> UiNode` | Appends a Rust native handler for the bubble phase of `event`; the handler's descriptor must declare `event`. |
| `node.on_capture(event: String, handler: FnPtr) -> UiNode` | Appends a capture-phase handler for a pointer, wheel or `key:` event, run from outer to inner nodes before target handlers. |
| `node.on_capture(event: String, handler: NativeHandlerRef) -> UiNode` | Appends a Rust native handler for the capture phase of `event`; the handler's descriptor must declare `event`. |
| `node.on_change(handler: FnPtr) -> UiNode` | Appends a handler for the `change` event a primitive such as a text input or slider emits with its proposed value. |
| `node.on_click(handler: FnPtr) -> UiNode` | Appends a click handler, run on a mouse click or Enter or Space while focused, with payload `()`; makes the node a tab stop. |
| `node.on_click(handler: NativeHandlerRef) -> UiNode` | Appends a Rust native click handler, run like a script one; the handler's descriptor must declare `click`. |
| `node.on_click_value(handler: FnPtr, value: ?) -> UiNode` | Appends a click handler that receives `value` as its payload, so one named function can serve many nodes; other click handlers keep theirs. |
| `node.on_click_value(handler: NativeHandlerRef, value: ?) -> UiNode` | Appends a Rust native click handler that receives `value` as its payload, not shared with other click handlers; its descriptor must declare `click`. |
| `node.on_hover_change(handler: FnPtr) -> UiNode` | Appends a handler called with `true` when the pointer enters the node and `false` when it leaves. |
| `node.on_hover_value(handler: FnPtr, value: ?) -> UiNode` | Appends a hover handler whose payload is `#{ hovered, value }`: `hovered` is `true` on enter and `false` on leave. Other hover handlers keep theirs. |
| `node.on_key_value(key: String, handler: FnPtr, value: ?) -> UiNode` | Appends a handler for `key` (`escape`, `ctrl+s`, ...) pressed while focus is on or inside the node, with its own `value` as payload. |
| `node.on_open_change(handler: FnPtr) -> UiNode` | Appends a handler for an overlay's `open_change` event, called with the requested open state, such as `false` on dismissal. |
| `node.progress_motion(binding: MotionProgressBinding) -> UiNode` | Attaches a natively driven motion such as `motion_hover(...)` or `motion_scroll(...)`, replacing one for the same property. |
| `node.scrollbars(horizontal: String, vertical: String) -> UiNode` | Overlays themed scrollbars on a scrollable node; each axis is `auto`, `always` or `hidden`, which changes presentation only. |
| `node.selectable(selectable: bool) -> UiNode` | Lets the user drag-select a `text()` node and copy it with the platform copy action; `true` on other nodes raises an error. |
| `node.shared_layout(id: String) -> UiNode` | Gives the node the shared-layout identity `id` in the group of the enclosing `motion_group`, linking its layout motion. |
| `node.shared_layout(group: String, id: String) -> UiNode` | Gives the node the shared-layout identity `id` in `group`, overriding an enclosing `motion_group`; unique in the window. |
| `node.signal_style(signal: NativeSignal, states: Map) -> UiNode` | Merges `states[value]` over the node's style, where `value` is the current value of a string signal; switches without a rerender. |
| `node.tab_group() -> UiNode` | Makes the node a tab group, so the `tab_index` values of its descendants order focus locally; the node is a tab stop itself unless `tab_stop(false)`. |
| `node.tab_index(index: int) -> UiNode` | Sets the node's tab order within its tab group, from -32768 to 32767, and makes it a tab stop unless `tab_stop(false)`. |
| `node.tab_stop(tab_stop: bool) -> UiNode` | Sets whether Tab reaches the node; `false` keeps it focusable from code but out of keyboard traversal. |
| `node.test_id(id: String) -> UiNode` | Sets a non-semantic automation ID of 1-128 letters, digits, `_`, `-`, `.` or `:` that automation locators find. |
| `node.timeline(timeline: MotionTimeline) -> UiNode` | Attaches a motion timeline to the keyed node, replacing one of the same name; `ctx.motion_handle(name)` controls it. |
| `node.translate_wheel() -> UiNode` | Lets a one-axis scroll container also scroll with the other wheel axis, as a tab strip under a vertical mouse wheel. |
| `node.window_drag_area() -> UiNode` | Makes a press on the node move the window and a double press run the title-bar action; inert unless the Host allows it. |
| `node.with_key(key: String) -> UiNode` | Sets the node's stable identity among its siblings, which keeps its state, focus, motion and refs across reorders. |
| `node.with_part_style(part: String, style: Style) -> UiNode` | Sets the style of a named part the node draws, such as `scrollbar_thumb` or a layer's `backdrop`, replacing an earlier one. |
| `node.with_ref(reference: ElementRef) -> UiNode` | Attaches an `element_ref`, through which the component reads the node's committed bounds, focuses it or scrolls it. |
| `node.with_style(style: Style) -> UiNode` | Merges `style` over the node's style: the properties it sets override earlier ones and the rest are kept. |
| `node.with_table_column(index: int) -> UiNode` | Places the node in the 0-based column `index` (below 256) of the enclosing table track, which sets its width. |
| `node.with_table_track(columns: Array, width_signals: Array) -> UiNode` | Makes a box a table column plan of `#{ key, width: #{ kind, value } }` maps; `width_signals` is `[]` or one override signal per column. |

## Style methods

Methods of the `style()` builder; each returns the style, so calls chain.

| Call | Description |
|---|---|
| `style.absolute() -> Style` | Takes the node out of the flow and places it by its insets (`top`, `left`, `inset_start`, ...) within its parent. |
| `style.active(style: Style) -> Style` | Paints the nested style's background, border color, text color and opacity while the pointer is pressed on the node. |
| `style.background(color: ColorValue) -> Style` | Fills the node with a solid color, literal or theme token; replaces a gradient set earlier on this style. |
| `style.block() -> Style` | Uses block layout: children stack vertically in normal flow and flex alignment does not apply. |
| `style.border(width: Length) -> Style` | Sets the border width of all four edges in px or rem, replacing per-edge widths; a `relative` length is ignored. |
| `style.border_bottom(width: Length) -> Style` | Sets the bottom border width in px or rem; a `relative` length is ignored. |
| `style.border_color(color: ColorValue) -> Style` | Sets one border color, literal or theme token, for every edge. |
| `style.border_dashed() -> Style` | Draws the borders dashed; one line style applies to every edge. |
| `style.border_end(width: Length) -> Style` | Sets the border width of the logical end edge (right in LTR, left in RTL); overrides the physical edge there. |
| `style.border_left(width: Length) -> Style` | Sets the left border width in px or rem; `border_start`/`border_end` override it on that side. |
| `style.border_right(width: Length) -> Style` | Sets the right border width in px or rem; `border_start`/`border_end` override it on that side. |
| `style.border_solid() -> Style` | Draws the borders solid, the default; one line style applies to every edge. |
| `style.border_start(width: Length) -> Style` | Sets the border width of the logical start edge (left in LTR, right in RTL); overrides the physical edge there. |
| `style.border_top(width: Length) -> Style` | Sets the top border width in px or rem; a `relative` length is ignored. |
| `style.bottom(inset: AutoLength) -> Style` | Sets the bottom inset of a positioned node to auto. |
| `style.bottom(inset: Length) -> Style` | Sets the bottom inset of a positioned node in px, rem or a `relative` fraction of the parent's height. |
| `style.bottom(inset: SignedLength) -> Style` | Sets the bottom inset of a positioned node to a signed `offset_*` length, which may be negative. |
| `style.clip() -> Style` | Clips the node's children to its bounds without making it scrollable. |
| `style.col_span(span: int) -> Style` | Makes a grid item span `span` columns, from 1 to 1024. |
| `style.cursor_crosshair() -> Style` | Shows the crosshair cursor while the pointer is over the node. |
| `style.cursor_default() -> Style` | Shows the default arrow cursor while the pointer is over the node. |
| `style.cursor_move() -> Style` | Shows the closed-hand (grabbing) cursor while the pointer is over the node. |
| `style.cursor_not_allowed() -> Style` | Shows the not-allowed cursor while the pointer is over the node. |
| `style.cursor_pointer() -> Style` | Shows the pointing-hand cursor while the pointer is over the node. |
| `style.cursor_resize_x() -> Style` | Shows the left-right resize cursor while the pointer is over the node. |
| `style.cursor_resize_y() -> Style` | Shows the up-down resize cursor while the pointer is over the node. |
| `style.cursor_text() -> Style` | Shows the text I-beam cursor while the pointer is over the node. |
| `style.disabled(style: Style) -> Style` | Merges the nested style over the node while it or an ancestor is disabled; `hover`, `active` and `focus` paint then stop. |
| `style.flex_basis(basis: AutoLength) -> Style` | Sets the flex basis to auto: the node starts from its own size before growing or shrinking. |
| `style.flex_basis(basis: Length) -> Style` | Sets the main-axis size the node starts from before growing or shrinking, in px, rem or a `relative` fraction. |
| `style.flex_col() -> Style` | Makes the node a flex container that stacks its children vertically. |
| `style.flex_grow() -> Style` | Lets the node grow into free main-axis space with factor 1, replacing an earlier weight. |
| `style.flex_grow(weight: float) -> Style` | Lets the node grow into free main-axis space by a positive `weight`, relative to its siblings' weights. |
| `style.flex_grow(weight: int) -> Style` | Lets the node grow into free main-axis space by a positive `weight`, relative to its siblings' weights. |
| `style.flex_nowrap() -> Style` | Keeps a flex container's children on one line. |
| `style.flex_row() -> Style` | Makes the node a flex container that lays its children out in a row from the logical start, mirrored in RTL. |
| `style.flex_shrink(shrink: bool) -> Style` | Sets whether the node may shrink below its flex basis when space runs out: `true` lets it, `false` forbids it. |
| `style.flex_wrap() -> Style` | Lets a flex container wrap its children onto further lines. |
| `style.flex_wrap_reverse() -> Style` | Lets a flex container wrap its children onto further lines, stacked in reverse cross-axis order. |
| `style.focus(style: Style) -> Style` | Paints the nested style's background, border color (default `focus_ring`), text color and opacity while the node has keyboard focus. |
| `style.focus_within(style: Style) -> Style` | Merges the nested style over the node while it or any descendant has keyboard focus. |
| `style.font_fallbacks(families: Array) -> Style` | Sets the ordered fallback font families, 1 to 16 unique non-empty names, used for glyphs the primary font lacks. |
| `style.font_family(family: String) -> Style` | Sets the font family by name, a system or registered font of at most 256 bytes; overrides the typography role's family. |
| `style.font_feature(tag: String, value: int) -> Style` | Sets an OpenType feature: `tag` is four ASCII letters or digits like `"liga"`, `value` 0 to 65535; calls accumulate. |
| `style.font_size(size: Length) -> Style` | Sets the font size in px or rem, overriding the typography role's; a `relative` length is ignored. |
| `style.font_weight(weight: int) -> Style` | Sets the numeric font weight, 1 to 1000 (400 regular, 700 bold), overriding the typography role's. |
| `style.gap(gap: Length) -> Style` | Sets the space between children on both axes, in px, rem or a `relative` fraction of the container. |
| `style.grid() -> Style` | Makes the node a grid container; `grid_cols` and `grid_rows` set its tracks. |
| `style.grid_cols(columns: int) -> Style` | Makes the node a grid with `columns` equal-width columns, from 1 to 1024. |
| `style.grid_rows(rows: int) -> Style` | Makes the node a grid with `rows` equal-height rows, from 1 to 1024. |
| `style.group_focus(style: Style) -> Style` | Merges the nested style over the node while its nearest focusable ancestor-or-self with a `focus` style has keyboard focus. |
| `style.group_hover(style: Style) -> Style` | Paints the nested style's background, border color, text color and opacity while the nearest ancestor with a `hover` style is hovered. |
| `style.height(length: AutoLength) -> Style` | Sets the height to auto, so content and layout decide it. |
| `style.height(length: Length) -> Style` | Sets the height in px, rem or a `relative` fraction of the parent's height. |
| `style.hidden() -> Style` | Removes the node from layout and paint (display none); `invisible` keeps its space instead. |
| `style.hover(style: Style) -> Style` | Paints the nested style's background, border color, text color and opacity while the pointer is over the node. |
| `style.inset_end(inset: AutoLength) -> Style` | Sets the inset from the logical end edge (right in LTR, left in RTL) to auto; overrides `left`/`right` on that side. |
| `style.inset_end(inset: Length) -> Style` | Sets the inset from the logical end edge (right in LTR, left in RTL) in px, rem or a `relative` fraction; overrides `left`/`right` there. |
| `style.inset_end(inset: SignedLength) -> Style` | Sets the inset from the logical end edge (right in LTR, left in RTL) to a signed `offset_*` length; overrides `left`/`right`. |
| `style.inset_start(inset: AutoLength) -> Style` | Sets the inset from the logical start edge (left in LTR, right in RTL) to auto; overrides `left`/`right` on that side. |
| `style.inset_start(inset: Length) -> Style` | Sets the inset from the logical start edge (left in LTR, right in RTL) in px, rem or a `relative` fraction; overrides `left`/`right` there. |
| `style.inset_start(inset: SignedLength) -> Style` | Sets the inset from the logical start edge (left in LTR, right in RTL) to a signed `offset_*` length; overrides `left`/`right`. |
| `style.invisible() -> Style` | Hides the node but keeps its layout space; `visible` undoes it. |
| `style.italic() -> Style` | Sets the italic font style. |
| `style.items_center() -> Style` | Centers children on the container's cross axis. |
| `style.items_end() -> Style` | Aligns children to the cross-axis end: the bottom of a row, the logical end of a column (left in RTL). |
| `style.items_start() -> Style` | Aligns children to the cross-axis start: the top of a row, the logical start of a column (right in RTL). |
| `style.justify_around() -> Style` | Distributes free main-axis space evenly around each child. |
| `style.justify_between() -> Style` | Puts the first and last children at the main-axis ends and spreads the free space evenly between children. |
| `style.justify_center() -> Style` | Centers children along the main axis. |
| `style.justify_end() -> Style` | Packs children toward the end of the main axis; in an RTL row that is the left. |
| `style.justify_start() -> Style` | Packs children toward the start of the main axis; in an RTL row that is the right. |
| `style.left(inset: AutoLength) -> Style` | Sets the left inset of a positioned node to auto; `inset_start`/`inset_end` win there. |
| `style.left(inset: Length) -> Style` | Sets the left inset of a positioned node in px, rem or a `relative` fraction of the parent's width; `inset_start`/`inset_end` win there. |
| `style.left(inset: SignedLength) -> Style` | Sets the left inset of a positioned node to a signed `offset_*` length, which may be negative; `inset_start`/`inset_end` win there. |
| `style.line_clamp(lines: int) -> Style` | Limits text to `lines` lines, from 1 to 10000, and hides the rest; `text_ellipsis` marks the cut. |
| `style.line_height(height: Length) -> Style` | Sets the line height in px or rem, overriding the typography role's; a `relative` length is ignored. |
| `style.linear_gradient(gradient: LinearGradientSpec) -> Style` | Fills the node with a two-stop gradient from `linear_gradient(#{angle, from, to})`; replaces a background set earlier on this style. |
| `style.margin(margin: AutoLength) -> Style` | Sets all four margins to auto, centering the node in the free space, and replaces per-edge margins. |
| `style.margin(margin: Length) -> Style` | Sets all four margins in px, rem or a `relative` fraction, replacing per-edge margins set earlier. |
| `style.margin(margin: SignedLength) -> Style` | Sets all four margins to a signed `offset_*` length, which may be negative, replacing per-edge margins. |
| `style.margin_bottom(margin: AutoLength) -> Style` | Sets the bottom margin to auto, taking up the free space there. |
| `style.margin_bottom(margin: Length) -> Style` | Sets the bottom margin in px, rem or a `relative` fraction. |
| `style.margin_bottom(margin: SignedLength) -> Style` | Sets the bottom margin to a signed `offset_*` length, which may be negative. |
| `style.margin_end(margin: AutoLength) -> Style` | Sets the logical end margin (right in LTR, left in RTL) to auto, taking the free space there; overrides the physical margin. |
| `style.margin_end(margin: Length) -> Style` | Sets the logical end margin (right in LTR, left in RTL) in px, rem or a `relative` fraction; overrides the physical margin there. |
| `style.margin_end(margin: SignedLength) -> Style` | Sets the logical end margin (right in LTR, left in RTL) to a signed `offset_*` length; overrides the physical margin there. |
| `style.margin_left(margin: AutoLength) -> Style` | Sets the left margin to auto, taking up the free space there; `margin_start`/`margin_end` override it on that side. |
| `style.margin_left(margin: Length) -> Style` | Sets the left margin in px, rem or a `relative` fraction; `margin_start`/`margin_end` override it on that side. |
| `style.margin_left(margin: SignedLength) -> Style` | Sets the left margin to a signed `offset_*` length, which may be negative; `margin_start`/`margin_end` override it on that side. |
| `style.margin_right(margin: AutoLength) -> Style` | Sets the right margin to auto, taking up the free space there; `margin_start`/`margin_end` override it on that side. |
| `style.margin_right(margin: Length) -> Style` | Sets the right margin in px, rem or a `relative` fraction; `margin_start`/`margin_end` override it on that side. |
| `style.margin_right(margin: SignedLength) -> Style` | Sets the right margin to a signed `offset_*` length, which may be negative; `margin_start`/`margin_end` override it on that side. |
| `style.margin_start(margin: AutoLength) -> Style` | Sets the logical start margin (left in LTR, right in RTL) to auto, taking the free space there; overrides the physical margin. |
| `style.margin_start(margin: Length) -> Style` | Sets the logical start margin (left in LTR, right in RTL) in px, rem or a `relative` fraction; overrides the physical margin there. |
| `style.margin_start(margin: SignedLength) -> Style` | Sets the logical start margin (left in LTR, right in RTL) to a signed `offset_*` length; overrides the physical margin there. |
| `style.margin_top(margin: AutoLength) -> Style` | Sets the top margin to auto, taking up the free space there. |
| `style.margin_top(margin: Length) -> Style` | Sets the top margin in px, rem or a `relative` fraction. |
| `style.margin_top(margin: SignedLength) -> Style` | Sets the top margin to a signed `offset_*` length, which may be negative. |
| `style.margin_x(margin: AutoLength) -> Style` | Sets the left and right margins to auto, centering the node horizontally. |
| `style.margin_x(margin: Length) -> Style` | Sets the left and right margins in px, rem or a `relative` fraction; `margin_start`/`margin_end` still override. |
| `style.margin_x(margin: SignedLength) -> Style` | Sets the left and right margins to a signed `offset_*` length, which may be negative. |
| `style.margin_y(margin: AutoLength) -> Style` | Sets the top and bottom margins to auto, centering the node vertically. |
| `style.margin_y(margin: Length) -> Style` | Sets the top and bottom margins in px, rem or a `relative` fraction. |
| `style.margin_y(margin: SignedLength) -> Style` | Sets the top and bottom margins to a signed `offset_*` length, which may be negative. |
| `style.max_height(length: AutoLength) -> Style` | Removes a max height limit by setting it to auto. |
| `style.max_height(length: Length) -> Style` | Sets the max height in px, rem or a `relative` fraction of the parent's height. |
| `style.max_width(length: AutoLength) -> Style` | Removes a max width limit by setting it to auto. |
| `style.max_width(length: Length) -> Style` | Sets the max width in px, rem or a `relative` fraction of the parent's width. |
| `style.merge(overlay: Style) -> Style` | Lays every property `overlay` sets over this style; pseudo-state styles merge state by state. |
| `style.min_height(length: AutoLength) -> Style` | Sets the min height to auto, the content-based minimum of a flex item. |
| `style.min_height(length: Length) -> Style` | Sets the min height in px, rem or a `relative` fraction of the parent's height. |
| `style.min_width(length: AutoLength) -> Style` | Sets the min width to auto, the content-based minimum of a flex item. |
| `style.min_width(length: Length) -> Style` | Sets the min width in px, rem or a `relative` fraction of the parent's width. |
| `style.not_italic() -> Style` | Sets the upright font style, undoing `italic`. |
| `style.occlude() -> Style` | Blocks all pointer input, scrolling included, from reaching elements painted behind the node. |
| `style.occlude_except_scroll() -> Style` | Blocks pointer input from reaching elements painted behind the node but lets scroll wheel input through. |
| `style.opacity(opacity: float) -> Style` | Sets the node's opacity from 0 (transparent) to 1 (opaque); a bound signal or animation overrides it. |
| `style.opacity(opacity: int) -> Style` | Sets the node's opacity to 0 (transparent) or 1 (opaque); a bound signal or animation overrides it. |
| `style.overflow_hidden() -> Style` | Clips children to the node's bounds on both axes, without scrolling. |
| `style.overflow_scroll() -> Style` | Makes the node scroll on both axes, clipping its children to its bounds. |
| `style.overflow_x_scroll() -> Style` | Makes the node scroll horizontally; wheel input stays on that axis unless the node calls `translate_wheel()`. |
| `style.overflow_y_scroll() -> Style` | Makes the node scroll vertically; wheel input stays on that axis unless the node calls `translate_wheel()`. |
| `style.padding(padding: Length) -> Style` | Sets all four paddings in px, rem or a `relative` fraction, replacing per-edge paddings set earlier. |
| `style.padding_bottom(padding: Length) -> Style` | Sets the bottom padding in px, rem or a `relative` fraction. |
| `style.padding_end(padding: Length) -> Style` | Sets the padding of the logical end edge (right in LTR, left in RTL); overrides the physical padding there. |
| `style.padding_left(padding: Length) -> Style` | Sets the left padding in px, rem or a `relative` fraction; `padding_start`/`padding_end` override it on that side. |
| `style.padding_right(padding: Length) -> Style` | Sets the right padding in px, rem or a `relative` fraction; `padding_start`/`padding_end` override it on that side. |
| `style.padding_start(padding: Length) -> Style` | Sets the padding of the logical start edge (left in LTR, right in RTL); overrides the physical padding there. |
| `style.padding_top(padding: Length) -> Style` | Sets the top padding in px, rem or a `relative` fraction. |
| `style.padding_x(padding: Length) -> Style` | Sets the left and right padding in px, rem or a `relative` fraction; `padding_start`/`padding_end` still override. |
| `style.padding_y(padding: Length) -> Style` | Sets the top and bottom padding in px, rem or a `relative` fraction. |
| `style.radius(radius: Length) -> Style` | Rounds all four corners in px or rem, replacing per-corner radii; a `relative` length is ignored. |
| `style.radius_bottom_left(radius: Length) -> Style` | Rounds the bottom-left corner in px or rem; `radius_start`/`radius_end` override it on that side. |
| `style.radius_bottom_right(radius: Length) -> Style` | Rounds the bottom-right corner in px or rem; `radius_start`/`radius_end` override it on that side. |
| `style.radius_end(radius: Length) -> Style` | Rounds both corners of the logical end side (right in LTR, left in RTL); overrides the physical corners there. |
| `style.radius_start(radius: Length) -> Style` | Rounds both corners of the logical start side (left in LTR, right in RTL); overrides the physical corners there. |
| `style.radius_top_left(radius: Length) -> Style` | Rounds the top-left corner in px or rem; `radius_start`/`radius_end` override it on that side. |
| `style.radius_top_right(radius: Length) -> Style` | Rounds the top-right corner in px or rem; `radius_start`/`radius_end` override it on that side. |
| `style.relative() -> Style` | Keeps the node in normal flow, shifted by any insets; this is the default and undoes `absolute`. |
| `style.right(inset: AutoLength) -> Style` | Sets the right inset of a positioned node to auto; `inset_start`/`inset_end` win there. |
| `style.right(inset: Length) -> Style` | Sets the right inset of a positioned node in px, rem or a `relative` fraction of the parent's width; `inset_start`/`inset_end` win there. |
| `style.right(inset: SignedLength) -> Style` | Sets the right inset of a positioned node to a signed `offset_*` length, which may be negative; `inset_start`/`inset_end` win there. |
| `style.row_span(span: int) -> Style` | Makes a grid item span `span` rows, from 1 to 1024. |
| `style.self_center() -> Style` | Centers this node on its parent's cross axis, overriding the parent's `items_*`. |
| `style.self_end() -> Style` | Aligns this node to the end of its parent's cross axis, overriding `items_*`; start and end swap in RTL. |
| `style.self_start() -> Style` | Aligns this node to the start of its parent's cross axis, overriding `items_*`; start and end swap in RTL. |
| `style.self_stretch() -> Style` | Stretches this node across its parent's cross axis, overriding the parent's `items_*`. |
| `style.shadow(shadow: ShadowSpec) -> Style` | Sets one box shadow from `shadow(#{x, y, blur, spread, color})`, in logical pixels, replacing an earlier one. |
| `style.text_center() -> Style` | Centers text horizontally. |
| `style.text_color(color: ColorValue) -> Style` | Sets the text color, literal or theme token; descendants inherit it. |
| `style.text_ellipsis() -> Style` | Truncates text that overflows the available width with an ellipsis at the end. |
| `style.text_left() -> Style` | Aligns text to the logical start: left in LTR, right in RTL. |
| `style.text_right() -> Style` | Aligns text to the logical end: right in LTR, left in RTL. |
| `style.top(inset: AutoLength) -> Style` | Sets the top inset of a positioned node to auto. |
| `style.top(inset: Length) -> Style` | Sets the top inset of a positioned node in px, rem or a `relative` fraction of the parent's height. |
| `style.top(inset: SignedLength) -> Style` | Sets the top inset of a positioned node to a signed `offset_*` length, which may be negative. |
| `style.translate_x(offset: float) -> Style` | Moves the painted node right by `offset` logical pixels (negative moves left, not mirrored in RTL); layout is unchanged. |
| `style.translate_x(offset: int) -> Style` | Moves the painted node right by `offset` logical pixels (negative moves left, not mirrored in RTL); layout is unchanged. |
| `style.translate_y(offset: float) -> Style` | Moves the painted node down by `offset` logical pixels (negative moves up); layout is unchanged. |
| `style.translate_y(offset: int) -> Style` | Moves the painted node down by `offset` logical pixels (negative moves up); layout is unchanged. |
| `style.typography(role: String) -> Style` | Applies a theme typography role such as `"body"` or `"code"`; explicit family, fallbacks, size, weight or line height win. |
| `style.visible() -> Style` | Makes the node visible again, undoing `invisible`. |
| `style.whitespace_normal() -> Style` | Wraps text at the available width. |
| `style.whitespace_nowrap() -> Style` | Keeps text on one line without wrapping. |
| `style.width(length: AutoLength) -> Style` | Sets the width to auto, so content and layout decide it. |
| `style.width(length: Length) -> Style` | Sets the width in px, rem or a `relative` fraction of the parent's width. |

## Methods of other values

Properties and methods of handles, signals, documents, collections and canvas values.

| Call | Description |
|---|---|
| `asset_id.to_string() -> String` | Returns the namespaced logical asset ID, such as `core/check`. |
| `canvas_command.clip_rect(x: float, y: float, width: float, height: float) -> CanvasCommand` | Clips a path command to an axis-aligned Canvas-local rectangle that does not rotate with it, replacing an earlier clip; other commands raise an error. |
| `canvas_command.clip_rect(x: ?, y: ?, width: ?, height: ?) -> CanvasCommand` | Clips a path command to an axis-aligned Canvas-local rectangle that does not rotate with it, replacing an earlier clip; other commands raise an error. Numbers may be integers or floats. |
| `canvas_command.rotate(degrees: float) -> CanvasCommand` | Sets the path's rotation in degrees about the Canvas origin, replacing an earlier one; other commands raise an error. |
| `canvas_command.rotate(degrees: int) -> CanvasCommand` | Sets the path's rotation in degrees about the Canvas origin, replacing an earlier one; other commands raise an error. |
| `canvas_command.scale(factor: float) -> CanvasCommand` | Sets the path's positive uniform scale, applied before rotation and translation and replacing an earlier one; other commands raise an error. |
| `canvas_command.scale(factor: int) -> CanvasCommand` | Sets the path's positive uniform scale, applied before rotation and translation and replacing an earlier one; other commands raise an error. |
| `canvas_command.translate(x: float, y: float) -> CanvasCommand` | Sets the path's translation, applied after scale and rotation and replacing an earlier one; other commands raise an error. |
| `canvas_command.translate(x: ?, y: ?) -> CanvasCommand` | Sets the path's translation, applied after scale and rotation and replacing an earlier one; other commands raise an error. Numbers may be integers or floats. |
| `element_ref.key -> String` | The component-local key the ref was declared with in `element_ref(key)`. |
| `element_ref.scope -> String` | The ref's full identity, `<component path>:<key>`, which tells apart refs with the same key in different components. |
| `event_response.capture_pointer() -> EventResponse` | Captures the pointer for the handler's node, so its later move and up events go to that node until released. |
| `event_response.prevent_default() -> EventResponse` | Prevents the event's native default, such as an ancestor taking focus on press or an enclosing window drag area moving the window. |
| `event_response.release_pointer() -> EventResponse` | Releases this pointer's capture, so its later events hit-test normally again. |
| `event_response.stop() -> EventResponse` | Stops the event from reaching further nodes; the current node's remaining handlers for this phase still run. |
| `event_response.stop_immediate() -> EventResponse` | Stops the event at once: no further handler runs, on this node or any other. |
| `image_decode_handle.to_string() -> String` | Returns a debug label for the pending decode, `image-decode#<n>`. |
| `native_chart_data.revision -> int` | Returns the data's revision, which increases by one each time the Host replaces its datasets. |
| `native_chart_data.len(dataset: String) -> int` | Returns the number of rows in `dataset` at the current revision, or 0 for an unknown dataset. |
| `native_collection.key_field -> String` | The name of the row field that holds each row's unique key. |
| `native_collection.len -> int` | The number of rows; Rhai can count the rows of a collection but not index or enumerate them. |
| `native_collection.to_string() -> String` | Returns a debug label with the row count, `NativeCollection(len=<n>)`. |
| `native_handler_ref.id -> String` | The `namespace.name` ID the Host registered this native handler under. |
| `native_signal.key -> String` | The signal's component-local key. |
| `native_signal.kind -> String` | The signal's value type: `bool`, `integer`, `float`, `optional_float`, `string` or `color`. |
| `native_text_document.identity -> String` | The document's identity; a new identity starts a new reading session, while a new revision keeps it. |
| `native_text_document.len -> int` | The length of the document's text in UTF-8 bytes. |
| `native_text_document.revision -> int` | The document's revision number, which increases with each published revision of the same identity. |
| `native_text_document.to_string() -> String` | Returns a debug label, `NativeTextDocument(<identity>, revision=<n>)`. |
| `opaque_handle.kind -> String` | The kind of Rust-owned resource the handle refers to, such as `image`. |
| `opaque_handle.to_string() -> String` | Returns a debug label, `<kind>#<id>`. |
| `span.background(color: ColorValue) -> Span` | Paints `color` behind the span's text, as for inline code. |
| `span.bold() -> Span` | Sets the span's text in bold. |
| `span.color(color: ColorValue) -> Span` | Sets the span's text color. |
| `span.italic() -> Span` | Sets the span's text in italic. |
| `span.motion(source: MotionSource) -> Span` | Declares an `opacity` motion for the span, replacing an earlier one; the span needs `with_key`, and other properties are rejected. |
| `span.typography(role: String) -> Span` | Sets the span's family and weight from the typography `role`, such as `code`; size and line height stay the paragraph's. |
| `span.with_key(key: String) -> Span` | Names the span, unique within its text, so a span `motion` can target it. |
| `subscription_handle.to_string() -> String` | Returns a debug label, `subscription#<n>`. |
| `task_handle.to_string() -> String` | Returns a debug label, `task#<n>`. |

## Operators

Arithmetic defined on runtime values.

| Call | Description |
|---|---|
| `Length * float -> Length` | Scales `length` by a non-negative `factor`; a token keeps the factor until it resolves, and a relative result must stay within 0 to 1. |
| `Length * int -> Length` | Scales `length` by a non-negative integer `factor`; a token keeps the factor until it resolves, and a relative result must stay within 0 to 1. |

## Native primitives

Rust elements a component calls through the `gpui_rhai` module with one props
map, for example `gpui_rhai::TextInputPrimitive(#{ key: "name", value: name })`.
Official components wrap them; application components may too.

### ChartPrimitive

`gpui_rhai::ChartPrimitive(props)` · `gpui_rhai.chart`. Native retained 2D chart with typed data, transforms, hover, selection, brush and zoom; `charts/chart` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `data` | array of any value (at most 10000) or chart data | required | Up to 10,000 inline row objects, or a `NativeChartData` handle from `ctx.get_native_chart_data` for large data. |
| `hidden_series` | array of string (at most 10000) | `[]` | Series keys to hide; controlled, so update it from `legend_change` payloads. |
| `key_dimension` | string or `()` | — | Inline-row field whose value keys each datum across replacement, selection, focus and motion. |
| `on_annotation_activate` | callback or `()` | — | Called with the annotation's `key` and `label` when it is clicked or activated by Enter or Space. |
| `on_brush_change` | callback or `()` | — | Called with the brushed datum keys, references and rectangle when a brush drag ends. |
| `on_legend_change` | callback or `()` | — | Called with `#{ series_key, visible }` when a legend entry is clicked; `visible` is the requested new state. |
| `on_select` | callback or `()` | — | Called with the datum reference, name and value when a data mark is clicked or activated by Enter or Space. |
| `on_zoom_change` | callback or `()` | — | Called with the proposed camera, typed `viewport` and `viewport_revision` when a wheel zoom or pan commits. |
| `pan_x` | number | `0.0` | Scalar camera offset along x in logical plot pixels; goes with `zoom`. |
| `pan_y` | number | `0.0` | Scalar camera offset along y in logical plot pixels; goes with `zoom`. |
| `selected_keys` | array of string (at most 10000) | `[]` | Datum keys drawn as selected; controlled, so update it from `select` or `brush_change` payloads. |
| `spec` | any value | required | Typed chart description: series and their `encode`, regions, axes, legend, `brush`, annotations and links. |
| `viewport` | any value | `()` | Controlled typed viewport, a cartesian axis window or a geo camera, or `()`; store `zoom_change`'s `viewport`. |
| `viewport_revision` | integer ≥ 0 | `0` | Revision of the last `zoom_change` you answered; write its `viewport_revision` back, also when rejecting it. |
| `zoom` | number 0.5–20 | `1.0` | Scalar camera zoom where 1 shows the whole domain; shorthand for `viewport` when one camera is enough. |

| Event | Payload | Description |
|---|---|---|
| `annotation_activate` | object | Emitted when an annotation is clicked or activated by Enter or Space; the payload is its `key` and `label`. |
| `brush_change` | object | Emitted when a brush drag ends; the payload lists the enclosed data and the rectangle in chart-local pixels. |
| `legend_change` | object | Emitted when a legend entry is clicked; the payload names the series and its requested visibility. |
| `select` | object | Emitted when a data mark is clicked or activated by Enter or Space; the payload identifies the datum and its value. |
| `zoom_change` | object | Emitted when a wheel zoom or a middle-button pan commits; the payload is the proposed camera and its new revision. |

### CodeViewPrimitive

`gpui_rhai::CodeViewPrimitive(props)` · `gpui_rhai.code_viewer`. Native read-only, virtualized code view with syntax highlighting, selection, search and wrapping; `components/code_viewer` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `error_style` | style | required | Style of the message that replaces the view when preparing fails, for example past a size limit. |
| `file_name` | string or `()` | — | File name, such as `service.toml`, whose extension picks the syntax when `language` does not. |
| `gutter_style` | style | required | Style of each row's line-number gutter; its font defaults to the platform monospace family. |
| `label` | string | required | Human-readable document name; the native view does not show it, so also pass it to `accessibility_label`. |
| `language` | string or `()` | — | Syntax name or extension, such as `rust`, ahead of `file_name`; unknown ones fall back to it, then plain text. |
| `line_style` | style | required | Style of each whole row, gutter and text together. |
| `loading_style` | style | required | Style of the placeholder shown while the document is first prepared in the background. |
| `on_location_activate` | callback or `()` | — | Called with the 1-based `#{ line, column }` when the text is double-clicked or Enter is pressed. |
| `search_style` | style | required | Style of the find bar that Cmd-F opens. |
| `show_line_numbers` | bool | `true` | Shows 1-based line numbers in the gutter on the first row of each line. |
| `source` | string or native document | required | The document: an inline string or a `NativeTextDocument` from `ctx.get_native_text_document`. |
| `tab_size` | integer 1–16 | `4` | Columns between tab stops; a tab expands to the next stop for display and wrapping. |
| `text_style` | style | required | Style of each row's text; its `typography`, font size and line height set the row height. |
| `wrap` | `"none"` or `"viewport"` or `"column"` | `"none"` | `none` keeps each line on one row, `viewport` wraps at the visible width, `column` at `wrap_column` characters. |
| `wrap_column` | integer 20–500 | `100` | Character column where lines wrap when `wrap` is `column`; ignored otherwise. |

| Event | Payload | Description |
|---|---|---|
| `location_activate` | object | Emitted on a double-click in the text or Enter at the selection; the payload is the 1-based `line` and `column`. |

### ColumnResizePrimitive

`gpui_rhai::ColumnResizePrimitive(props)` · `gpui_rhai.column_resize`. Native table column separator that previews a dragged width and proposes `resize`; double-click auto-fits; `components/table` uses it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `column_key` | string | required | Key of the column this handle resizes; it is `key` in the `resize` payload. |
| `column_ref` | element ref | required | Ref to the column's header cell; a drag starts from its width, and auto-fit uses it when no text was measured. |
| `max_width` | number > 0, ≤ 16384 | — | Largest width a drag or auto-fit may propose, in logical pixels; defaults to 16,384. |
| `min_width` | number > 0, ≤ 16384 | — | Smallest width a drag or auto-fit may propose, in logical pixels; defaults to 48. |
| `on_resize` | callback or `()` | — | Called with the proposed fixed width when a drag ends or a double-click auto-fits the column. |
| `signal` | signal | required | Optional-float signal for the previewed width, kept after release without `on_resize`; text measured with it as `group` sets auto-fit. |
| `source_kind` | `"fixed"` or `"percent"` or `"flex"` | required | Kind of the column's controlled width; with `source_value` it identifies the source, and a change cancels a drag. |
| `source_value` | number > 0 | required | Value of the column's controlled width; a change cancels a drag and clears the preview. |

| Event | Payload | Description |
|---|---|---|
| `resize` | object | Emitted when a drag ends or a double-click auto-fits the column; the payload is `key` and a `fixed` `width` in logical pixels. |

### DiffViewPrimitive

`gpui_rhai::DiffViewPrimitive(props)` · `gpui_rhai.diff_viewer`. Native read-only diff of two documents, unified or split, with folds, search and change navigation; `components/diff_viewer` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `context_lines` | integer 0–100 or `"all"` | `3` | Unchanged lines kept around each change; longer equal runs fold into an expandable row, and `all` never folds. |
| `error_style` | style | required | Style of the message that replaces the view when preparing fails, for example past a size limit. |
| `fold_style` | style | required | Style of the row that stands for folded unchanged lines; clicking it expands them. |
| `gutter_style` | style | required | Style of each row's line-number gutter; its font defaults to the platform monospace family. |
| `header_style` | style | required | Style of the header bar that shows `left_label` and `right_label`. |
| `left_file_name` | string or `()` | — | File name whose extension picks the left side's syntax when `left_language` does not. |
| `left_label` | string | required | Name of the left side in the header, and its file name in a unified patch copied with Option-Cmd-C. |
| `left_language` | string or `()` | — | Syntax name or extension for the left side, ahead of `left_file_name`; unknown ones fall back to it. |
| `left_source` | string or native document | required | The left document: an inline string or a `NativeTextDocument`; left and right are neutral, not old and new. |
| `line_style` | style | required | Style of each whole diff row, gutters and text together. |
| `loading_style` | style | required | Style of the placeholder shown while the comparison is first calculated in the background. |
| `mode` | `"unified"` or `"split"` | `"unified"` | `unified` interleaves both sides in one column; `split` shows left and right side by side on aligned rows. |
| `on_location_activate` | callback or `()` | — | Called with the 1-based `#{ side, line, column }` when the text is double-clicked or Enter is pressed. |
| `right_file_name` | string or `()` | — | File name whose extension picks the right side's syntax when `right_language` does not. |
| `right_label` | string | required | Name of the right side in the header, and its file name in a unified patch copied with Option-Cmd-C. |
| `right_language` | string or `()` | — | Syntax name or extension for the right side, ahead of `right_file_name`; unknown ones fall back to it. |
| `right_source` | string or native document | required | The right document: an inline string or a `NativeTextDocument`; left and right are neutral, not old and new. |
| `search_style` | style | required | Style of the find bar that Cmd-F opens; it searches both documents. |
| `show_line_numbers` | bool | `true` | Shows each side's 1-based line numbers in the gutter. |
| `status_style` | style | required | Style of the bottom status bar that shows the current change and the change-navigation keys. |
| `tab_size` | integer 1–16 | `4` | Columns between tab stops; a tab expands to the next stop for display and wrapping. |
| `text_style` | style | required | Style of each row's text; its `typography`, font size and line height set the row height. |
| `whitespace` | `"exact"` or `"ignore_changes"` or `"ignore_all"` | `"exact"` | `exact` compares lines as they are, `ignore_changes` collapses and trims whitespace, `ignore_all` drops it all. |
| `wrap` | `"none"` or `"viewport"` or `"column"` | `"none"` | `none` keeps each line on one row, `viewport` wraps at the pane width, `column` at `wrap_column` characters. |
| `wrap_column` | integer 20–500 | `100` | Character column where lines wrap when `wrap` is `column`; ignored otherwise. |

| Event | Payload | Description |
|---|---|---|
| `location_activate` | object | Emitted on a double-click in either pane or Enter at the selection; the payload is the `side` and 1-based position. |

### DragSourcePrimitive

`gpui_rhai::DragSourcePrimitive(props)` · `gpui_rhai.drag_source`. Native drag source that carries a typed payload to a `DropZonePrimitive` and reports `drag_end`; `components/drag_source` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Ignores presses and keys and removes the source from the tab order. |
| `keyboard_target` | string or `()` | — | `target_id` of the drop zone that Enter or Space on the focused source drops onto; `()` for no keyboard drop. |
| `on_drag_end` | callback or `()` | — | Called with the outcome when a started drag is dropped or cancelled, or a keyboard drop runs. |
| `operation` | `"copy"` or `"move"` | required | Whether dropping copies or moves the item; only drop zones that list it in `operations` accept the drag. |
| `payload` | any value | required | Bounded application data the drag carries; the drop zone gets it back in its `drop` payload. |
| `payload_type` | string | required | Type name of the payload; only drop zones that list it in `payload_types` accept the drag. |
| `source_id` | string | required | Stable identifier of the dragged item; it is `source_id` in the target's `drop` payload. |
| `threshold` | number 0–64 | — | Pointer movement in logical pixels before a press becomes a drag; defaults to 4. |

| Event | Payload | Description |
|---|---|---|
| `drag_end` | object | Emitted when a started drag ends or a keyboard drop runs; the payload says whether and where it was accepted, or cancelled. |

### DraggablePrimitive

`gpui_rhai::DraggablePrimitive(props)` · `gpui_rhai.draggable`. Native pointer and keyboard mover that previews an object's position and proposes one `move`; `components/draggable` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `axes` | `"both"` or `"horizontal"` or `"vertical"` | `"both"` | Directions the object moves in: `both`, `horizontal` or `vertical`. |
| `boundary_ref` | element ref | required | Ref to the container the position is local to; resizing it during a drag cancels the drag. |
| `contain` | bool | `true` | Keeps the object inside `boundary_ref`; presses are ignored while the controlled position lies outside it. |
| `disabled` | bool | `false` | Ignores presses and arrow keys and removes the object from the tab order. |
| `handle_ref` | element ref or `()` | — | Ref to a drag handle inside the object; when set, only presses inside it start a drag. |
| `keyboard_step` | number > 0 | — | Logical pixels one arrow-key press moves the object, up to 512; Shift multiplies it by four; defaults to 8. |
| `object_ref` | element ref | required | Ref to the moved object; its size bounds `contain`, and without `handle_ref` a press on it starts a drag. |
| `on_move` | callback or `()` | — | Called with the proposed `{x, y}` when a drag ends or an arrow key is pressed. |
| `snap_x` | number or `()` | — | Grid step in logical pixels the left edge rounds to, or `()` to move freely. |
| `snap_y` | number or `()` | — | Grid step in logical pixels the top edge rounds to, or `()` to move freely. |
| `threshold` | number 0–64 | — | Pointer movement in logical pixels before a press becomes a drag; defaults to 4. |
| `x` | number | required | Controlled left edge of the object in `boundary_ref`'s local logical pixels. |
| `x_signal` | signal | required | Optional-float signal that receives the previewed horizontal offset from `x` during a drag, or `()` when idle. |
| `y` | number | required | Controlled top edge of the object in `boundary_ref`'s local logical pixels. |
| `y_signal` | signal | required | Optional-float signal that receives the previewed vertical offset from `y` during a drag, or `()` when idle. |

| Event | Payload | Description |
|---|---|---|
| `move` | object | Emitted once when a drag ends or an arrow key moves the object; the payload is the next `{x, y}`. |

### DropZonePrimitive

`gpui_rhai::DropZonePrimitive(props)` · `gpui_rhai.drop_zone`. Native drop target that accepts typed payloads, paints drop feedback and proposes one `drop`; `components/drop_zone` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Takes the zone out of drag targeting, so it neither highlights nor accepts drops. |
| `on_drop` | callback or `()` | — | Called with the drop proposal when an accepted drag is released over this zone or a keyboard drop targets it. |
| `operations` | array of `"copy"` or `"move"` (at most 2) | required | Operations this zone accepts, `copy`, `move` or both; other drags cannot drop here. |
| `payload_types` | array of string (at most 32) | required | Payload types this zone accepts; a drag of another type shows the invalid highlight and cannot drop. |
| `priority` | integer | — | Rank among overlapping zones: the highest wins, then the smallest area; defaults to 0. |
| `target_id` | string | required | Stable identifier of this zone; it is `target_id` in `drop` and in the source's `drag_end`. |

| Event | Payload | Description |
|---|---|---|
| `drop` | object | Emitted when an accepted drag drops here by pointer or key; the payload is the source, its payload and operation, and window `x`, `y`. |

### IntrinsicTextMeasurePrimitive

`gpui_rhai::IntrinsicTextMeasurePrimitive(props)` · `gpui_rhai.intrinsic_text_measure`. Zero-size native element that measures one line of text for column auto-fit; `components/table` pairs it with `ColumnResizePrimitive`.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `extra_width` | number 0–16384 | — | Logical pixels added once to the measured width, such as room for a sort icon; 0 when omitted. |
| `group` | signal | required | The column's optional-float signal, shared with its `ColumnResizePrimitive`; auto-fit takes the widest measure. |
| `horizontal_padding` | length | — | Padding added on both sides of the text, in pixels or rems, such as the cell's inset; 0 when omitted. |
| `text` | string | required | Text measured as one line in the text style inherited here; place it inside the cell it measures. |

### PanZoomPrimitive

`gpui_rhai::PanZoomPrimitive(props)` · `gpui_rhai.pan_zoom`. Native pan and zoom over a Canvas that previews its transform and proposes `transform_change`; `components/pan_zoom` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `axes` | `"both"` or `"horizontal"` or `"vertical"` | `"both"` | Directions a drag and the arrow keys pan: `both`, `horizontal` or `vertical`. |
| `content_ref` | element ref | required | Ref to the transformed Canvas; wheel zoom anchors at the pointer over it, key zoom at its centre. |
| `disabled` | bool | `false` | Ignores drags, the wheel and keys and removes the surface from the tab order. |
| `keyboard_pan_step` | number 0.1–512 | — | Logical pixels one arrow-key press pans; Shift multiplies it by four; defaults to 16. |
| `keyboard_zoom_factor` | number 1.001–4 | — | Factor `+` multiplies and `-` divides the scale by, around the viewport centre; defaults to 1.2. |
| `max_scale` | number > 0 | required | Largest zoom factor the wheel and keys may reach; at least `scale`. |
| `min_scale` | number > 0 | required | Smallest zoom factor the wheel and keys may reach; at most `scale`. |
| `on_transform_change` | callback or `()` | — | Called with the proposed `{x, y, scale}` when a pan ends, a wheel zoom settles or a key pans, zooms or resets. |
| `pan_button` | `"left"` or `"middle"` | `"left"` | Mouse button that drags the view: `left` or `middle`. |
| `scale` | number > 0 | required | Controlled zoom factor, from `min_scale` to `max_scale`; 1 shows the content at its own size. |
| `scale_x_signal` | signal | required | Float signal that receives the previewed zoom factor; bind it to the Canvas `scale_x`. |
| `scale_y_signal` | signal | required | Float signal that receives the same previewed zoom factor; bind it to the Canvas `scale_y`. |
| `source_token` | string | required | Fingerprint of the controlled transform and settings; a new value cancels any gesture and shows `x`, `y`, `scale` again. |
| `source_token_signal` | signal | required | String signal that keeps the last `source_token` shown, so a new source can be told from a rerender. |
| `threshold` | number 0–64 | — | Pointer movement in logical pixels before a press becomes a pan; defaults to 4. |
| `viewport_ref` | element ref | required | Ref to the visible viewport; a pan starts only from a press inside its bounds. |
| `wheel_active_signal` | signal | required | Bool signal that is true while a trackpad wheel gesture with explicit phases is in progress. |
| `wheel_generation_signal` | signal | required | Integer signal that counts wheel bursts, so a delayed commit from an older burst is dropped. |
| `wheel_pending_signal` | signal | required | Bool signal that is true while a wheel zoom is previewed and not yet proposed. |
| `wheel_zoom` | `"off"` or `"modifier"` or `"always"` | `"modifier"` | When the wheel zooms: `off`, `modifier` (with Ctrl or Cmd held; other wheel input scrolls ancestors) or `always`. |
| `x` | number | required | Controlled horizontal pan in logical pixels, applied after scaling about the content's centre. |
| `x_signal` | signal | required | Float signal that receives the previewed horizontal pan; bind it to the Canvas `translate_x`. |
| `y` | number | required | Controlled vertical pan in logical pixels, applied after scaling about the content's centre. |
| `y_signal` | signal | required | Float signal that receives the previewed vertical pan; bind it to the Canvas `translate_y`. |

| Event | Payload | Description |
|---|---|---|
| `transform_change` | object | Emitted once when a pan ends, a wheel zoom settles or a key is pressed; the payload is the next `{x, y, scale}`. |

### RangeInputPrimitive

`gpui_rhai::RangeInputPrimitive(props)` · `gpui_rhai.range_input`. Native one-thumb range control with styled track, fill and thumb that proposes `change`; `components/slider` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks pointer and keyboard input, leaves the tab order and dims the control. |
| `fill_style` | style | required | Style of the fill from `min` to the thumb; its length along the track is set natively from the value. |
| `max` | number | required | Value at the end of the track; must be greater than `min`. |
| `min` | number | required | Value at the start of the track: the leading edge, or the bottom when vertical; must be less than `max`. |
| `on_change` | callback or `()` | — | Called with the snapped value when a drag ends or an arrow, Home or End key or an accessibility action moves it. |
| `orientation` | `"horizontal"` or `"vertical"` | required | `horizontal` runs the track in the reading direction (mirrored in RTL); `vertical` runs it bottom to top. |
| `step` | number | required | Positive increment the value snaps to, counted from `min`, and the arrow-key step; at most `max - min`. |
| `thumb_style` | style | required | Style of the thumb centered on the value; its `focus` and `disabled` states apply. |
| `track_style` | style | required | Style of the track that holds the fill and thumb; give it its size here, such as full width by 4px. |
| `value` | number | required | The selected value, snapped to `step` from `min`; controlled, so store each `change` payload or the thumb snaps back. |

| Event | Payload | Description |
|---|---|---|
| `change` | number | Emitted when a drag ends or on an arrow, Home or End key; the payload is the proposed snapped value. |

### RangeSliderPrimitive

`gpui_rhai::RangeSliderPrimitive(props)` · `gpui_rhai.range_slider`. Native two-thumb range control that keeps `low` and `high` ordered and proposes `change`; `components/range_slider` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks pointer and keyboard input, takes both thumbs out of the tab order and dims the control. |
| `fill_style` | style | required | Style of the fill between the two thumbs; its position and length are set natively from the values. |
| `high` | number | required | Upper selected value; controlled, so store `change.high` here. At most `max` and `minimum_gap` above `low`. |
| `high_label` | string | required | Accessible name of the upper thumb, such as `Maximum price`. |
| `low` | number | required | Lower selected value; controlled, so store `change.low` here. At least `min` and `minimum_gap` below `high`. |
| `low_label` | string | required | Accessible name of the lower thumb, such as `Minimum price`. |
| `max` | number | required | Value at the end of the track; must be greater than `min`. |
| `min` | number | required | Value at the start of the track: the leading edge, or the bottom when vertical; must be less than `max`. |
| `minimum_gap` | number ≥ 0 | — | Smallest allowed distance between `low` and `high`, at most `max - min`; 0 when omitted. |
| `on_change` | callback or `()` | — | Called with `#{ low, high }` when a drag ends or an arrow, Home or End key moves a thumb. |
| `orientation` | `"horizontal"` or `"vertical"` | required | `horizontal` runs the track in the reading direction (mirrored in RTL); `vertical` runs it bottom to top. |
| `step` | number > 0 | required | Increment both values snap to, counted from `min`, and the arrow-key step. |
| `thumb_style` | style | required | Style of both thumbs, each centered on its value; its `focus` and `disabled` states apply per thumb. |
| `track_style` | style | required | Style of the track that holds the fill and thumbs; give it its size here, such as full width by 4px. |

| Event | Payload | Description |
|---|---|---|
| `change` | object | Emitted when a drag ends or a key moves a thumb; the payload is the proposed `#{ low, high }`, snapped and gapped. |

### ResizableHandlePrimitive

`gpui_rhai::ResizableHandlePrimitive(props)` · `gpui_rhai.resizable_handle`. Native edge or corner handle that previews a rectangle resize and proposes `resize`; `components/resizable` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `aspect_ratio` | number or `()` | — | Width-to-height ratio a resize keeps, or `()` to resize each axis freely. |
| `boundary_ref` | element ref | required | Ref to the container the rectangle is local to; resizing it during a drag cancels the drag. |
| `contain` | bool | `true` | Keeps the rectangle inside `boundary_ref`; presses are ignored while the controlled rectangle lies outside it. |
| `disabled` | bool | `false` | Ignores presses and arrow keys and removes the handle from the tab order. |
| `handle` | `"n"` or `"s"` or `"e"` or `"w"` or `"ne"` or `"nw"` or `"se"` or `"sw"` | required | Physical edge (`n`, `s`, `e`, `w`) or corner (`ne`, `nw`, `se`, `sw`) this handle drags; it does not flip in RTL. |
| `handle_ref` | element ref or `()` | — | Ref to a decorative handle node; presses and hover over its visible bounds count as the handle's own. |
| `height` | number > 0 | required | Controlled height of the rectangle in logical pixels. |
| `height_signal` | signal | required | Optional-float signal that receives the previewed height during a drag, or `()` when idle. |
| `keyboard_step` | number > 0, ≤ 512 | — | Logical pixels one arrow-key press moves the dragged edge; Shift multiplies it by four; defaults to 8. |
| `line` | bool | `true` | Paints the native handle mark (a square on a corner); set `false` when a handle node draws its own. |
| `line_inset` | number 0–256 | — | How far the native line stops short of each end of the handle, in logical pixels; defaults to 4. |
| `max_height` | number > 0, ≤ 16384 | — | Largest height a resize may propose, in logical pixels; defaults to 16,384. |
| `max_width` | number > 0, ≤ 16384 | — | Largest width a resize may propose, in logical pixels; defaults to 16,384. |
| `min_height` | number > 0, ≤ 16384 | — | Smallest height a resize may propose, in logical pixels; defaults to 24. |
| `min_width` | number > 0, ≤ 16384 | — | Smallest width a resize may propose, in logical pixels; defaults to 24. |
| `on_resize` | callback or `()` | — | Called with the proposed `{x, y, width, height}` when a drag ends or an arrow key is pressed. |
| `state_signal` | signal or `()` | — | String signal that receives `idle`, `hover`, `drag`, `focus` or `disabled`, for a handle node's `signal_style`. |
| `width` | number > 0 | required | Controlled width of the rectangle in logical pixels. |
| `width_signal` | signal | required | Optional-float signal that receives the previewed width during a drag, or `()` when idle. |
| `x` | number | required | Controlled left edge of the rectangle in `boundary_ref`'s local logical pixels. |
| `x_signal` | signal | required | Optional-float signal that receives the previewed horizontal offset from `x` during a drag, or `()` when idle. |
| `y` | number | required | Controlled top edge of the rectangle in `boundary_ref`'s local logical pixels. |
| `y_signal` | signal | required | Optional-float signal that receives the previewed vertical offset from `y` during a drag, or `()` when idle. |

| Event | Payload | Description |
|---|---|---|
| `resize` | object | Emitted once when a drag ends or an arrow key is pressed; the payload is the next rectangle, shaped like the controlled one. |

### RotatablePrimitive

`gpui_rhai::RotatablePrimitive(props)` · `gpui_rhai.rotatable`. Native rotation around a pivot that previews a Canvas angle and proposes `rotate`; `components/rotatable` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `angle` | number | required | Controlled rotation in degrees, clockwise on screen; it is wrapped into 0 to 360. |
| `angle_signal` | signal | required | Float signal that receives the previewed angle in degrees; bind it to the Canvas `rotate`. |
| `content_ref` | element ref | required | Ref to the rotated Canvas; the pivot is local to it, and moving or resizing it during a drag cancels the rotation. |
| `disabled` | bool | `false` | Ignores presses and keys and removes the handle from the tab order. |
| `keyboard_step` | number 0.1–180 | — | Degrees one arrow-key press turns when `snap` is unset; Shift multiplies it by four; defaults to 5. |
| `on_rotate` | callback or `()` | — | Called with the proposed angle when a rotation drag ends or an arrow key or Home is pressed. |
| `pivot_x` | number | required | Horizontal position of the pivot in `content_ref`'s local logical pixels. |
| `pivot_y` | number | required | Vertical position of the pivot in `content_ref`'s local logical pixels. |
| `snap` | number or `()` | — | Step in degrees, up to 360, that angles round to and keys turn by; `()` turns snapping off. |
| `source_token` | string | required | Fingerprint of the controlled angle, pivot and settings; when it changes, the preview returns to `angle`. |
| `source_token_signal` | signal | required | String signal that keeps the source last shown or proposed, so a rerender with the same source reads as a rejection. |
| `threshold` | number 0–64 | — | Pointer movement in logical pixels before a press becomes a rotation; defaults to 4. |
| `x_signal` | signal | required | Float signal that receives the horizontal shift that keeps the pivot in place while the Canvas turns about its centre. |
| `y_signal` | signal | required | Float signal that receives the vertical shift that keeps the pivot in place while the Canvas turns about its centre. |

| Event | Payload | Description |
|---|---|---|
| `rotate` | number | Emitted once when a rotation drag ends or an arrow key or Home turns it; the payload is the next angle in degrees, 0 to 360. |

### SelectionAreaPrimitive

`gpui_rhai::SelectionAreaPrimitive(props)` · `gpui_rhai.selection_area`. Native click, range and marquee selection of Canvas objects that proposes `selection_change`; `components/selection_area` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_key` | string or `()` | required | Controlled key of the active target that arrow keys move from and Space toggles, or `()`. |
| `anchor_key` | string or `()` | required | Controlled key a Shift range extends from, or `()`. |
| `canvas_ref` | element ref | required | Ref to the Canvas the targets live on; pointer positions go through its inverse transform, pan, zoom and rotation included. |
| `disabled` | bool | `false` | Ignores presses and keys, cancels a running marquee and removes the area from the tab order. |
| `marquee` | `"intersect"` or `"enclose"` | `"intersect"` | `intersect` selects targets the marquee touches; `enclose` only targets entirely inside it. |
| `multiple` | bool | `true` | Allows more than one selected key; `false` turns off toggle, range and additive marquee selection. |
| `on_selection_change` | callback or `()` | — | Called with the proposed selection on a click, a marquee release, or an arrow, Home, End or Space key. |
| `selected_keys` | array of string (at most 10000) | required | Controlled keys of the selected targets; the caller stores the `selection_change` payload and passes it back. |
| `targets` | array of map of any value (at most 10000) | required | Selectable `{key, x, y, width, height, disabled}` rectangles in Canvas-local logical pixels; list order sets range and stacking. |
| `threshold` | number 0–64 | — | Pointer movement in logical pixels before a press becomes a marquee; defaults to 4. |

| Event | Payload | Description |
|---|---|---|
| `selection_change` | object | Emitted on a click, a marquee release or a selection key; the payload is the next `selected_keys`, `active_key` and `anchor_key`. |

### SortableItemPrimitive

`gpui_rhai::SortableItemPrimitive(props)` · `gpui_rhai.sortable_item`. Native drag grip for one keyed item that previews insertion and proposes `reorder`; `components/sortable` and `components/tab_bar` use it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `collection_id` | string | required | Drag scope: items take drops only from items with the same id, and a virtual list pins the dragged row by it. |
| `direction` | `"vertical"` or `"horizontal"` or `"grid"` | required | `vertical` splits the item into top and bottom drop halves; `horizontal` and `grid` into left and right halves. |
| `disabled` | bool | `false` | Stops this item from being dragged or moved by key; it still takes drops from other items. |
| `first_key` | string | required | Key of the first item in the list; Alt+Home moves this item before it. |
| `item_key` | string | required | Stable key of this item; it is the `source_key` or `anchor_key` of `reorder` and the `tap` payload. |
| `item_ref` | element ref or `()` | — | Ref to the element whose bounds hold the drop halves and the drag highlight; defaults to this primitive's bounds. |
| `last_key` | string | required | Key of the last item in the list; Alt+End moves this item after it. |
| `list_id` | string | required | Identifier of the list the item belongs to; with `item_key` it names the item's native interaction. |
| `next_key` | string or `()` | required | Key of the item after this one, or `()` for the last; drops next to it are no-ops and Alt+Down moves after it. |
| `on_reorder` | callback or `()` | — | Called with the proposed move when an item is dropped on this one, or this item moves by Alt+Arrow, Home or End. |
| `on_tap` | callback or `()` | — | Called with `item_key` when a press is released before it became a drag; needs `tap`. |
| `previous_key` | string or `()` | required | Key of the item before this one, or `()` for the first; drops next to it are no-ops and Alt+Up moves before it. |
| `source_index` | integer ≥ 0 or `()` | — | Index of the item in a virtual collection, so its row stays realized while dragged out of view; `()` otherwise. |
| `source_item` | any value or `()` | — | The item's value in a virtual collection; with `source_index` it identifies the dragged row to keep realized. |
| `take_focus` | bool | `true` | Whether a press focuses the item; `false` leaves focus alone, for items inside a control with one tab stop. |
| `tap` | bool | `false` | Whether a press released before the drag threshold emits `tap`, so the item can be clicked as well as dragged. |
| `threshold` | number 0–64 | — | Pointer movement in logical pixels before a press becomes a drag; defaults to 4. |

| Event | Payload | Description |
|---|---|---|
| `reorder` | object | Emitted when a drop or an Alt-key move changes the order; the payload moves `source_key` `before` or `after` `anchor_key`. |
| `tap` | string | Emitted when `tap` is on and a press is released before it became a drag; the payload is `item_key`. |

### SplitResizePrimitive

`gpui_rhai::SplitResizePrimitive(props)` · `gpui_rhai.split_resize`. Native separator between two panels that previews a split ratio and proposes `resize`; `components/split_pane` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Ignores presses and hover, so the handle cannot be dragged. |
| `group_ref` | element ref | required | Ref to the element holding both panels and the handle; its length along the axis is what the ratio divides. |
| `handle_ref` | element ref or `()` | — | Ref to a decorative handle node; presses and hover over its visible bounds count as the handle's own. |
| `line` | bool | `true` | Paints the native handle mark (a square on a corner); set `false` when a handle node draws its own. |
| `line_inset` | number 0–256 | — | How far the native line stops short of each end of the handle, in logical pixels; defaults to 4. |
| `max_start` | number 0–16384 | — | Largest start-panel length a drag may propose, in logical pixels; defaults to 16,384. |
| `min_end` | number 0–16384 | — | Smallest end-panel length a drag may leave, in logical pixels; it wins when both minimums cannot fit. |
| `min_start` | number 0–16384 | — | Smallest start-panel length a drag may propose, in logical pixels; defaults to 0. |
| `on_resize` | callback or `()` | — | Called with the proposed start-panel ratio when a drag ends. |
| `orientation` | `"horizontal"` or `"vertical"` | required | `horizontal` resizes the start panel's width (panels side by side); `vertical` resizes its height. |
| `signal` | signal | required | Optional-float signal that receives the start-panel length to preview, in logical pixels, or `()` for none. |
| `source_ratio` | number 0–1 | required | Controlled start-panel share of the group's length, 0 to 1; the caller stores the `resize` payload here. |
| `start_ref` | element ref | required | Ref to the start panel; its length when a drag begins is where the preview starts. |
| `state_signal` | signal or `()` | — | String signal that receives `idle`, `hover`, `drag`, `focus` or `disabled`, for a handle node's `signal_style`. |

| Event | Payload | Description |
|---|---|---|
| `resize` | number 0–1 | Emitted once when a drag that moved ends; the payload is the next start-panel ratio, 0 to 1. |

### TextInputPrimitive

`gpui_rhai::TextInputPrimitive(props)` · `gpui_rhai.text_input`. Native single-line text field with IME, selection, clipboard and undo; `components/input` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `autofocus` | bool | `false` | Focuses the field once when it first mounts, unless it is disabled. |
| `disabled` | bool | `false` | Blocks focus, clicks and edits and dims the field. |
| `on_blur` | callback or `()` | — | Called when the field loses keyboard focus. |
| `on_change` | callback or `()` | — | Called with the full new text after each edit, paste, cut, undo, redo or IME composition step. |
| `on_focus` | callback or `()` | — | Called when the field gains keyboard focus. |
| `on_submit` | callback or `()` | — | Called with the current text when Enter is pressed in an enabled field. |
| `placeholder` | string or `()` | — | Hint text shown dimmed while `value` is empty. |
| `read_only` | bool | `false` | Keeps focus, selection and copy but blocks every edit. |
| `typography` | string | required | Theme typography role, such as `control`, that sets the font family, size, weight and line height. |
| `value` | string | required | The field's text; controlled, so store each `change` payload here or the next render restores the old text. |

| Event | Payload | Description |
|---|---|---|
| `blur` | none | Emitted when the field loses keyboard focus; the payload is `()`. |
| `change` | string | Emitted after each edit, including IME composition, undo and redo; the payload is the full new text. |
| `focus` | none | Emitted when the field gains keyboard focus; the payload is `()`. |
| `submit` | string | Emitted when Enter is pressed in an enabled field; the payload is the current text. |

### TextareaPrimitive

`gpui_rhai::TextareaPrimitive(props)` · `gpui_rhai.textarea`. Native multi-line text field with wrapping, auto-growing rows, IME, selection and undo; `components/textarea` wraps it.

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `autofocus` | bool | `false` | Focuses the field once when it first mounts, unless it is disabled. |
| `caret_style` | style | required | Its background, or else its text color, colors the caret; theme `accent` when it sets neither. |
| `disabled` | bool | `false` | Blocks focus, clicks and edits and dims the field. |
| `max_length` | integer 0–1000000 or `()` | — | Most graphemes the text may hold; typing, pasting and IME commits are cut to fit, and `()` means no limit. |
| `max_rows` | integer 1–1000 | `8` | Most visible rows when `rows` is unset, at least `min_rows`; longer text scrolls vertically. |
| `min_rows` | integer 1–1000 | `3` | Fewest visible rows when `rows` is unset; the field grows with its wrapped text from here up to `max_rows`. |
| `on_blur` | callback or `()` | — | Called when the field loses keyboard focus. |
| `on_change` | callback or `()` | — | Called with the full new text after each edit, newline, paste, cut, undo, redo or committed IME composition. |
| `on_focus` | callback or `()` | — | Called when the field gains keyboard focus. |
| `placeholder` | string or `()` | — | Hint text shown in the `placeholder_style` color while `value` is empty. |
| `placeholder_style` | style | required | Its text color, or else its background, colors the placeholder; theme `text_muted` when it sets neither. |
| `read_only` | bool | `false` | Keeps focus, selection and copy but blocks every edit. |
| `rows` | integer 1–1000 or `()` | — | Fixed number of visible rows that overrides `min_rows` and `max_rows`; `()` lets the height follow the text. |
| `scroll_style` | style | required | Its background, or else its text color, fills the scrolling text viewport; transparent when it sets neither. |
| `selection_style` | style | required | Its background, or else its text color, fills selected text; a translucent theme `accent` when it sets neither. |
| `typography` | string | required | Theme typography role, such as `body`, that sets the font family, size, weight and row height. |
| `value` | string | required | The field's text; controlled, so store each `change` payload here; more than `max_length` graphemes is an error. |

| Event | Payload | Description |
|---|---|---|
| `blur` | none | Emitted when the field loses keyboard focus; the payload is `()`. |
| `change` | string | Emitted after each edit and when IME composition commits, not during it; the payload is the full new text. |
| `focus` | none | Emitted when the field gains keyboard focus; the payload is `()`. |
