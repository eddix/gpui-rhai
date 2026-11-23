# gpui-rhai User Guide

This guide is for application authors and coding agents building GPUI interfaces
with gpui-rhai. It explains the mental model, the normal workflow, and the
boundary between Rhai UI code and a Rust GPUI host.

gpui-rhai is not a binding that exposes arbitrary GPUI objects to scripts. Rhai
owns declarative UI composition; Rust owns GPUI, platform mechanisms, trusted
services, and performance-sensitive native paths.

## 1. Mental model

```text
Rust GPUI host
├─ Window, App, Context, async work and trusted services
├─ ScriptViewHost: shared overlay/focus domain for embedded views
└─ gpui-rhai runtime
   ├─ Rhai Engine + restricted module resolver
   ├─ lifecycle, typed state, stores, themes and locales
   ├─ retained UiNode tree and transactional reconciliation
   └─ GPUI renderer and native primitives
      └─ Input, Textarea, overlays, scrolling, IME, Canvas, etc.

Rhai source
├─ view(ctx) -> UiNode
├─ formal source components imported from ui/components/
├─ semantic Style and theme tokens
└─ generation-bound callbacks and effect descriptors
```

The important rules are:

1. Rhai returns declarative `UiNode` snapshots. It never receives a GPUI
   `Window`, `App`, `Context`, `Div`, or `AnyElement`.
2. Rust validates a candidate before committing it. Failed events and reloads
   retain the previous AST, state, callbacks, and visible tree.
3. Official components are copied Rhai source. Applications may read and edit
   them; gpui-rhai does not depend on `gpui-component`.
4. Native Rust remains a first-class path. Use it for host integration,
   privileged operations, high-frequency work, or a mechanism scripts should
   compose rather than implement.

When a Rhai-authored shell must wrap an existing Host element or mounted view,
register it with `HostSlotRegistry`. The script receives only an opaque,
styleable box through `gpui_rhai::HostSlot`; ownership, events, suspension, and
disposal stay on the Rust side.

See [Architecture](docs/architecture.md) for the complete runtime design.

## 2. Start a project

This runtime line requires Rust 1.99 or newer. Run `rustc --version` in the
application directory to check the compiler Cargo will actually use. A
toolchain selected inside the SDK checkout does not select it for another
project; keep the application's toolchain configuration deliberate.

Install the CLI from crates.io. Use the CLI of the runtime line you build
against; the CLI, runtime and registry are released together:

```text
cargo install gpui-rhai-cli --locked
```

Then, from a Cargo application root (`cargo new my-app` for a new one):

```text
gpui-rhai init --profile productivity
gpui-rhai add button input form_field
gpui-rhai check
gpui-rhai dev
```

Inside this repository, contributors can invoke the matching workspace CLI with:

```text
cargo run -p gpui-rhai-cli -- --root /path/to/app init
```

`init` creates the normal source tree; `add` fills `components/` and `assets/`
(`gpui-rhai add lucide/<name>` installs a [Lucide icon](docs/assets.md#lucide)):

```text
ui/
├─ app.toml
├─ main.rhai
├─ theme.rhai        # full design preset or alternate palette
├─ tokens.rhai       # shared geometry, type, motion and component recipes
├─ styles.rhai       # application-wide part styles for official components
├─ profile.rhai      # with --profile: the design rules `check` reports
├─ themes/
├─ fonts/            # native UI faces; declaring a family does not install it
├─ licenses/         # font and icon license notices
├─ locales/
├─ components/
└─ assets/

.gpui-rhai/
├─ manifest.toml
└─ baselines/        # pristine installed component source
```

The files under `ui/` belong to the application. The files under
`.gpui-rhai/baselines/` let the CLI distinguish local edits from later bundled
source updates. `init` replaces the hello-world `src/main.rs` that `cargo new`
writes; any other `src/main.rs` stays, and `init` writes
`gpui-rhai-host-snippet.rs` for deliberate integration.

`init` sets the crates.io dependency's minimum to the CLI's complete version,
because installed sources can use APIs added in a patch release. For example,
a 0.4.3 CLI writes:

```toml
gpui-rhai = { version = "0.4.3", features = ["dev-reload"] }
```

This is a normal Cargo caret requirement, not an exact pin: subsequent
compatible patches remain eligible. The build helper and offscreen
dev-dependency use the same minimum. `init` preserves dependencies already
declared by the application; keep those aligned deliberately when upgrading.

To upgrade an existing application, keep `gpui-rhai`, `gpui-rhai-registry`
and `gpui-rhai-cli` on one version, run `gpui-rhai update`, review its
three-way source merges, then run `gpui-rhai check`. The
[release and upgrade index](docs/releases/README.md) lists each version's
migration; [0.2.0](docs/releases/0.2.0.md) moves to Runtime API 3 and adds the
token base.

For runtime development, replace it temporarily with the checkout you are
testing:

```toml
gpui-rhai = { path = "/path/to/gpui-rhai/crates/gpui-rhai", features = ["dev-reload"] }
```

For another machine testing an unreleased commit, pin the exact revision rather
than silently following a moving branch:

```toml
gpui-rhai = { git = "https://github.com/eddix/gpui-rhai", rev = "<commit>", features = ["dev-reload"] }
```

Disable `dev-reload` in production builds unless source watching is an explicit
product requirement.

Useful commands:

```text
gpui-rhai add combobox select date_picker
gpui-rhai diff
gpui-rhai update
gpui-rhai metadata
gpui-rhai skills .claude/skills
gpui-rhai theme-studio
gpui-rhai gallery
```

- `add` installs requested components and their source dependencies.
- `update` recomputes the target component dependency/asset graph, installs new
  transitive requirements, and three-way merges component updates, the token
  base and the bundled palettes ([source updates](docs/source-updates.md)). The
  complete plan is staged before any target is replaced; it never silently
  overwrites application-owned source.
- `check` validates scripts, schemas, themes and known calls (including calls
  into the application's own modules), runs one complete headless first frame,
  and with a profile reports design-rule warnings in application source (not in
  installed official components).
- `metadata` emits component schemas, snippets, and Rhai language-server
  definitions from the actual installed APIs. The definitions,
  `.gpui-rhai/editor/definitions/gpui_rhai.d.rhai`, are in Rhai's definition
  format: the Rhai language server (`rhaiscript/lsp`, experimental) parses them
  without errors and resolves calls to the functions they declare.
- `skills <directory>` writes the agent skills `gpui-rhai` and
  `gpui-rhai-design` for this CLI's version: `.claude/skills` for Claude Code,
  `.agents/skills` for Codex, Cursor and other agents. Run it again after an
  upgrade; it replaces the files it wrote.
- `theme-studio [path]` opens the theme editor and complete component specimen.
- `gallery` opens the Gallery: every official component in both densities,
  with its source.

## 3. Your first Rhai view

```rhai
import "components/button" as button;
import "components/label" as label;

fn state_schema() {
    #{ fields: #{
        count: #{
            schema: #{ type: "integer", min: 0 },
            "default": #{ type: "integer", value: 0 },
        },
    } }
}

fn increment(ctx, payload) {
    ctx.set_state("count", ctx.get_state("count") + 1);
}

fn view(ctx) {
    column([
        label::Label(#{
            text: "Counter",
            description: "State remains owned by the mounted Rhai view",
        }),
        text(`${ctx.get_state("count")}`)
            .with_style(style().typography("display")),
        button::Button(#{
            text: "Increment",
            on_click: Fn("increment"),
        }),
    ]).with_style(
        style()
            .width(px(320))
            .padding(theme_spacing("lg"))
            .gap(theme_spacing("sm"))
            .background(theme_color("surface"))
    )
}
```

Only `view(ctx)` is required. `state_schema`, `init(ctx)`, `suspend(ctx)`,
`resume(ctx, elapsed_ms)`, and `dispose(ctx)` are optional.

Do not perform effects in `view`. Rendering may be retried, rolled back, or run
because a dependency changed. Use callbacks, effects, capabilities, tasks, or
subscriptions for effectful work.

Imported Rhai modules are declarations, not miniature applications. Their
top-level statements may contain imports, named functions, literal constants,
and direct component declarations. Mutable globals, control flow, arbitrary
calls, and computed registration are rejected before evaluation. Put mutable UI
state in `UiContext` and start work through lifecycle/event APIs.

Rhai compilation validates syntax, not every dynamic overload. The
`gpui-rhai check` also lints known calls in every branch: names, arities, and
literal arguments against the native parameter types, so
`style().flex_shrink(0.0)` fails as `` `flex_shrink` takes (bool), not
(float)``. It executes the real initial lifecycle
when no Host capability context is required. If a declared capability is used
by `init`, the CLI performs static validation but deliberately skips that init
and any initial View that may depend on its state; the summary says which level
ran. Use an application-owned fixture with real handlers for that remaining
validation. Event branches still need representative runtime value types. Rhai
maps are key-sorted maps, not insertion-ordered records, and scripts do not
define Rust-like struct or class types.

Functions and custom primitives the Host registers itself are invisible to
`check` until `ui/app.toml` declares them under `[host]`, for example
`functions = ["open_file/1", "omb::status/0"]` and `primitives = ["OmbChart"]`
for `gpui_rhai::OmbChart`. When the scripts call one of them or
`gpui_rhai::HostSlot`, `check` validates the calls statically and skips the
headless first frame, and its summary names them. See
[Host extensions in `check`](docs/capabilities.md#host-extensions-in-check).

## 4. State and controlled components

State field defaults are plain Rhai values, typed by the field's schema:

```rhai
fn state_schema() { #{ fields: #{
    open: #{ schema: #{ type: "bool" }, "default": false },
    ratio: #{ schema: #{ type: "float" }, "default": 1 },
    theme: #{ schema: #{ type: "map", values: #{ type: "string" } },
        "default": #{ family: "Default", name: "Dark" } },
    picked: #{ schema: #{ type: "optional", value: #{ type: "string" } }, "default": () },
} } }
```

An integer where the schema reads a float becomes one, so `1` above is `1.0`.
The serialized `UiValue` form, `#{ type: "integer", value: 0 }` with a `type`
descriptor at every level, still works and takes precedence: a map of only
`type`, naming a `UiValue` type, and `value` reads as that form. A plain map
with exactly those two keys must therefore be written tagged. Invalid
defaults report the owning state field.

State is declared, schema-checked, and scoped to a stable component instance.

```rhai
fn set_name(ctx, value) { ctx.set_state("name", value); }

input::Input(#{
    key: "profile-name",
    label: "Name",
    value: ctx.get_state("name"),
    placeholder: "Ada Lovelace",
    on_change: Fn("set_name"),
})
```

The component is controlled: the prop is the business value, while the native
Input entity retains only interaction state such as focus, selection, marked
text, undo history, and caret scrolling.

For a short inline field, set `intrinsic_width: true` to prefer the actual
shaped text width (the placeholder when empty). Give it a maximum width
inside a row so longer text leaves room for its neighbors. An explicit
width and the parent's layout still take precedence. Leave it off for
ordinary forms whose fields should not resize as the user types. Try
**Input → Fit to text** in the Gallery.

Best practices:

- Give every stateful or interactive component a stable semantic `key`.
- Store business state in the Rhai state schema or an explicit Host store.
- Treat callback payloads as the component's declared event schema; do not
  infer payload shapes from labels or visual implementation.
- Keep one source of truth. Do not mirror the same value in a Rhai state field,
  a Rust store, and an uncontrolled native widget.
- Use nested state paths for bounded, schema-checked access, but remember that
  local state still dirties its owning component; the path does not create a
  smaller component boundary. App/window store path accessors additionally
  provide exact nested dependency tracking. Ordinary Rhai Map indexing is not
  transparently observable.

Compatible state survives keyed rerenders and successful hot reloads.
Incompatible schema changes reset only the affected field to its default.

### 4.1 Drafting and rich choices

`patterns/composer::Composer` is a controlled composition over native Textarea:

```rhai
import "patterns/composer" as composer;

composer::Composer(#{
    key: "prompt", label: "Message", value: ctx.get_state("draft"),
    busy: ctx.get_state("sending"), can_submit: true,
    on_change: Fn("draft_changed"), on_submit: Fn("submit_draft"),
    on_stop: Fn("stop_request"),
})
```

Import `patterns/composer`, then declare the draft/sending fields and named
handlers in the owning component. `on_submit` receives the full draft; the
application decides whether to send, clear or retain it. `on_stop` is a
cancellation request, not an SDK task or network operation. Busy disables Send
while native drafting, selection, IME and undo remain available. `can_submit`,
`disabled` and `read_only` are separate policies; attachment/model controls in
`leading` remain application-owned.

Composer defaults to `submit_key: "enter"`: Enter submits and Shift+Enter
inserts a native newline. `"mod_enter"` uses Cmd+Enter on macOS or Ctrl+Enter
elsewhere, leaving Enter as a newline. Textarea exposes the same optional
`submit_key` / `on_submit` contract; unset preserves ordinary multiline Enter.
IME confirmation never submits. Counts and `max_length` use graphemes, not
UTF-8 bytes; `min_rows`/`max_rows` control growth before internal scrolling.

`patterns/model_picker::ModelPicker` can be used as a Composer's `leading`
control. Give it application-provided `models` with stable `id`s and display
names. Optional ratings and latency/price are descriptive and do not affect
selection. To own the settings, pass `value: #{model:"auto", thinking:false,
search:false, effort:"medium"}` and accept or reject the proposed full value
in `on_change`. The separate `open`/`on_open_change` pair controls only
disclosure; both pairs are local if the value prop is omitted. Prefer
`placement: "top"` for an editor anchored near the bottom of a window. The
picker does not fetch a model list, execute a request, or modify a draft.

`components/message_actions::MessageActions` attaches controls to one reply.
Supplying `text` enables the SDK's direct-input, after-commit clipboard
path. `on_copy` reports the version, not the text. `versions` is a count and
`version` is a zero-based accepted index; omit the index for local navigation
that follows newly appended versions. The app owns regeneration and speech,
using `regenerating` and `speaking` to report their state. The feedback panel
emits `#{version, kind, reason}` with a null kind on undo and an optional
reason following a downvote. Choose a stable instance key per message;
reusing the key transfers local vote and version state.

`components/reaction_bar::ReactionBar` supports `like`, `comment`,
`repost`, and `share` in the requested order. With `reactions` supplied,
the app accepts or rejects each `on_react` proposal (`#{id,count,active}`);
omitting it lets the bar own its counts. The count is compact on screen but
exact in the accessible description. Neither an accepted proposal nor a
local toggle posts, comments, or shares anything on the application's behalf.

`CitedAnswer` accepts literal
`segments: [#{text:"The test passed.", cite:["report/7"]}]` and ordered
`sources` with unique IDs, titles and site names. Set `paragraph: true` to
start a paragraph. Alternatively, give it a Markdown string or a Host-owned
document as `source` and write links such as `[1](gpui-cite:1)`; the number
addresses the 1-based position in `sources`. Other Markdown links emit
`on_link`. Previewing does not open anything. The separate Open source
action emits `#{id,url}`; validate the destination in the application.
Pass `preview` (empty string for closed) to own which preview is shown.
Quote `highlight` phrases are literal; without them, a structured answer
can highlight words shared with the citing statements.

`AIAnswer` uses that same evidence surface with a question, streaming
status, retry, copy, feedback and follow-ups. Pass a new immutable native
document revision as `source` while `status: "writing"`; keep its identity
stable. `on_stop` asks your application to cancel the producer. Report
`status: "stopped"` with the last accepted source to keep the partial reply.
No text is generated or fetched by the component. To let Copy use text
from an opaque native document, supply `copy_text` explicitly.

`ThinkingLoader` is the lightweight activity cue used by `AIAnswer`. Supply
ordered `phases` and a zero-based `phase` as your work changes, and set
`done` or `stopped` from the Host. `elapsed_ms` formats a completion
caption from an application-owned duration. `autoplay` and `repeat` are
for visual demos: they only rotate labels when no controlled phase is
supplied, and never complete business work. Use `variant: "plain"` inside
an existing surface.

`ThinkingState` shows **public work steps** with stable IDs and optional
sources, not model reasoning or a hidden trace. The app controls `status`,
the steps already received, and elapsed time. Set `extra: true` on a source
to show it behind the step's disclosure button. `on_open_source` reports
the step, source ID and opaque `href`; the component does not fetch or open
it. `open` and `on_open_change` optionally control whether the work summary
is expanded, independently of the actual run.

`patterns/run_controls::RunControls` composes a transcript, accepted
queue, native editor, context meter and run actions. Keep `key` stable to
retain the editor; set a new `run_id` for a new run or a retry. The transcript
follows updates only while the reader is at the end; Home and End navigate
its focused viewport. A new run starts with a new transcript anchor.
`log_height` sets the native viewport height, independently of the editor.

Pass the accepted `draft` and store the `on_change` proposal. Enter sends
outside a running task and steers while running. Alt+Enter queues while
running, and Shift+Enter inserts a newline. All four draft events carry
`#{run,text}`, and the Host alone decides whether to clear the draft. Stop,
resume, retry and dismiss carry the run ID, so reject requests that no
longer match your current run. Queue removal carries `#{run,id}`. A
steering note's `at` is a Unicode scalar offset into the accepted literal
answer; pass notes in ascending offset order.

Mode and permission are values the application can choose to accept or
reject, not execution permissions granted by this UI. The first Escape
closes the open permission menu; Escape within the run controls can then
request stop. Pending native IME composition does not send, queue, or stop
work. For other native editors that need alternate submit, Textarea's
optional `on_alt_submit` opts in without changing ordinary Enter.

`patterns/prompt_library::PromptLibrary` lets users search literal
templates and fill `{name}` variables. Variables use letters, digits,
underscore and hyphen, expand once, and retain drafts per template ID.
`variables` can supply a `default` and a description. Inline preview
markers focus the corresponding native editors. Copy can preserve unfilled
placeholders; Use is enabled only when each required value is nonblank.
An expanded prompt over 64,000 UTF-8 bytes cannot be used or copied.
`on_use` returns `#{id,text}`. Put that text in your own draft or other
application state; the library never sends it, invokes a model or evaluates it.

`patterns/integration_picker::IntegrationPicker` browses metadata supplied
by the application. `value` can control which integration is selected;
`connected` is the accepted list of IDs. `on_toggle` proposes
`#{id,connected}` and does not make a connection. Group placement stays
stable during a toggle; changing the search recomputes the grouping. In
the search field, Enter toggles the accepted visible choice. If filtering
hides that choice, Enter proposes a new selection first so it cannot modify
an unseen integration. Escape clears the search before requesting close.
`on_explore` returns an ID, not a URL to open.

`patterns/integration_settings::IntegrationSettings` edits a sync status,
site domain and time zone. Pass accepted `value:#{enabled,site,zone}`, an
application-owned `revision`, and the available `zones:[#{id,label}]`.
`on_save` proposes `#{integration,revision,site,zone}`; check the revision
before accepting it and publish the new values on success. Set `saving`
while the request is in flight and `save_message` to report rejection. The
draft remains available after rejection or pausing sync. A conflicting Host
update requires a new choice before saving. Site entries are normalized to
ASCII DNS hostnames (including IDNA); paths, ports and IP addresses are not
accepted. This performs no DNS lookup and grants no network permission.

The API key controls are separate. `on_generate_key`, `on_copy_key`,
and `on_reveal_key` carry a nonsecret `integration` and `version`, plus
`visible` for reveal. The Host validates each action and owns the secret,
including any clipboard write. Pass only status and version to the form.
If you use `credential_preview`, mount a trusted native reveal surface and
authorize it with `credential_visible`; do not put actual credentials in a
Rhai text node, toast, script state or save event. The Gallery uses a
fabricated key and an in-memory Host only.

`patterns/comment_thread::CommentThread` displays accepted messages in
order. Pass a stable `id` for each message, a caller-formatted `time` if
needed, and `status`/`details` for an expandable result. `thinking`
shows accepted assistant activity; it does not start work. The native
transcript follows new messages only while at the tail.

`patterns/comments_panel::CommentsPanel` adds local filters and sorting
by caller-provided `timestamp`. Both components render `draft` as
accepted text, emit `on_change` as it is edited and propose trimmed text
via `on_send`. Keep it until you accept the send, or leave it unchanged
with `message` on failure. Set `sending` during an in-flight reply to
avoid duplicate submission.
In CommentsPanel, supply `posted_id` for the viewer's newly accepted
comment to reveal it in the all-comments, newest-first view. Leave it
unchanged for other incoming comments.

The panel's `on_resolve` sends `#{id,revision,resolved}`; `on_edit`
sends `#{id,revision,action}` for `run` or `undo`. Look up the record by
ID, check its revision and authorize the action in your Host. Publish
accepted `resolved` and `edit_state` afterwards. A typed `/edit` is
display and classification only: unless the Host supplies an edit
state and callback, it cannot expose a run action. The component does
not execute comment text or perform undo itself.

`patterns/writing_assistant::WritingAssistant` combines welcome suggestions,
a native multiline editor and a following conversation. The Host supplies
`conversation`, `revision`, accepted messages and `draft`. Every proposal
includes that conversation and revision. On reset, publish a different
conversation ID and discard any late response for the old ID. Use the same
message ID for successive text updates to one assistant answer. Enter sends;
Shift+Enter inserts a newline. Waiting and streaming prevent a second send.

For dictation, supply optional `speech` state and `on_dictate`. Check the
`lease` on stop and discard callbacks from an obsolete take. Interim text
is only a preview; present the final transcript as a normal editable draft
before sending. The SDK does not open the microphone, request OS permission,
recognize speech, send the text, or call a model. Gallery explicitly
simulates those Host states without using a microphone or network.

`patterns/approval_card::ApprovalCard` displays a request and its accepted
receipt. `on_decision` receives `#{request,revision,action,always}`. Check the
ID, current revision, deadline and permission policy in the Host before
committing a decision. Increment the revision when changing the facts that
are being reviewed or whether durable approval is offered. The card only
displays `remaining_ms`; pointer/focus activity can pause a soft reminder,
but never extends an authorization deadline. During an asynchronous request,
set `busy` and leave the last accepted status unchanged until a result arrives.

`patterns/permission_request::PermissionRequest` distinguishes the scope
being proposed from the actual `grant` accepted by the Host. Timed allow
sends the requested minutes, not a locally generated deadline. Revoke sends
the exact accepted grant ID. For destructive requests the card never offers
perpetual access; if an old controlled choice no longer exists, it does not
fall back to a broader choice. The Host still validates every proposal;
only it can grant, revoke or expire access. Both Gallery pages simulate
requests in memory without any actual external effect.

`patterns/decision_card::DecisionCard` pages through choices, visual
picks, or approvals. It proposes `#{request,revision,kind,option,text}`.
Remove a decision only after accepting it. Revising the decision clears
its old draft; navigating away and back to the same revision does not.
`Later` and Escape fold the card without an answer, and changing `reveal`
unfolds it. When focus is on the options, use plain digits to answer,
or move to the custom answer field; modifier chords do not answer.

`patterns/decision_tree::DecisionTree` compares branching consequences.
Each item has a stable `id`, `revision`, and options with stable IDs.
Use `effects` for read-only evidence; a span such as
`#{text:"about $147",note:"Two renewals are pending",source:"Draft ledger"}`
exposes an explanation by pointer and keyboard instead of implying certainty.
Options with `children` open another level; only leaves or a custom
instruction propose `#{request,revision,option,text}`. Publish `busy` while
working, `notice` for a non-final response, and `receipt: #{option,message}`
only when the Host accepts the proposal. The Gallery demonstrates all of
these paths without acting on a real request.

`patterns/review_center::ReviewCenter` groups pending reviews by risk and
oldest wait within each group, and filters high-risk or low-confidence items.
Set `order` to `oldest` or `given` to use another order. Leave `filter`
unset for local filtering, or pass it and accept proposals through
`on_filter_change`. Pass the Host's `now_ms` for SLA and receipt expiry. Evidence can contain
flagged text, fields, plan steps, source IDs or `#{id,name,before,after}`
file comparisons on the native DiffViewer. Paths are labels, not file reads.
An approval with edits includes changed plain body text and a map of changed
field values by ID. Keep rejected proposals and their drafts visible; only
remove a request after accepting its exact revision. Undo identifies the
accepted receipt, not a queue index. Skip does not resolve the request.

`patterns/delegate_board::DelegateBoard` presents unassigned work and
accepted agent queues. Use a distinct `board` drag namespace for each
board in a window; task and agent IDs may be reused on different boards.
Move a card by dragging it, or focus the card and press Enter or Space to
choose a destination. The owning task's first queue entry is running; its
remaining time counts instead of its original estimate. Preview suggestions
don't move work. Manual, Assign all and automatic mode emit the same
`#{board,revision,kind,moves}` proposal; each move names its task, expected
source and destination. Accept the whole batch or keep the previous model.
Advance `revision` when the task, agent or mode model changes, not on clock ticks.
A rejected automatic proposal
does not retry unchanged data. The fixture assigns only sample records.

`patterns/agent_queue::AgentQueue` presents a time-stamped backlog, worker
slots, recent outcomes and a depth history from the same accepted data.
Supply a common `now_ms`; preserve records overlapping the history window
and provide `totals` if older outcomes have been pruned. A queued job
contributes from enqueue until start or termination; overlaps in rate
limits are counted once. Explicit job progress wins over estimated
progress. Keyboard arrows move among jobs; Enter pins evidence.
`on_action` gives the exact queue and revision for pause, resume,
concurrency, promotion, cancellation, or retry. The component never
starts a job or changes accepted ownership or counts.

`patterns/agent_lanes::AgentLanes` shows a shared time window for an ordered
set of agents. Overlapping and short tasks are placed on separate tracks
when their hit areas would collide. A task can link to its successor with
`handoff_to`; the line disappears when an endpoint leaves the window.
Each waiting-task approval, rejection, or optional open action is a
proposal carrying the exact timeline, revision and task ID. Selecting a
card opens inspection without changing the accepted timeline.

`patterns/agent_handoff::AgentHandoff` keeps a relay's agents, tasks, and
historical transfers separate. The newest transfer is first for each
adjacent pair, but older records can still be inspected. Refer to a task by
ID, not its title. Restoring material emits a proposal with the relay,
revision, transfer, task, and item IDs. Assignments work through a drag
handle or a keyboard destination picker; the Host applies the new owner.
The component does not fetch or publish the materials it displays.

`patterns/agent_inbox::AgentInbox` groups waiting and failed tasks in
Needs you, and running and done tasks in their own lanes. Board and list
layouts reuse the task cards; a narrow board stacks without changing the
accepted layout. Set `view` to control layout through `on_view_change`,
or omit it for a local choice. A single `now_ms` drives active durations;
finished cards use their fixed `elapsed_ms`. Expand a card to read up
to four steps. Approval, rejection, retry, open, and new task actions
are proposals with the exact inbox, revision and task identity.

`patterns/agent_plan::AgentPlan` reviews an ordered draft and then shows
Host execution. Use the grip to drag a step, or focus the grip and use
Option/Alt with an arrow. Edit and Add hold uncommitted text locally until
the Host accepts the proposal. Run includes the current accepted step
order; Retry identifies the failed step. Provide `now_ms` for active
elapsed time and a fixed `elapsed_ms` for completed steps. The component
does not execute tools or change the accepted plan on its own.

`patterns/agent_checkpoints::AgentCheckpoints` displays a run's accepted
history, oldest first. Give each checkpoint a distinct ID and reference only
earlier parents; omit the parent to start an independent attempt and omit
`head` to use the latest checkpoint. Arrow keys, Home and End navigate the
history. Rewind proposes `#{run, revision, head, checkpoint}` to `on_rewind`;
the Host validates that snapshot and can append a new checkpoint with the
selected checkpoint as its parent. Accepting an attempt does not require
deleting the earlier branch. Use `busy` and `notice` to show progress or a
refusal without moving the head.

`patterns/agent_trail::AgentTrail` shows execution, not a draft plan. Pass
the accepted `steps` and a single `now_ms`; running steps use `started_ms`
and finished steps keep their `elapsed_ms`. Running substeps open by
default. The view shows a bounded window around an active substep; the
earlier and later controls reveal all items. Pass `expanded` and
`on_expand` to control disclosure. For a waiting step, `on_decision`
receives `#{run,revision,step,choice}`. Set `busy_step` while resolving a
proposal; publish a new step state only when it is accepted. On refusal
the disclosure and question stay available for retry.

`patterns/agent_tree::AgentTree` shows a run's dispatch hierarchy. Pass
a flat `agents` array with stable IDs and optional parent IDs; the order
of siblings follows the input. An optional `supervisor` is a separate
peer, not a child of the root. Costs are integer millionths of the
currency unit. Folding hides cards but does not remove their costs or
their entries in the waiting and failed review panels. Arrow keys
navigate the tree; Left/Right follow the writing direction for folding
or entering a branch. Supply `collapsed` and `on_collapse` to control
folding, and apply `selected` only after accepting `on_select`. The review
actions propose `#{run,revision,agent,action}` to `on_resolve`; publish
`busy_agent` while resolving a proposal.

`patterns/agent_graph::AgentGraph` displays arbitrary communication between
agents, tools and people. Supply standing `edges`, chronological `calls`
with stable IDs, and one `now_ms` clock. A newly accepted pending call sends
a filled pulse; changing that call to `ok` or `error` sends a hollow pulse
back along its route. Initial history is not replayed. Link width reflects
calls in `heat_window_ms`, independent of the last-minute count. For
inspection, focus nodes with the arrow keys or Home; Enter opens a node and
Escape closes it. If you provide `on_select`, apply the proposed
`#{run, revision, node}` to `selected` explicitly; an empty node requests
dismissal.

For a custom decorative pulse, add
`scene.path_pulse(path_key, radius, paint, hollow, reverse)` to a static
Canvas path and animate the node's `path_progress`. The marker uses the
presented projection and does not add a hit target.

`patterns/agent_fleet::AgentFleet` shows a forest of lead agents and
the sub-agents of one open lead. Use the same flat agent IDs and
parent links as AgentTree; the total includes every branch, including
closed ones and negative cost credits. Use `controlled_open`, `open`
and `on_open_change` to accept or refuse disclosure in the Host; omit
controlled mode for local disclosure. Each sub-agent's task inspector
can show steps, approval evidence or errors. Resolution requests use
`#{run,revision,agent,action}` just like AgentTree; `busy_agent` disables
the affected agent's actions until the Host answers.

`patterns/agent_monitor::AgentMonitor` presents a single run. Send
`now_ms`, an accepted state, and a Host clock snapshot. Use
`paused_at_ms` while paused, accumulate completed intervals in
`paused_ms` on resume, and fix `ended_ms` for a terminal run. The
component never starts a clock or performs an action. A duration
segment opens the matching history step; review commands and step
output are literal evidence. Pause, Resume, Allow, Decline, Cancel,
Retry and Open log use `on_action` with `{run,revision,action,step,request}`.
An approval shortcut is active only on the focused request surface.

`patterns/agent_steering::AgentSteering` sends notes to a run without
changing its accepted steps or queue. Enter proposes to queue the
draft; Cmd+Enter on macOS or Ctrl+Enter elsewhere proposes sending it
to the active step. Shift+Enter is a native newline. A queued note can
be applied now or withdrawn by its ID. On acceptance, move the same
note ID into a step instead of duplicating it. Echo the action's
`proposal` in `receipt`; only an unchanged local draft clears after a
matching accepted receipt. A controlled `draft` always belongs to the
Host. Use a fresh run or component key for a new acknowledgement owner.

`Textarea` also accepts an `alt_submit_key` of `mod_enter` with
`on_alt_submit`. Its default alternate shortcut remains Alt+Enter.

Use `min_rows` and `max_rows` to grow a Textarea by visual lines. In an
inline composer, `min_viewport_height` and `max_viewport_height` can
add explicit limits to its native scroll viewport, after the row policy.
Use positive lengths; padding and borders are outside the viewport.
The viewport scrolls when content exceeds its height.

`patterns/agent_recap::AgentRecap` shows a versioned interval of activity.
Supply `run`, `revision`, epoch `since_ms` / `until_ms`, and keyed
`events`. The display time zone does not change event positions. In the
three-lane board, `limit` caps the visible cards per lane, not the
summary or plot. Set `board: false` for a summary strip. The app changes
event state only after accepting `on_action` proposals
`{run, revision, event, action}`; the action is `primary`, `review`,
`undo`, or `redo`. Keep the same card key and original kind after
resolution. Use `replay_serial` to request another decorative replay;
zero `replay_ms`, reduced motion and no motion show the whole interval.
You can observe chart and card inspection with `on_inspect`.

`patterns/agent_findings::AgentFindings` shows a team's verification
progress and its newest findings. The agent filter is local; run and
agent counts always come from the accepted snapshot, even when
`rows` caps the feed. Findings from departed agents remain available.
Supply `now_ms` for a reproducible Host clock; omit it for a view-owned
relative-time refresh. A finding with no timestamp appears last. When
present, `on_open` receives `{run, revision, finding, agent}`; validate
the revision and open your application's detail view by ID. Without it
the feed is read-only. Preserve IDs across updates so focus and row
movement follow the data, not its current position.

`patterns/agent_ledger::AgentLedger` presents a newest-first log of accepted
changes. Use stable IDs and explicit `depends_on` links within the supplied
acyclic graph. Hover or focus an Undo command to preview the same reversible
set the command will request; irreversible descendants are shown but left
unchanged. `on_action` receives the run, revision, command anchor, root,
operation kind, topologically ordered `changes`, and `locked` IDs. Validate
the revision and apply or reject the batch in the Host; the component does
not perform an external action or commit optimistically. For Undo, process
dependents before prerequisites when the application's transaction requires
it. Store accepted batches in `undone_batches` and hold `busy` while awaiting
a result. Redo restores exactly one accepted batch and is unavailable if a
separate prerequisite batch remains undone. Omit `on_action` for read-only
history; use `show_actions: true` when controls should not depend on hover.

`patterns/agent_cursors::AgentCursors` presents a bounded, read-only document
with live reviewers and optional review commands. Publish the accepted
`document`, `revision`, `active_tab`, `blocks`, `agents`, `suggestions` and
`comments`. Keep IDs stable across updates. Text offsets (including ranges
inside a proposal's insertion) use extended graphemes, not bytes or UTF-16
code units. Nonoverlapping suggestions decorate the current text without
changing it; use the original grapheme offsets until the Host accepts a
replacement. Images and table cells are independently keyed targets.

On `action`, check the document, active tab and revision before applying
a follow, tab change, suggestion decision, reply, resolution or row addition.
Keep `busy` true while awaiting a decision. Publish a new accepted snapshot
with rebased suggestions and cursor locations after changing text. A rejected
reply keeps the draft; an accepted reply clears it when the reply appears
in the published thread. Use the Gallery's AgentCursors story to try both
outcomes, move the typing cursor, or resize a held illustration. Omitting the
callback leaves document review and inspection available without mutation
commands. Cursor movement follows the view's motion preference and never
changes the accepted selection or the location of an interactive review pin.

`AgentCursors` fades the sheet by default; pass `fade: false` to keep its bottom
fully visible. This paint choice does not change the scroll range, review
controls or accessibility geometry. Custom surfaces can use
`style().bottom_fade(0.5, 0.97)` to fade the already-composited foreground
between two border-box height fractions; `without_bottom_fade()` clears it.
Use `style().edge_fade("y", 24, 24)` for transparent top and bottom edges,
or `"x"` for left and right. Distances are logical pixels, zero disables
that edge, and short boxes proportionally compress the ramps. The mask
does not change hit testing, scrolling, or accessibility geometry. It
replaces a less-specific fade, and `without_bottom_fade()` clears either
form. For a virtual list, give the first and last rows real scrollable
breathing room with the numeric `content_padding_top` and
`content_padding_bottom` options; they do not add item keys.

On a plain text node, `style().glyph_sweep(ink, 0.36, 0)` declares a
highlight band. Paint phase runs from 0 (off the left edge) to 1 (off
the right edge); the half width is a fraction of measured text width.
Animate it with `motion_keyframes("glyph_sweep", ...)` on a keyed text
node. The normal text remains visible, and the highlight reuses the
glyphs' own atlas coverage. Reduced and disabled motion suppress
the moving band. The isolated candidate currently draws this effect
on Apple monochrome text; color glyphs and other renderers keep their
ordinary output.
Use `style().glyph_ink_sweep(ink, 0.25, 1.5, 0)` when the highlight
should **replace** the original glyph ink rather than be layered on
top. It interpolates the premultiplied ink and applies glyph coverage
once, so a translucent highlight can also reduce opacity. Travel is a
text-box fraction, at least `1 + 2 * half_width`; use the same
`glyph_sweep` motion property and clear either mode with
`without_glyph_sweep()`.

Animate `background_position_x` to move only the front image of a
`layered_gradient`. Values are fractions of the remaining horizontal
positioning space (origin width minus image width), not pixels. This
preserves the authored border or padding clip, rounded corners, and
the other layers; it does not move the element or its input geometry.
With reduced or disabled motion the image stays at its authored
position, rather than freezing a moving highlight inside its clip.

Use `style().paint_reveal_x(4, 1)` with an entry motion on
`paint_reveal_x` from 0 to 1 to reveal a painted card from its physical
left edge. The radius is in logical pixels and is constrained by the
visible rectangle. Layout, keyboard focus and hit testing keep the
original size. Progress 1 returns to ordinary unmasked paint; reduced
and disabled motion use the authored progress.

In a theme's token map, `component_styles_replace: true` declares the
entire preset sheet. An omitted part or style property does not borrow
one from the token base. Leave this flag unset for palette-only themes
or partial style layers. Host, application, and component-instance
overrides still apply above the selected preset.

To animate a controlled size in a custom part, declare
`.layout_size_motion(key, "width", 260, "ease_out")` (or `"height"`). Use a key
that changes with the size-driving content or value, not on every render. The
layout engine constrains the new size before interpolation; siblings, clipping
and hit bounds see the same intermediate layout. Initial mount, same-key
reflow, and Reduced/None do not start a track.

For custom text placement, `text(...).wrap_exclusion("left", width, height)`
reserves a rectangle beside the initial visual rows, then restores the full
line width. Use `"right"` for the opposite physical side and pass geometry
relative to the text's top. Plain and styled text keep a single source and
text-anchor plan; zero width or height clears the rectangle. This method does
not place a figure or coordinate exclusions across multiple blocks.

Use `column([figure, paragraph, ...]).leading_float("start", 432, 126)`
to coordinate one figure with several prose nodes. The figure is measured
by the native layout, including any caption or current size animation. Each
prose node keeps its own text and anchors. The figure stacks when there is
too little room for the column or the text beside it. `AgentCursors` uses
this layout for consecutive prose after a floated image; a heading, list,
code block, or another figure ends the flow.

`patterns/decision_inbox::DecisionInbox` presents two quick lanes for each
request. Map its versioned `choice` (primary/secondary) to the intended
action in your Host; the caption is not an action ID. Publish `busy` while
working and a `resolved` row with a `receipt` after acceptance. You can
retain the receipt briefly, then remove the row. A rejected proposal stays
pending; an accepted receipt can unfold the next waiting item.

`patterns/approval_gates::ApprovalGates` shows the pending stack, blocked
steps, and accepted history of a Host run. Open gates can display caller
text, or file comparisons with `#{name,before,after}`; the component does
not read or modify files. Its `on_decide` includes the run, gate ID and
accepted revision. `approve_edited` carries the reviewed text or arguments;
`request_changes` carries feedback on a file proposal and is **not** an
approval. Publish the receipt only after accepting the correct revision.

`patterns/autonomy_dial::AutonomyDial` groups actions from the accepted
level, the caller's pins and independent `required_confirmation` rules.
Proposals contain the policy and revision, and change only a level or one
pin. Keep the accepted level and groups while a proposal is pending or
rejected. The dial is not an execution or authorization policy engine.

`MarkdownViewer` also accepts paint-only `annotations` into its **visible
plain text**, using inclusive/exclusive UTF-8 byte offsets. They do not
change text, link hits or selection. Use `markdown_literal` for escaped
literal inline text, `utf8_byte_count` for offsets and `text_match_ranges`
for bounded literal matches. Do not use Markdown source offsets for
annotations in a document containing Markdown syntax.

`components/selectable_card_group::SelectableCardGroup` takes controlled
`values` (an array even for `mode: "single"`) and keyed `items` with a textual
`label` and presentational `content`. Use `mode: "multiple"` for checkboxes,
and `layout: "row" | "column" | "grid"` for presentation. Put buttons, links
and other independent controls in each item's `actions`, not in its choice
content: acting there never changes the selection. Checked state is distinct
from focus. Single-mode arrows select by default; `select_on_focus: false`
requires Enter/Space to commit. Multiple-mode arrows only move focus. Choice
disabling does not silently disable independent actions. Use List/Select rather
than card groups for large collections.

## 5. Components are source

Formal components use the pattern:

```rhai
import "components/button" as button;

button::Button(#{
    text: "Save",
    variant: "primary",
    size: "md",
    on_click: Fn("save"),
})
```

Component schemas define required/optional props, event payloads, slots, parts,
state, dependencies, assets, and supported runtime API range. Unknown props and
invalid values fail before the component is committed.

Interactive components use explicit accessible names. `Input`, `Textarea`,
`Combobox`, `Select`, `DatePicker`, `Pagination`, `Menu`, `ContextMenu`,
`Popover`, `Tooltip`, and `Progress` require a textual `label` even when an
adjacent visual label exists. A placeholder is only a visual hint and is never
used as the control name. Omit Icon's optional `label` only when the icon is
decorative; a meaningful standalone image must provide one. IconButton always
requires its own action label. For persistent icon-only tool state, pass the
controlled `selected` prop: `ghost`, `outline` and `secondary` fill with
`selection` (the glyph keeps its color), stop reacting to hover, and expose
pressed accessibility state.

`Pagination` keeps `current_page` and `page_size` controlled. Accept its single
`#{ current_page, page_size }` proposal together; page-size changes propose page 1.
`compact:true` omits numbers/ellipses, independently of the compact density
preference. `disabled:true` blocks page and page-size input and closes the size
popup, while leaving the range text undimmed. The normal number window has a
bounded, stable count of page/ellipsis slots, including near either end.
Page targets keep business-page keys and the current target stays focusable:
accepting a proposal preserves native focus without a delayed focus command
that could steal it from another control. Gallery includes interactive mode
toggles, compact, disabled, wrapped, automatically narrow and empty examples.
When the numeric controls cannot fit, the native overflow row hides their whole
cohort and retains Previous/Next. Widening restores the same number targets;
this changes presentation, not the controlled page. The `controls` and
`page_slot` parts expose the same public overflow style mechanism to Host rules.

Formal component render functions are pure automatic reuse boundaries. When
root state changes, unchanged non-slot props and a clean component subtree let
the runtime reuse the prior component before calling Rhai. State, dependency
subscriptions, callbacks, effects, timers, signals, refs, and virtual
collections remain attached. Node/slot props are compared conservatively and
rerender. Behavior must never depend on how often a render function executes.

The main read paths have deliberately different invalidation semantics:

| Read | Runtime behavior |
|---|---|
| `ctx.get_state(...)` / `get_state_path(...)` | Component-local; a changed field dirties the owning component |
| `ctx.get_app_store(...)` / `get_window_store(...)` | Whole-field dependency |
| `ctx.get_app_store_path(...)` / `get_window_store_path(...)` | Exact bounded path dependency, including keyed-array selectors |
| `ctx.element_bounds(ref)` / `element_bounds("key")` | Tracked last-committed layout/visual/clip geometry; first render may return `()` |
| `ctx.element_size(ref)` / `element_size("key")` | Tracked layout width/height only; position, transform and clip changes do not rerender its reader |
| `ctx.element_width_class(ref, [480, 800])` / the ref-key overload | Committed layout-width band, `0..=2` here; only boundary/binding changes rerender the reader, unless a broader size/bounds read is also made |
| `ctx.event_target_bounds()` | Event-only, untracked current-target visual snapshot |
| `ctx.get_signal(...)` during render | Untracked hot value, but makes that component ineligible for render bailout |
| `node.bind_signal(...)` | Native property sampling without a Rhai rerender |

Prefer declarative props and tracked state/store reads for ordinary UI. Prefer
signal bindings or retained native work for values that change every frame.

Width-class boundaries are increasing finite non-negative logical-pixel
numbers (at most32). At an exact boundary, the lower band wins. A missing or
unmeasured ref returns `()` and remains subscribed during render; event reads
are untracked. This is a last-committed measurement on the Host controller,
not a synchronous CSS layout query. Choose a size-bounded measurement target
whose width does not alternate as a consequence of the selected band. Multiple
reads coalesce by intersecting their current stable intervals, without keeping
a growing threshold history.

Use `ui/styles.rhai` for coherent application-wide overrides keyed by formal
component ID and declared part. Use instance `style`/`part_styles` for an
intentional one-off. Edit the copied component source when the product needs a
structural or behavioral fork; do not hide one behind a growing stack of visual
overrides. See [Component stylesheets](docs/component-styles.md).

The bundled catalog contains official source components and patterns. Each
keeps one contract for its ID and export, controlled-state boundary, semantic
events, size vocabulary and style parts:

- foundations and status: Label, Divider, Icon, Avatar, Badge, Tag, Alert,
  Card, GroupBox, Empty, Kbd, Progress, Spinner, Skeleton, TitleBar, and
  StatusBar;
- actions and choices: Button, IconButton, ButtonGroup, Checkbox, Radio,
  RadioGroup, Switch, Toggle, ToggleGroup, Slider, and SelectableCardGroup;
- forms: Input, InputGroup, Textarea, FormField, Combobox, Select, DatePicker,
  RangeSlider, NumberInput, Calendar, TimeInput, and TimeRangePicker;
- navigation, layout, and data: Tabs, TabOverflowMenu, Carousel, SlidingPanel,
  Accordion, Collapsible, Menu, Pagination, Table, List, ScrollArea, SplitPane,
  Resizable, Draggable, DragSource, DropZone, Sortable, PanZoom, Rotatable,
  SelectionArea, Tree, ColumnBrowser, and Timeline;
- commands and overlays: Command, CommandDialog, ContextMenu, Popover, Dialog,
  AlertDialog, Sheet, Tooltip, and Toast;
- read-only documents: CodeViewer, DiffViewer, and MarkdownViewer;
- application patterns: Composer, NavigationStack, AppShell, DataView, and the
  other source-owned compositions under `patterns/`;
- primitives for Box/Text/Image/SVG/Canvas, layout, scrolling, refs, signals,
  layers, and generic overlays.

Use the unified CLI Gallery for running components, application scenes and
their Rhai source, rather than collecting separate demo executables:

```text
gpui-rhai gallery --list
gpui-rhai gallery --page scene.messages
gpui-rhai gallery --page navigation_stack --density compact
gpui-rhai gallery --page time_range_picker --theme default-light --locale zh-CN
```

In the Gallery, press Cmd+K (Ctrl+K on other platforms) and search for a theme
such as `Clean Light` or `Modern Dark`. This also works in a narrow window
where the title bar does not show the theme picker.
The same palette lists languages, searchable by names such as `Arabic` or
`Simplified Chinese`, even when the title-bar locale picker does not fit.

In a narrow window, or a medium window at 150% or larger reading size,
**Browse pages** opens the navigation drawer without taking width from the
main page. Select a page to close it, or press Escape to return to the previous
control. You can also go directly to a page with the command palette.

From this checkout, the matching command is
`cargo run -p gpui-rhai-cli -- gallery --page markdown_viewer`.
The new page IDs below describe this checkout's unreleased source; use its
matching CLI/runtime/registry:

| Workflow | Page IDs |
|---|---|
| Retained navigation | `retained_tabs`, `navigation_stack` |
| Reader position and tail following | `scrolling` |
| Hierarchical browsing and formatted ranking | `column_browser`, `formatted_table` |
| Drafting and rich choices | `composer`, `selectable_card_group` |
| Native Markdown | `markdown_viewer` |
| Dates, clock text and ranges | `calendar`, `date_range_picker`, `time_input`, `time_range_picker` |
| Event history | `timeline` |
| Integrated conversation | `scene.messages` |

The Gallery uses bundled, offline fixtures, including Host-owned large documents
and simulated streaming; it does not contact an AI service or load application
files. `--motion reduced` / `none`, density, themes and locales use the same
shared Gallery controls. Development stories, such as
`gpui-rhai gallery --story workbench/interaction-lab`, remain available through
the same CLI.
See [the component catalog](docs/components/catalog.md) for ownership and
behavior distinctions that similar-looking controls must preserve.
See [Gallery and acceptance application](docs/gallery.md) for story metadata,
Host/Rhai ownership, deterministic fixtures, lifecycle behavior, and the
verification matrix.

`IconButton` omits `selected` for an ordinary action. Pass `selected: false`
or `true` only for a controlled toggle, which exposes the corresponding
accessible pressed state. Its callback does not change that state itself;
the application stores the next value and passes it back.

`Kbd(#{ keys: ["mod", "shift", "p"] })` displays one combination in one
keycap, with platform-specific glyphs and a spelled-out accessible name.
Use `keys: "esc"` for a named single key, or `text: "F12"` for an unparsed
literal. Empty content draws no chip. `size` is `sm` (default) or `md`;
`platform: "mac"` / `"other"` is an optional presentation override, not a
keyboard-binding setting. Combinations preserve input order in RTL layouts.
Use `appearance: "inline"` for a bare legend in a menu or control.
Use `appearance: "menu"` for a contextual20px keycap inheriting the surrounding
ink. Its menu geometry is independent of the standalone `sm`/`md` size.
`key_legend("mod")` exposes the same `#{ label, spoken, glyph, platform }`
value to custom UI; the two-argument overload selects an explicit platform.
`key_shortcut("cmd-p")` instead parses an existing GPUI shortcut for display.
Each parsed chord exposes `key`, `modifiers`, and ordered `keys` display/spoken
descriptors; use those to build individual keycaps, not string splitting.
Already-formatted text such as `"⌘P"` or `"Ctrl+P"` stays one literal legend.
Neither function installs a binding; bind the application's actual action
through the normal Host action/keymap APIs.
Registered action IDs use `namespace.snake_case_action`, such as
`app.open_palette`. They are not arbitrary labels or collection keys; do not
apply this naming restriction to business row keys or component identity keys.
For an embedded view, `ScriptView::key_binding` supplies the approved shortcut
metadata; the Rust Host must explicitly install it with
`ScriptViewHost::bind_keys`. Standalone launch performs that Host step itself.

Use `accessibility_hidden(true)` only for decorative duplicates whose meaning
is already on their semantic owner. It removes the subtree from accessibility
and implicit names, not from painting, layout or contrast audits; it does not
disable focus or input. Do not use it to hide interactive controls.

Use `accessibility_description(text)` for auxiliary plain text that should not
replace a control's name. A declared `accessibility_described_by` relationship
takes precedence, including an empty resolved result. Data-only virtual rows
can use direct descriptions without declaring an ElementRef; references still
belong to formal components, including formal children rendered inside a row.

### 5.1 Retained pages, navigation and hierarchical browsing

`components/breadcrumb::Breadcrumb` presents a controlled root-to-current
trail. Items have stable `value` keys and full `label`s; the last item is
current and never navigates. Long trails fold their middle according to
`max_items` and the measured width of that specific container. The accessible
ellipsis reuses Menu for arrows, activation, dismissal, and focus restoration.
Selecting an inline or folded ancestor emits the same string to
`on_navigate`; your application owns the route and may reject the request.
Use `separator_kind: "slash"` for directory paths and `overflow_label` to
localize the ellipsis. Install it with `gpui-rhai add breadcrumb`.

`Tabs` defaults to presenting only the selected panel. Set
`keep_mounted: true` when switching away must preserve native scrolling,
editing, selection, undo and focus identity. Hidden retained panels take no
layout space and leave input traversal and presented accessibility; hidden
motion does not keep requesting frames. Returning shows the retained state,
not a second mount.

This includes the actual native virtual-list state: measured rows, reading
anchor, paused tail-follow mode and unseen-item accounting. New data received
while hidden does not silently resume following. Real panel/collection removal
releases its native resources; recreating the same public key is a new mount.
Tab values must be unique strings; `""` is valid, for example for an All filter.

Mounting, visibility and business activity are separate. Hiding never
automatically cancels a capability task, subscription or application effect.
`ctx.is_visible()` is a tracked read of last-committed page visibility, false
before first placement. Use it in an explicit application pause/effect policy
where needed. Eager node construction still follows caller ownership: omitting
a passed node from presentation does not dispose its formal component. Stop
constructing the owner, or cancel its work deliberately, to end its lifetime.

`patterns/navigation_stack::NavigationStack` uses the same retained-page policy
for a caller-owned root-to-top `stack`:

```rhai
import "patterns/navigation_stack" as navigation;

navigation::NavigationStack(#{
    key: "details", label: "Service details", header: "breadcrumb",
    stack: [
        #{ key: "visit-services", title: "Services", content: services_page },
        #{ key: "visit-ticket-4821", title: "Ticket 4821", content: ticket_page },
    ],
    on_pop: Fn("pop_requested"),
})
```

Import `patterns/navigation_stack` and construct the page nodes in their
application owner. Keys identify visits, not business record IDs: opening the
same record twice requires distinct visit keys. Back, breadcrumb and keyboard
interaction emit `#{ depth, source }`; the application accepts by truncating
its visit data and rebuilding the stack. The component never loads a route,
fetches data or changes the stack itself. `max_depth` bounds retention without
silently evicting visits, and `motion_key` enables optional push/pop continuity
that respects RTL and reduced motion. A Rust Host can dispatch `NavigateBack`
or call `ScriptViewHost::navigate_back(window, cx)` before its outer Back; a
true result means the focused inner stack owns the proposal, even if its caller
declines it. Surviving pages restore focus where possible.

`components/column_browser::ColumnBrowser` is a controlled chain of virtualized
List columns, not a filesystem or loading service. Each keyed column supplies
`items` (Array or NativeCollection), a `selected` key or shared controlled
`path`, and its own loading/error/empty state. `on_select` proposes
`#{ column, key, path }`; `on_activate` is a separate leaf Enter/double-click
proposal, and `on_retry` names the failed column. The caller replaces
descendants and rejects late loading results using its path/request revision.
Columns do not shrink to fit: the strip scrolls horizontally and can include
caller-owned trailing `detail`. Supply exactly one of `height` or
`fill_height: true`. Logical depth arrows respect RTL, while path breadcrumbs
and Copy path preserve the controlled hierarchy. There is no automatic
selection when a controlled key disappears.

### 5.2 Other layout and command compositions

For responsive equal-width tiles, use a Box with
`style().width(relative(1.0)).auto_grid_columns(128, 160, 224).gap(theme_spacing("sm"))`.
The arguments are minimum, preferred and maximum track widths: logical pixels,
or Lengths such as `theme_length("metrics.label_column")` so the grid and its
cards share one token. Tokens resolve natively in the node's environment, so a
theme or density change moves the tracks without running Rhai on resize; bounds
that resolve out of order lay out with the minimum kept and the maximum raised
to it, and the Host receives a presentation diagnostic. Minimum determines
auto-fit breakpoints: tracks shrink to minimum before losing a column. Preferred supplies the native initial/natural width hint,
not a second breakpoint or a history-dependent column-count policy. Maximum
caps growth and can leave unused space after the start-aligned tracks.
The last row uses the same tracks as preceding rows. Rows retain their
intrinsic content height; square media can use
`style().width(relative(1.0)).aspect_ratio(1)` on the media node, leaving its
height automatic. Two explicit dimensions take precedence over aspect ratio.

This is a native equal-cell layout, not complete CSS Grid: use explicit
`grid_cols` / `col_span` for spanning cells, and virtual collections for large
data sets. Native width refinement can request one additional presentation
frame after mount or resize; it does not execute Rhai, invoke resize callbacks
or write application state. Padding, border and theme gap lengths participate
in layout. See the CLI Gallery's **Foundations → Responsive grid** page.

Tabs with panels allocate the bounded remainder to their panel container on
either axis. Give the Tabs root a bounded size (or a flex item allocation) for
fill-height content such as SplitPane and Table. Without panels, Tabs leave
cross-axis alignment to their parent; they work in centered TitleBar/Toolbar
rows without a wrapper override.

`SplitPane` is a controlled two-panel composition. Its `size` is the start
panel ratio; pointer movement writes only a native optional-float signal and
`on_resize` fires once on release. Arrow keys on the semantic separator commit
the same ratio contract. `start_collapsed`/`end_collapsed` are controlled, and
nested SplitPane instances cover multi-panel layouts without introducing a
docking or persistence framework.

Its default grip grows smoothly from 24px to 40px while dragged, using the
theme's `fast` motion (120ms by default); hover/focus change its paint, not its
length. Reduced/None motion
snaps to the current state. The stable hit area does not grow with the bar.
`grip_length` and `grip_active_length` configure the motion range; `grip: false`
removes only the decoration. Use `handle: node` for a static replacement, or
`handle_renderer: ctx.node_factory(Fn("my_grip"))` for a custom stateful look.
Bind the factory where it is declared, before forwarding it through a helper
module. It receives the receiving component's `ctx` and a detail map with native
`state`, `orientation`, `duration_ms` and `easing`. `signal_style` and
`progress_motion(motion_signal(detail.state, ["drag"], source))` preserve native
sampling and Host motion policy. It is not an event handler or a per-frame
callback; SplitPane continues to own dragging, keyboard, focus and cancellation.
Use `grip` state parts for the hit area and `grip_content` state parts for its
decoration. The CLI Gallery's SplitPane page shows default, static and custom
animated grips, including a 7px transparent hit area on a 1px separator.

`Resizable` controls one positioned rectangle inside its own local boundary.
Its `rect` contains `x`, `y`, `width`, and `height`; physical edge/corner
handles propose one complete replacement rectangle. Native signals preview
position and dimensions without Rhai pointer-move callbacks. Left/top handles
move the origin while preserving the opposite edge. SplitPane never adopts
these free-rectangle semantics.

Command palettes separate seating, preview, and confirmation. The caller-owned
`active_value` determines the current highlight when the palette opens.
`on_active_change` receives deduplicated keyboard/pointer roving requests and is
suitable for temporary theme, font, or density previews; `on_action` is the
Enter/click commit boundary. CommandDialog forwards both as formal component
events, so their callbacks retain the caller's Rhai module context.
`Command` defaults to `appearance:"panel"`, which owns its tray/card chrome.
Use `appearance:"embedded"` when a surrounding host panel owns the edge;
CommandDialog uses that mode automatically. These modes keep one native editor
and the same keyed results subtree. The outer semantic group contains the
editable field and a separate labelled listbox; it is not itself the listbox.
`footer` accepts application-owned node content, and `footer_visible:false`
omits its presentation. Footer controls retain their own semantics and keys:
Command's direction/Enter handling is scoped to search/results, not those
controls. As with other eager node props, stop constructing an effectful footer
component in the caller if hiding it should also end its lifecycle.

The `search` part owns header spacing and its separator; `search_input` owns
editor typography and geometry, and `search_placeholder` owns its soft ink.
`card`, `root_embedded`, `card_embedded`, `list`, `list_viewport`, `footer` and
the icon/hint parts are separate override points. `list` padding is outside the
measured row viewport, so it does not silently remove space from the last row.
`max_visible` sets its cap in command-row units; group headings have independent
typography/padding and their natural measured height. Short filtered results
fit their actual extent, without eagerly measuring a large result set.
The default footer describes existing arrow/Enter behavior and installs no
global binding. The Gallery's command page demonstrates panel/embedded modes,
observable actions and an interactive caller-owned footer.

`narrow_at` defaults to480 logical pixels of the **inner card**, independently
of window size and density. After its first measurement, narrow presentation
uses44/40px rows in Modern/Clean, hides visual hints, keycaps and the Enter
badge, but keeps option descriptions and real bindings. Modern retains the
navigation footer; Clean omits its default footer. Caller-owned footer content
stays visible when `footer_visible` allows it. `item_narrow` is an override
part in the same common/state cascade. CommandDialog forwards `narrow_at` and
uses exactly the shared Command item contract. The initial unmeasured layout
uses wide presentation; transitions occur after a committed width-band change.
Gallery's Wide/Narrow buttons exercise the same keyed control, not two mockups.

Command items may also provide a secondary `hint`, a decorative `icon`, or a
`logo` on a small tile. Array icons/logos accept a non-interactive node or a
logical asset ID string; NativeCollection rows use asset ID strings, not UI
nodes. `logo` takes precedence over `icon`. The leading slot remains reserved
when both are absent, so labels align. Hints share the label's baseline, take
part in search with keyword precedence, and describe the option without
replacing its required textual name. `icon_slot`/`icon_slot_active`, `logo_slot`,
`icon`, `logo`, `text`, `label` and `hint` are separate part-style override
points. Rich `content` still replaces only the label; decorations never acquire
their own tab stop or command handler. Referenced assets must be registered and
prepared through the normal asset pipeline; a string is not a filesystem path
or permission to fetch a URL.

`TitleBar` and `StatusBar` are application chrome compositions, not a window
authority escape hatch. An embedded view can render either but cannot move,
minimize, or close its Host window. A trusted standalone Rust Host that wants
content under the native macOS titlebar configures transparent chrome itself:

```rust
use gpui_rhai::gpui::{TitlebarOptions, point, px};

ScriptApplication::new(prepared)
    .window_options(|mut options, _cx| {
        options.titlebar = Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(point(px(9.0), px(9.0))),
        });
        options
    })
    .run()?;
```

The corresponding Rhai `TitleBar` may reserve the traffic-light area with
`inset_start: 70`. Native dragging and window actions remain Rust-owned and can
be connected through explicitly registered native handlers when required.
`label` is the required accessible window-area name. `title` and optional
`subtitle` accept either plain strings or structured nodes: strings receive the
default type treatment, while nodes retain their own styling and event/native
handler bindings inside the same inset, clipping, and region layout.

Use [Component authoring](docs/component-authoring-guide.md) and the component
specifications under `docs/components/` when modifying or creating components.

## 6. Callback context: the part agents most often get wrong

### 6.1 Script callbacks

A normal callback is a Rhai `FnPtr`:

```rhai
fn selected(ctx, value) {
    ctx.set_state("selected", value);
}

select::Select(#{
    key: "country",
    label: "Country",
    value: ctx.get_state("country"),
    open: ctx.get_state("country_open"),
    query: ctx.get_state("country_query"),
    options: countries,
    on_change: Fn("selected"),
    on_open_change: Fn("country_opened"),
    on_query_change: Fn("country_queried"),
})
```

The runtime binds the callback to:

- its script generation;
- the owning component instance path;
- the current mount incarnation of that path;
- the declared event schema;
- its imported-module invocation context;
- any `UiValue`-convertible curried arguments.

`Fn("name")` in the SDK engine retains the module environment **where the
pointer is created**, including in ordinary application helper/page modules,
not only formal component modules. `named_function("name")` is its explicit,
reserved SDK alias (do not declare a script function with that name).
A callback can be returned from a helper, placed in a handler map,
curried and forwarded without changing that environment. Same-named helpers
in different modules remain distinct, including during incremental updates.
Synchronous `call` and `curry` also retain the module environment.

Lexical lookup and UI state ownership are separate: a plain page function does
not create a component instance. Its callback still reads/writes the state of
the calling UI owner. A formal component's own handler uses that component's
state. Generation, incarnation, event-schema and durable-curry checks still
apply to both. Anonymous/capturing functions have not become durable callbacks.

This environment retention is an SDK facility over pinned Rhai 1.26, not a
property of bare Rhai's name-only `Fn`. Do not reduce callbacks to a function-name
string, reconstruct one in a different module, or call a retained FnPtr against
an unrelated AST.

A formal component may bind a caller callback to a node it owns, but it must not
pass that callback through another formal component's callback prop. Although
the runtime preserves its original scope, doing so bypasses the composite's
declared event contract. A composite component must receive the child event
with its own named handler and then emit its declared event:

```rhai
fn child_changed(ctx, value) { ctx.emit("change", value); }

child::Child(#{ on_change: Fn("child_changed") })
```

The runtime then dispatches `change` to the composite's `on_change` prop in the
original caller context. This event boundary applies equally to pointer and
keyboard handlers.

Node-valued slots preserve callbacks already attached by their caller. The
runtime binds every `Node`/`Array<Node>` prop to the caller before invoking the
child component, so a raw button placed in TitleBar `start`/`center`/`end` or a
Dialog action updates the state of the component that created it. Component
code must not strip and reconstruct a slot's handler as an unscoped `FnPtr`.

Passed formal components also preserve their caller-owned state and resources.
If the receiving component rerenders, the runtime refreshes its node props from
the passed components' latest owned snapshots without rerunning them; effects,
tasks, timers, signals, refs, and virtual state continue unchanged. Styles,
handlers and other presentation mutations added by the receiver survive child
updates and do not accumulate on repeated receiver renders. Node construction
is eager, so the caller—not the receiver—must conditionally stop constructing a
component when hiding it should also unmount its lifecycle.

Durable callbacks must be named, non-capturing functions. gpui-rhai rejects
anonymous/capturing closures and curry values that cannot cross the `UiValue`
boundary. Pass durable data in component props, state/store fields, or explicit
`Fn("name").curry(value)` arguments instead of relying on a closure environment.

The first callback parameter is gpui-rhai `UiContext`, not GPUI `Context`.
`UiContext` exposes the safe runtime surface: state, stores, locale, themes,
effects, tasks, subscriptions, refs, signals, and validated commands.

Old callbacks are rejected after reload. Unmounting and later recreating the
same logical key produces a new incarnation, so callbacks and NativeSignal
handles retained from the old mount remain stale instead of targeting the new
component. Deliveries after unmount or disposal are discarded by
generation/scope/incarnation ownership.

Subscriptions are legal only from a declared formal-component effect start.
This gives the runtime an exact cleanup owner on dependency replacement,
unmount, retained suspension, reload, and disposal. One-shot `start_task` calls
may also originate in `init`, ordinary events, and `resume`; do not emulate a
long-lived stream with recursively started unowned tasks.

### 6.2 Events, propagation, and geometry

Atomic nodes accept `on(event, handler)`, `on_capture(event, handler)`, and
`on_bubble(event, handler)`. Raw pointer/wheel and focused `key:*` dispatch
support capture → target → bubble. Ordinary native clicks and declared
component events retain target-only delivery; adding a phased handler does
not turn them into raw events.
Handlers may return `event_response()` and refine it with `prevent_default()`,
`stop()`, `stop_immediate()`, `capture_pointer()`, or `release_pointer()`.

Raw `pointer_down`, `pointer_up`, and `pointer_move` payloads contain pointer
identity/type, `window`, `local`, and scroll-adjusted `content` coordinates,
movement, buttons, modifiers, click count, timestamp, capture state, optional
pressure/tilt, and `target`. Raw `wheel` payloads contain those three coordinate
spaces, delta, precision, modifiers, timestamp, and `target`.

`target` is `{ x, y, width, height }` in window coordinates: the committed
visual bounds of the node owning the currently executing handler. This is
`currentTarget` semantics, not a guessed logical card or the deepest painted
child. The value is captured when the event is dispatched.

Click and component events keep their declared payload type. They do not gain a
hidden map wrapper. Read the same geometry from the callback context instead:

```rhai
fn float_clicked(ctx, payload) {
    let bounds = ctx.event_target_bounds();
    if bounds != () {
        // Pass `#{ id: payload, bounds }` to the app's declared bridge.
    }
}
```

`ctx.event_target_bounds()` is event-only and untracked. Callbacks not
dispatched from a retained node—including actions and custom primitive
emissions—receive `()`. Calling it from render/init/dispose is an error. This is
the correct path for low-frequency geometry use such as positioning a detached
window; do not maintain a resize → store → Rhai geometry channel for it.

Use `element_ref(...)` plus `ctx.element_bounds(ref)` only when rendering must
react to another retained element's last committed geometry. That read creates
an exact dependency, returns `#{ layout, visual, clip }`, and may initially be
`()`. The runtime keeps the unresolved dependency by ref identity, binds it to
the current `NodeId`, and rerenders after first prepaint reports geometry.
The logical subscription remains live through appearance, rebind and removal;
removal produces `()` and old-node changes no longer wake a rebound reader. It
is released when its component/item contribution leaves the committed graph; do
not add an unrelated redraw to make it self-heal. Event callbacks cannot retain
the custom `ElementRef` value and instead call
`ctx.element_bounds("component_local_ref_key")`. It cannot provide synchronous
same-layout feedback.

When only dimensions matter, use `ctx.element_size(ref | "key")` instead: it
returns `#{ width, height }` and preserves the same pending/ref-rebinding
subscription, without waking Rhai when a native drag merely moves the element.

Rust elements that need caret or selection geometry should read the same
layout they paint. See [native text geometry](docs/text-geometry.md) for
grapheme boundaries, visual rows, and layout-to-window coordinates.

### 6.3 Rust native handlers callable from Rhai

Use a `NativeHandlerRef` when a Rhai-authored component should call trusted Rust
directly:

```rust
use std::collections::BTreeMap;
use gpui_rhai::{
    EventResponse, ScriptView, NativeHandlerDescriptor, NativeHandlerId,
    ScriptViewExtension, ValueSchema,
};

struct HostBridge;

impl ScriptViewExtension for HostBridge {
    fn configure_engine(
        &self,
        engine: &mut gpui_rhai::RuntimeEngine,
    ) -> Result<(), String> {
        let descriptor = NativeHandlerDescriptor::new(
            NativeHandlerId::parse("host.refresh").map_err(|e| e.to_string())?,
            BTreeMap::from([("click".to_owned(), ValueSchema::Null)]),
        ).map_err(|e| e.to_string())?;

        engine.register_native_handler(
            descriptor,
            |event, runtime, window, app| {
                // `event.payload` has already passed the declared schema.
                // `event.target` is the optional event-time visual bounds.
                let _ = (event.target, runtime, window, app);
                Ok(EventResponse::new().stop())
            },
        ).map_err(|e| e.to_string())
    }
}

let prepared = ScriptView::files("ui/main.rhai")
    .extension(HostBridge)
    .prepare()?;
```

Rhai resolves only pre-registered namespaced IDs:

```rhai
button::Button(#{
    text: "Refresh",
    on_click: native_handler("host.refresh"),
})
```

Native handlers work through formal component callback props and native
primitive callback props. Their declared event names and payload schemas are
checked before Rust runs. `NativeEvent::target` contains the same optional
event-time visual bounds without changing the schema-checked payload.

Use native handlers for:

- direct host bridging without an adapter Rhai callback;
- short performance-sensitive event paths;
- foreground coordination with a Rust-owned entity or service;
- validated file/window/application operations intentionally granted by the
  host.

Do not block the foreground thread. Start async or background work in Rust and
deliver validated data back through host state, tasks, subscriptions, or
signals.

### 6.4 HostCallback and custom primitives

`HostCallback` is for a Rust-built `UiNode` tree. Rhai cannot construct or
receive it. A custom primitive is trusted Rust implementing a mechanism that
ordinary source components can compose.

Choose the smallest boundary:

| Need | Use |
|---|---|
| Script state transition or ordinary UI event | Rhai `FnPtr` |
| Rhai component calls trusted Rust directly | `NativeHandlerRef` |
| Rust owns the whole declarative tree | `HostCallback` / `StaticUiView` |
| New GPUI/platform mechanism | custom primitive |
| Per-frame typed paint/layout value | `NativeSignal` or retained native subtree |

Host-built `StaticUiView` trees can opt into `with_theme(ThemeVariant)` and
`set_theme(Some(variant), cx)`. These reuse the normal token/typography resolver
without adding a script runtime. The Host owns preset selection and font
installation; switching the snapshot preserves keyed nodes and native input
state. See [Host-owned trees](docs/embedding.md#host-owned-interactive-trees)
and `cargo run --release -p gpui-rhai --example host_owned_tree`.

## 7. Rust host patterns

### Standalone script application

```rust
let prepared = gpui_rhai::ScriptView::files("ui/main.rhai")
    .prepare()?;

gpui_rhai::ScriptApplication::new(prepared).run()?;
```

### Embedded view inside a Rust-owned GPUI layout

```rust
gpui_rhai::install(cx);
let host = gpui_rhai::ScriptViewHost::new("main-window", cx)?;

let view = gpui_rhai::ScriptView::files("plugins/user-view/ui/main.rhai")
    .prepare()?
    .mount(
        gpui_rhai::ScriptViewConfig::new("user-view"),
        host.clone(),
        window,
        cx,
    )?;

let root = host.container(
    gpui::div()
        .flex()
        .child(rust_owned_sidebar)
        .child(view.flex_item()?)
);
```

The Rust application continues to own the window and surrounding tree. The
Rhai view owns only its declared view. Use one `ScriptViewHost` for sibling
views that should share overlay dismissal, Escape routing, portal placement,
and focus fallback. `flex_item()` is the normal direct child for Rust flex
layouts: it supplies a zero basis and `min-width/min-height: 0`, preventing a
wide scrollable Table from becoming the host column's automatic min-content
width. Use `element()` for fixed, absolute, grid, or manually styled placement.

Call `view.dispose(cx)?` when removing a mounted view permanently. Merely not
rendering it for one frame does not dispose its tasks, overlays, and scoped
resources.

Advanced Rust extensions that group several direct `UiRuntimeState` mutations
use `begin_transaction`, followed by exactly one `commit_transaction` or
`rollback_transaction(checkpoint)`. Checkpoints are runtime-bound opaque
tokens; a token from another runtime, an unmatched commit, or a reused token is
rejected.

For a small Host-owned LRU of expensive panels, use retained suspension instead
of dispose/remount:

```rust
view.suspend(window, cx)?;
// Omit it from newly built layout while view.state() is Suspended.
view.resume(cx)?;
```

Suspension preserves component/native/input/scroll/virtual state but cleans
effects, cancels their subscriptions and tasks, freezes timers and animations,
and closes presentation layers. Ordinary one-shot task results wait in a
bounded queue. `element()`, `flex_item()`, focus, automation, accessibility, and
element-ref interaction reject suspended handles. Resume delivers valid task
results, calls `resume(ctx, elapsed_ms)`, reconciles once, and starts fresh
effect activations atomically. A failed resume remains suspended. See
[Embedding](docs/embedding.md#retained-suspension-and-host-managed-tombstones)
for the full hot-reload and generation rules.

See [Embedding](docs/embedding.md) and [Multi-window](docs/multi-window.md).

Use ordinary `PreparedScriptView::mount` for an embedded view without native
window authority. If Rust deliberately delegates native open/focus/close and
close-confirmation policy to one view, use `mount_window` instead. It registers
the real native handle and enforces one command owner per native window; sibling
views remain restricted. It replaces the previous should-close callback, not
the Rust root/layout. Disposing the owner does not close Rust's window; native
closure disposes retained view handles. See the multi-window guide for the
queued-operation boundary and cleanup rules.

### The Rhai standard library in view scripts

Two string details matter when adapting JavaScript-like examples: `trim()`
mutates its string and returns `()`, so trim a local copy before comparing it.
Backtick templates preserve escape text: append `"\n"` outside the template
(or put an actual newline inside it) when building multiline source. Do not
expect a template's literal backslash-n to become a line break.

View scripts get Rhai's standard library as `rhai::Engine::new()` sets it up,
including `sleep`, `exit`, `eval`, `timestamp`, `parse_json`, `print` and
`debug`; gpui-rhai restricts none of it. A Host decides what its views keep in
`ScriptViewExtension::configure_rhai_engine`, which receives every engine the
runtime builds for the view (preparation, each script window and each
dev-reload candidate) after every extension's `configure_engine`:

```rust
use gpui_rhai::rhai::{Dynamic, Engine, EvalAltResult};

struct ViewPolicy;

impl gpui_rhai::ScriptViewExtension for ViewPolicy {
    fn configure_rhai_engine(&self, engine: &mut Engine) -> Result<(), String> {
        type Refused = Result<(), Box<EvalAltResult>>;
        fn refuse(name: &str) -> Refused {
            Err(format!("`{name}` is not available in a view").into())
        }
        // A keyword: a script that uses it fails to compile.
        engine.disable_symbol("eval");
        // A function: register every overload under the same name.
        engine.register_fn("sleep", |_: f64| -> Refused { refuse("sleep") });
        engine.register_fn("sleep", |_: i64| -> Refused { refuse("sleep") });
        engine.register_fn("exit", || -> Refused { refuse("exit") });
        engine.register_fn("exit", |_: Dynamic| -> Refused { refuse("exit") });
        engine.on_print(|text| eprintln!("[view] {text}"));
        Ok(())
    }
}
```

What the functions a Host most often limits do in a view:

- `sleep` blocks the window's thread, and the operation budget does not see it:
  it is one native call. A float too large for a `Duration` panics.
- `exit` in an event callback ends the callback; the view reports
  `NOT AN ERROR - exit value` as its error.
- `eval` compiles and runs source the script builds.
- `timestamp()` reads `Instant::now()`, not the Host's `runtime_clock`.
- `parse_json` decodes with serde_json, because gpui-rhai turns on Rhai's
  `metadata` feature. If any crate in the build turns on serde_json's
  `arbitrary_precision`, every number that is not an integer arrives as a map;
  register your own `parse_json` to decode numbers your way.
- `print` and `debug` write to standard output unless the Host sets `on_print`
  and `on_debug`.

A registered function replaces only the overload with its parameter types. The
operation budget runs in the engine's `on_progress` callback; replacing that
callback removes the budget. `gpui-rhai check` does not run the Host's
extensions: a script that uses a disabled keyword passes `check` and fails when
the Host prepares it.

## 8. Layout, text, images, and assets

Use `row` and `column` for ordinary flex containers:

```rhai
row([
    icon_node,
    text("Build complete"),
]).with_style(
    style().height(theme_length("metrics.row")).gap(theme_length("space.related")).items_center()
)
```

`items_*` controls the cross axis; `justify_*` controls the main axis. Styled
fixed-height Text nodes use their declarations to position their text content.
Input and Textarea own separate native line-layout implementations.

For wrapped flex and grid layouts, `.gap_x(theme_spacing("sm"))` and
`.gap_y(theme_spacing("xs"))` control horizontal and vertical spacing
independently. These are physical axes, not main/cross-axis or RTL aliases.
A later `.gap(...)` resets both axes; a later axis setter changes only that
axis. Automatic grids use the horizontal value for their column breakpoints.

For a small responsive toolbar, use `.overflow_row()` on a Box row and mark
optional children with `.overflow_priority(0)`. Equal-priority cells form one
cohort; lower-priority cohorts collapse first. Unmarked cells are required,
even when required content alone cannot fit. `.overflow_required()` clears a
lower-layer optional priority, and `.overflow_row(false)` restores ordinary
layout. Font/length/theme changes and resize use real native measurement,
without a Rhai render loop.

For a counted overflow suffix, append children marked
`.overflow_summary_for(n)`, one for each supported omitted-cell count.
The fitter reserves the corresponding suffix's actual measured width.
Only the matching suffix can be presented, and it never removes a required
child. For example, a compact strip of filter chips can display `+3` rather
than leaving the final three chip buttons in the native focus order.

This is intrinsic packing, not flex grow/shrink distribution: cells keep their
measured sizes. The row keeps its full preferred width and line height so it
can restore hidden cohorts when its allocation grows without vertical jitter.
Wrap a group in `row`/`box` to make it one cell; Fragment layout styles do not
make cells. Grid, wrapping, baseline and stretching layouts take precedence.
Omitted cells are still measured and retain logical state/effects; they do not
paint, receive keyboard input or appear in presented semantics. This is not
virtualization or a retained-page lifecycle pause. Use virtual collections for
large data, and `retained_page` for its explicit visibility/lifecycle policy.

Use semantic layout directions and `margin_start`/`padding_end` where RTL must
mirror. Avoid hard-coded left/right behavior for logical navigation.

### Layouts, patterns and the composition audit

Screens are composed from `layouts/` (neutral) and `patterns/` (opinionated)
rather than hand-built rows of padding and gaps:

| Module | Use |
|---|---|
| `layouts/stack`, `layouts/inline` | a column or row with one spacing relationship |
| `layouts/toolbar` | context, filters, one filling field, actions; wraps when narrow |
| `layouts/region` | a working area: title and actions, toolbar, filling body, footer |
| `patterns/section`, `patterns/description_list`, `patterns/stat` | headed content, label/value pairs, figures |
| `patterns/form_layout`, `patterns/inline_state` | aligned forms; loading, empty, error and stale states in place |
| `patterns/data_view`, `patterns/list_detail`, `patterns/app_shell` | a data region, a list beside its detail, a window shell |

The composition audit checks the committed geometry against the rules in
[composition](docs/design/composition.md): rows of mismatched height, text
edges a few pixels apart, inner gaps not smaller than outer ones, several solid
actions in one group, mixed type in a row, low-contrast text. A profile
(`ui/profile.rhai`, installed by `gpui-rhai init --profile productivity`)
selects the rules; `ScriptViewHandle::composition_audit` reports them at
runtime and `gpui-rhai check` adds static warnings for literal geometry in
application source. A node opts out of a rule with `.audit_allow([...])`.

Low-contrast findings include `AuditFinding::contrast`: foreground RGBA,
inherited element opacity, a solid backing's unquantized sRGB value (when one
exists), ratio, required minimum and a conservative-bound flag. This is the
declarative paint model, not a GPU pixel sample. Hidden/unpresented content is
not a visible contrast failure; elided fragments still contribute their actual
children. Opacity follows native per-primitive compositing, not assumed CSS
group isolation.

Modern/Clean's four exact presets retain their authored colors and report their
readability risks; preservation is not a WCAG pass. For higher contrast, choose
an explicit alternate palette or Host override rather than silently rewriting
those presets. The SDK audit stays strict for every theme. In the Gallery,
activate **Open audit report** in the status bar to read/copy the complete
findings, including color evidence. The report updates when facts change, even
if the finding count stays the same.

Declare shared component assets in component metadata. Application assets live
under `ui/assets` and are referenced by logical identity:

```rhai
image_source(asset("app/icons/check"))
```

SVGs may use `currentColor`. External inheritance reads the effective native
text color at presentation time, including native hover/focus refinements,
without requiring a Rhai rerender. A color declared within the SVG keeps its
own scope; fixed fills are not recolored. Shared icons should
use a consistent 24×24 viewBox and optical center.

Declared assets are synchronously validated during preparation. Dynamic
`start_image_decode` work and cold inline/tinted SVG variants perform parsing,
font resolution, rasterization, and image conversion on background workers;
the GPUI image element automatically repaints when a variant becomes ready.
Do not treat construction-time pixel availability as part of the component
contract. Rust Hosts can inspect the bounded shared variant cache through
`AssetRegistry::svg_cache_stats()`. SVG text resolves system-installed fonts and
generic fallbacks; Host-provided in-memory `FontSource` values currently apply
to GPUI text, not to the separate SVG font database.

Rust Hosts drawing a logical image handle through GPUI can use
`AssetRegistry::image_source_inheriting_text_color(handle)` for the same live
foreground behavior. `image_source_tinted(handle, color)` remains an explicit
color snapshot. Neither path rerasterizes a ready cached color on every frame;
an unseen color is still an asynchronous variant, not a GPU color animation.

See [Style](docs/style.md), [Assets](docs/assets.md),
[mutable-image Host bridging](docs/asset-refresh-host.md), [Canvas](docs/canvas.md),
and [Locale and RTL](docs/locale-and-rtl.md).

### 8.1 Read-only documents and Host-owned text

Use `CodeViewer` for a read-only source document and `DiffViewer` for a neutral
left/right comparison. They are native document surfaces, not arrays of Rhai
Text rows: syntax parsing, diff calculation, virtual scrolling, selection,
search and hunk navigation stay in Rust.

CodeViewer, DiffViewer and MarkdownViewer accept direct strings. A Rust Host
with large or frequently changing text registers a revisioned
`NativeTextDocument`, reads it from Rhai with
`ctx.get_native_text_document(name)`, and publishes later revisions through
`ScriptViewHandle::replace_native_text_document`, or appends a streamed chunk
with `append_native_text_document`, after which a MarkdownViewer parses only the
last block and the new text. Only exact subscribed components rerender; native parsing then completes in a cancellable background
job without moving Rhai values across threads.

Dependency ownership follows callback context: the formal component whose
`ctx` calls `get_native_text_document` is the reader. Keep that call in the
smallest component that owns the Viewer when replacing one document should not
rerun an application-level parent.

CodeViewer is deliberately read-only. DiffViewer uses direction-neutral
`left`/`right` descriptors and supports unified/split presentation, strict or
whitespace-insensitive comparison, Unicode-grapheme intraline emphasis, and
expandable equal context. It does not edit, merge, stage, or apply patches.

See [Native document viewers](docs/document-viewers.md),
[CodeViewer](docs/components/code-viewer.md), and
[DiffViewer](docs/components/diff-viewer.md).

`components/markdown_viewer::MarkdownViewer` renders CommonMark/GFM natively:

```rhai
import "components/markdown_viewer" as markdown;

markdown::MarkdownViewer(#{
    key: "reply-42", label: "Assistant reply",
    source: ctx.get_native_text_document("reply-42"),
    height: "content", max_height: 480,
    code_block_actions: ["copy"], on_link: Fn("link_requested"),
})
```

Import `components/markdown_viewer`. Rust owns source, parse models, highlighted
code, measured-height virtualization and selection; Rhai never receives a
block/span array. Long paragraphs, code and cells have bounded native text
segments. GFM tables keep alignment and horizontal scrolling, and task lists,
strikethrough, autolinks, nesting and typography parts follow the same theme.
Streaming jobs coalesce and reject stale revisions/owners. Parsing preserves
complete semantics: a later reference definition may invalidate an earlier
block, not just the last block.

Virtual-row eviction releases the native Viewer, not the Host document. To also
reuse its expensive preparation, implement `MarkdownPreparationCache` and install
it with `engine.document_runtime_config().set_markdown_cache(Some(pool))` from
`configure_engine`. The Host owns the pool's budget, eviction, sharing and release;
there is no implicit/global SDK cache. `PreparedMarkdown` is an immutable model
handle, not a UI element. `MarkdownPreparation::new(...).prepare(&canceled)` lets
Rust pre-prepare the same model on a background executor without creating a
Viewer. Complete source/config keys and stale-job checks stay in the SDK.
See [the cache contract](docs/document-viewers.md#host-owned-markdown-preparation-caches)
for lifecycle, accounting and actual preparation counters.

Select and copy across blocks within one viewer, not across separate messages.
Cmd/Ctrl+C copies rendered plain text; Alt+Cmd+C / Ctrl+Alt+C explicitly copies
the Markdown source. Code actions copy code without fences. HTML is literal,
never executed. Links emit `#{ href, text }` and never open a browser; the Host
must validate destinations. `image_assets` explicitly maps Markdown destinations
to Host asset IDs; absent assets leave alt text. The viewer never fetches URLs.
It is not an editor or a WebView. Document byte/line limits and the configurable
Markdown table-column budget bound preparation.

Register documents as messages arrive on an already mounted view:

```rust
view.register_native_text_document(
    "reply-42", NativeTextDocument::new("reply/42", 1, initial_text)?, cx,
)?;
view.replace_native_text_document(
    "reply-42", NativeTextDocument::new("reply/42", 2, next_text)?, cx,
)?;
// After the application removes the message/readers:
view.release_native_text_document("reply-42", cx)?;
```

Registration rejects duplicate names; replacement of one identity requires an
increasing revision. Caught lookups of a missing name establish bounded pending
reader dependencies, so later registration invalidates exactly those readers.
Release removes registry ownership without invalidating immutable snapshots
already held by a viewer/job. Its boolean reports reader invalidation: successful
`false` still released the snapshot. Disposal, reload and failed candidates use
the existing ownership/rollback rules. No fixed document pool or raised Rhai
string, collection or operation limits is needed; prefer native documents beyond
the direct-string budget.

### 8.2 Wall time, date/time ranges and histories

Wall time is independent of the monotonic Motion/test clock and is available
without the optional charts feature. `WallClock::system()`, `fixed(epoch_ms)`
and `from_source(...)` provide the Rust presentation clock; both
`ScriptView` accepts `.wall_clock(clock)`. A fixed
or custom source makes application tests deterministic without changing time
arithmetic or animation speed.

Use signed epoch milliseconds for instants. Rhai exposes `ctx.now()`,
`ctx.format_time(ms, "short" | "medium" | "long", zone)`,
`ctx.format_relative(ms)` and `ctx.format_duration(duration_ms)` with selected
locale digits, hour cycle, units and relative messages. Zones use `"local"`,
`"UTC"`, `"offset:<minutes>"` or an available IANA name such as
`"Asia/Shanghai"`. `time_parts(ms, zone)` converts an instant to civil fields;
`time_local_candidates(date, time, zone)` returns zero, one or two instants,
making DST gaps and repeated local times explicit.

`Calendar` controls ISO date strings, with locale week starts, keyboard day/week
and month movement, single/range presentation, one/two months and explicit
`min_date`/`max_date`. `TimeInput` controls localized clock *text*: incomplete
typing is retained rather than clamped or replaced. Parse it with
`ctx.parse_time(value, "minute" | "second")`, which returns `()` while invalid
or incomplete. Up/Down steps parseable values by the configured precision.

`DateRangePicker` (`gpui-rhai add date_range_picker`) edits two inclusive civil
dates, not instants or durations. Pass a controlled
`value: #{start:"2026-10-01", end:"2026-10-08"}` and a `compare_mode` of
`"off"`, `"previous"`, or `"year"`. A preset only edits the open draft.
Apply emits `#{range, compare_mode, comparison}`; persist the accepted range
and mode together. Cancel or Escape discards the draft. Increment `revision`
to withdraw an open edit even if the committed value did not change.

The previous period has the same number of days and ends the day before the
selection. Previous year shifts both endpoints by 12 civil months, using the
last day of the destination month if necessary. `comparison` is `()` when
disabled, off, or outside the representable calendar. Optional inclusive
`min_date`/`max_date` constrain the selected range; comparison dates may
precede those limits. With `max_date` omitted, today is the last selectable
date. Pass `today` for a reproducible comparison or reporting cutoff. The shared
Calendar uses two months at wide sizes and one with scrolling presets in a
narrow popover.

`NumberInput` takes a controlled **number**, or `()` for an empty field.
Install it with `gpui-rhai add number_input`. It keeps partial typing locally;
Enter or blur emits a finite localized numeric proposal, clamped to optional
`min`/`max`. Invalid drafts restore the controlled value; Escape cancels a
draft. The two step buttons and arrow keys follow the same grid, based at
`min` or zero. Hold a step button to repeat after a short pause; release,
leaving the button, deactivating the window, or reaching the limit stops it.
The press cadence still works with motion effects disabled.
Page Up/Down applies a larger step. Home/End go to a bound
when it is present. The `unit` is presentation only and isn't part of the
value. The independent parser `ctx.parse_number(text)` returns `()` for
invalid or empty input; grouping, digits, and separators follow the locale.

`StepperForm` (`gpui-rhai add stepper_form`) presents keyed steps with
caller-owned content, for example `FormField` and `Input` controls using your
existing values. A step may provide an `error` message that blocks forward
movement, and `on_invalid` lets you mark its individual fields invalid.
The form requests focus only within the current pane. Back keeps its native
editors, and a review row with `edit_step` returns directly to the review
after an accepted Continue.

Use local navigation or echo the proposed `step` through `on_step_change`.
The last action's `on_submit` lets you read the app's values and start the
operation. Set `status: "submitting"` when accepting an asynchronous
operation, then set `"succeeded"` or `"failed"` after validating the result
against your active request. The Stepper does not perform I/O or dispose your
work. A failed operation preserves the draft and permits a retry.
`on_reset` restores your data and status. At a measured width of 420px or
less, a compact progress summary replaces the full strip without rebuilding
the current pane.

`ProgressTracker` (`gpui-rhai add progress_tracker`) presents a flat checklist
with a progress meter. Each step has a nonempty, unique `id` and a `label`;
`done` seeds local state and `disabled` prevents edits without excluding it
from the total. To control the list, supply `value` (completed IDs) and update
it in `on_change`. A rejected proposal does not change the meter. Reordering
steps preserves completion by ID. Demonstrations may opt into `autoplay`;
pointer input or focus within the tracker stops it without undoing progress.

`SetupGuide` (`gpui-rhai add setup_guide`) uses the same contract for
`value` and `on_change`, with tasks organized into groups. Group IDs are
unique, and task IDs are unique across the whole guide. Only the next
unfinished task in each open group can be checked. A completed group hands
off to the next unfinished group once its change is accepted. Manual
disclosure or minimization supersedes that handoff. Optional `on_edit` and
`on_close` callbacks let the app supply those actions; minimizing leaves
completion and the meter intact.

`FilterMenu` (`gpui-rhai add filter_menu`) presents grouped multi-select
filters. Each group has an `id`, `label`, and ordered `options` with separate
IDs and labels. Set `value: #{status:["active"]}` and store accepted
`on_change` maps yourself, or omit `value` for local selection beginning at
`default_value`. Option IDs are local to a group. Disabled options remain
visible and cannot be changed by the row, a chip, or Clear all. Opening a group
does not propose filters. The application applies the selected IDs to its own
data.

Up/Down move through visible groups and options; Home/End go to either end.
The forward arrow opens a group and the backward arrow folds it. Enter and
Space toggle the focused row. Modern wraps selected chips; Clean keeps a
measured single row and shows `+N` for the omitted chips. Every selected value
is still reachable through its group regardless of chip overflow.

`TagInput` (`gpui-rhai add tag_input`) edits a controlled ordered list of
distinct strings. Return the accepted `on_change` list as `values`. Enter,
forward Tab, a separator or leaving the field commits its draft. Paste a
comma-, semicolon- or line-separated list to propose all valid additions at
once. Unaccepted and invalid text stays editable. Set `split_on_space` for
single words or recipient addresses, not for tags that contain spaces.
`normalize: "lowercase"` trims and lowercases before comparing tags; the
optional `validation: "email"` check looks for a local part and dotted domain,
not mailbox ownership. `max` bounds additions without disabling removal.
Two Backspaces in the empty editor remove the last chip; at the start of the
editor, the backward arrow enters the chips in the locale's direction.
The editor remains a single native text field, including IME and selection.

`OtpInput` (`gpui-rhai add otp_input`) edits a single verification code,
even when it draws a row of cells. Pass a controlled string so leading zeroes
survive. Paste a complete code to replace the field; tab navigation, keyboard
selection, and input-method composition all go through the native editor.
`on_complete` proposes a newly entered full code; your application performs
verification and sets `status` to `verifying`, `error`, or `success`.
With `on_resend`, the component shows a resend action and an active-view
cooldown. An error can propose a clear; setting a new code or status withdraws
that pending clear. Do not log the code or place it in display labels.

Modern and Clean animate newly accepted characters and the focus outline as
paint only; keyboard, pointer, and IME positions still refer to the actual
slots. On success, the slots keep their measured footprint, while the status
surface converges into the confirmation pill. The text editor is no longer
exposed as editable in that state. With motion disabled the control settles
straight to the same result.

An empty focused slot blinks only while it is actually presented in the
active window. An opted-in error clear fades the rejected code from the
right before the same clear event; it does not rewrite the value during the
fade or outlive a replacement attempt. Reducing motion keeps the caret
legible and removes the fade without changing the clear deadline.

`TimeRangePicker` takes one of these controlled values:

```rhai
let relative_range = #{ kind: "relative", duration_ms: 900000 };
let absolute_range = #{ kind: "absolute", start: 1791947400000, end: 1791952200000 };
// Resolve only when issuing the application query:
let interval = time_range_resolve(relative_range, ctx.now());
```

Relative presets remain relative; do not persist the temporary resolved pair as
their definition. Custom absolute edits remain a draft until Apply emits a
complete validated range; Escape/cancel does not change the controlled value.
Changing `revision` also replaces an open draft when the value itself is equal.
Optional `zone_switch` changes draft presentation and `shift_buttons` proposes
adjacent absolute windows. `min`/`max` bound absolute instants and
`max_span_ms` bounds duration. Nonexistent local times are errors; ambiguous
ones require an explicit occurrence/offset choice. Future times are valid by
default, not silently clamped to now.

`Timeline` presents bounded keyed stage/event records with nested `children`,
optional caller-owned `details`, controlled `expanded` and `selected_keys`,
status/current-step semantics and virtualized rows. `order` preserves or reverses
sibling source order; it is not a timestamp sorting service. `time_display`
chooses absolute, relative or duration labels, and `day_headers` supports
caller-ordered feeds. Current step, keyboard cursor and accepted selection are
different states. Pass `visible: ctx.is_visible()` when a retained page should
pause relative/duration presentation ticks while hidden. These ticks only
refresh labels: none of the time controls fetches business data or starts
automatic query refresh.

## 9. Design tokens, themes and Theme Studio

Components consume symbolic roles and theme-owned part recipes:

| Layer | File | Holds |
|---|---|---|
| Shared base | `ui/tokens.rhai` | geometry, typography, motion, semantic aliases and default component recipes |
| Full preset or palette | `ui/theme.rhai` (and `ui/themes/`) | Modern/Clean include complete geometry, type and recipes; alternate palettes change colors over the shared base |
| Host preferences | `ThemeTokenOverrides` | deliberate preferences over every loaded variant |
| Application styles | `ui/styles.rhai` | component/part refinements above preset recipes and below instance overrides |

Select family `Modern` or `Clean`, variant `Light` or `Dark`. The Gallery's
corresponding keys are `modern-light`, `modern-dark`, `clean-light`, and
`clean-dark`. Changing the design intentionally reflows: an md Button is 40px
in Modern and 28px in Clean; an md field is 36px and 32px respectively. Modern
and Clean are not density aliases. A light/dark change within one design keeps
its geometry. CLI `init` starts in Modern Dark and installs the licensed Inter
faces under `ui/fonts/`; embedded applications register their font resources
explicitly with `ScriptView::font_sources`.

Themes may vary tokens through inherited environment axes, such as `density`
(`comfortable`, `compact`) and `size` (`xs` to `lg`). Most controls also take a
`size` prop. The complete presets use their explicit size steps; declaring a
density axis alone does not promise that every theme changes geometry with it.
A third axis, the reading scale `scale` (`"100"`, `"110"`, `"125"`, `"150"`,
`"175"`, `"200"`), enlarges ordinary length-token references and typography roles together:
an application offers it as a reading-size setting with
`.env(#{ scale: state.scale })` on its root, and a Host sets the default for
every view through `ThemeTokenOverrides::environment_defaults`. Literal
lengths such as `px(320)` keep their size.
For a structural hairline, `theme_length("metrics.grid").without_reading_scale()`
keeps theme, size/density and explicit multiplication live while skipping only
the reading-size multiplier. It does not turn off device/display scaling.
Corners and insets are component-specific. Nonvisual layout/interaction
wrappers do not acquire decorative corners; Button, field, Card and overlay
families have their own aliases. Prefer those roles over a universal
control-height or radius-by-height formula:

```rhai
style()
    .min_height(theme_length("metrics.field_h"))
    .padding_x(theme_length("metrics.field_pad"))
    .gap(theme_length("space.related"))
    .radius(theme_length("metrics.field_radius"))
    .background(theme_color("surface_raised"))
    .typography("control")
```

The complete geometry/type/state specification is maintained in
[docs/design](docs/design/). An application with its own design can skip the
token base entirely; see the `byod_treemap` example.

Rust Hosts may layer user preferences uniformly over every default, bundled,
and user-supplied variant with
`ScriptView::theme_token_overrides`. `ThemeTokenOverrides` partially
merges semantic colors, spacing/radii, typography roles, motion roles, and
namespaced tokens and component recipes; it never changes theme identity. Overrides are validated on
startup and reapplied after file-theme hot reload. Use this for preferences
such as UI font/scale, corner scale, motion timing, and visualization palettes;
continue to use `styles.rhai` for component-specific structure and styling.

One namespaced path has one typed value. A single layer cannot declare
`colors["brand.part"]` and a non-color `brand.part` (for example a Style) at
the same time; this now returns a configuration error instead of dropping a
value. Across separate base, theme and Host layers, the later layer wins even
when it changes the kind. Consumers must still use the matching typed getter.

Use `style().typography(role)` with a role of the token base (`caption`,
`body_small`, `body`, `subtitle`, `title`, `heading`, `display`,
`display_large`, `label`, `key_legend`, `code`, `control`) instead of copying font sizes and
line heights into components. Theme values remain symbolic until
rendering. Switching a ThemeVariant advances
the theme generation and repaints/rerenders affected native content without
recompiling Rhai or discarding component state.

A tracked theme read rerenders a component; it does not automatically rebuild
literal colors already stored in an application cache. If an effect prepares
cached paint, include the palette inputs it actually uses in its explicit
plain-data dependencies and invalidate every affected paint slot there.
Do not rely only on the application's theme-picker callback: Host selection
and system appearance changes can bypass it. Keep geometry caches independent
when only paint changes, and prefer late-bound `theme_color(...)` expressions
when the cached representation supports them. See [theme dependencies](docs/theming.md).

Use `ui/styles.rhai` when the customization belongs to a formal component
rather than a universal token. A rule targets an exact component ID and one of
its declared parts with the normal typed `Style` builder. File-backed views
discover it automatically; production embedding passes
`COMPONENT_STYLES_SOURCE` to `ScriptView::component_styles`. See
[Component stylesheets](docs/component-styles.md).

Color expressions support `foreground.over(background)` (also
`over(foreground, background)`) for encoded-sRGB source-over composition. Unlike
`mix`, it accounts for both alpha channels and keeps theme leaves late-bound.
For readability calculations, first composite a translucent surface onto its
actual opaque backing: `wash.over(theme_color("surface_raised"))`. This computes
a color value, not a rendering group or a backdrop filter; keep the actual
surface's transparency when painting it. See the script API's `readable` limits
before evaluating contrast against an uncomposited translucent color.

For a custom focusable UiNode, `style().focus_ring(px(2), px(2),
theme_color("focus_ring"))` adds an exterior, non-layout indicator. It follows
`ScriptViewConfig::focus_indicators`, leaves the offset gap transparent and
does not change the node's bounds or hit area. Inside `group_focus` or
`focus_within`, the ring follows that explicit focus scope instead, so a
Checkbox box or Switch track can follow the actual owner without another
tab stop. Merely inheriting focus does not activate every descendant's base
ring. Use theme lengths when the ring should follow reading scale. Slider
and RangeSlider thumb parts share this decoration while retaining their
native focus handles; other native parts follow their handler's contract.
`focus_ring(px(2), offset_px(-2), color)` paints inside the border box instead.
Auto/relative offsets are invalid. `style().grid_cell(column, row)` places a
grid item at one-based indices; several children may share a cell and reserve
their maximum intrinsic size. Inactive measurement copies can use
`invisible()` to keep layout without exposing duplicate semantic content.

Open Theme Studio with:

```text
gpui-rhai theme-studio
gpui-rhai theme-studio ui/theme.rhai
```

Theme Studio supports:

- New, Open, Import-as-copy, and Save for gpui-rhai `.rhai` themes;
- live semantic-token preview;
- derived secondary roles from core anchors;
- contrast warnings without secret color rewriting;
- every official component and important visual state in one specimen.

The bundled catalog includes Modern, Clean, Default, Tokyo Night, Catppuccin, Ethereal,
Everforest, Gruvbox, Hackerman, Nord, Retro 82, Hermarchy, Futurism, and
Aetheria variants. See [Bundled themes](docs/design/themes.md#6-bundled-themes),
[Theming](docs/theming.md), and [Theme Studio](docs/theme-studio.md).

## 9.1 Native charts

Enable the optional `charts` Cargo feature or run `gpui-rhai add chart` to use
the source-owned `charts/chart` component over the native Chart Runtime. Small
Rhai object arrays are cached as typed columns until their durable content
changes; large and streaming data remains
Rust-owned in `NativeChartData` and is read with
`ctx.get_native_chart_data(name)`. Pointer/wheel hot paths, transforms,
downsampling, layout, motion, hit testing, linked charts, accessibility
projection, and export do not execute per-datum Rhai.

Selection and viewport props are controlled. A gesture emits one proposal at
commit; the next Host render accepts, clamps, or rejects it by writing the exact
coordinate-typed `viewport` and `viewport_revision` back. Scalar zoom/pan is
only shorthand and cannot represent every independent X/Y window. Selection and brush
events include dataset, series, region, datum, and presented revision identity,
so Hosts should not infer chart control roles from application key strings.

The first release includes Bar/Stacked Bar, Line/Area, Scatter, Pie/Donut,
Heatmap, Map/Choropleth, GeoScatter, GeoLines, Candlestick, Radar, Gauge, and
Funnel over Cartesian2D, Polar, and Host-supplied Geo2D maps. See
[Chart Runtime](docs/charts.md) for specifications, extension APIs, performance
contracts, and the explicit deferred boundary.

## 10. Overlays, focus, and multiple views

Combobox, Select, DatePicker, Menu, Popover, Dialog, Tooltip, and HoverCard use the generic
Overlay mechanism. Toast uses the generic Layer mechanism plus declarative
timers.

Use `gpui-rhai add hover_card` for an interactive preview rather than putting
actions in a Tooltip. It opens after pointer dwell or keyboard focus without
taking focus from the trigger. Tab enters open content; Escape leaves it.
A press on the trigger keeps its normal action. Set `open` and
`on_open_change` to control visibility, or omit `open` for local state.

The Rust Host owns:

- portal order and window-level placement;
- flipping and clamping;
- modal and parent/child dismissal order;
- outside click and Escape routing;
- focus trap/fallback and approved key bindings.

Rhai supplies stable local IDs, controlled open state when appropriate,
content, placement policy, and callbacks. IDs are automatically namespaced by
`view_id` inside a shared host.

`Input.autofocus` and `Textarea.autofocus` apply when their keyed native entity
is first mounted. Overlays retain their content identity while closed, so a
reopen must use the overlay lifecycle policy instead: set
`initial_focus: "first"` to move from the panel to its first focusable
descendant on every closed → open edge. The historical default is
`initial_focus: "panel"`. A structural container with key/click handlers gets
an interaction wrapper and is a tab stop by default; use `tab_stop(false)` when
that container should not precede an inner filter input in the focus order.

An anchored overlay can opt into `match_trigger_width: true`. Its native
wrapper then follows the styled overlay-root allocation and provides that width
to both the trigger and popup. Combobox uses this policy so `relative(...)` and
caller-applied flex growth track the parent layout instead of collapsing to
content width. `min_trigger_width: true` lays the trigger out the same way but
makes the popup only at least that wide, so wider content widens it; a Select's
list uses it. The two exclude each other.

While a modal is open, Overlay reasserts that focus remains inside its panel on
every focus-driven GPUI frame. An embedding Host must not repeatedly focus an
ancestor to keep shortcuts alive; register Host shortcuts on the surrounding
interaction domain instead. If a one-off Host focus request races with mount,
the modal reclaims focus so Escape and Tab continue through the modal path.

Do not simulate overlays by absolutely positioning a child inside a clipped
component. Use the generic Overlay/Layer path so the Host can coordinate the
whole window.

## 11. Async, performance, and high-frequency events

Rhai 1.26 is configured without its `sync` feature. Engine, Dynamic, FnPtr, and
stored invocation contexts stay on the GPUI foreground thread. Background work
must move typed Rust data, not Rhai runtime objects.

One-shot capability work uses the bounded shared worker pool. Return
`TaskWork::new` for ordinary work, or `TaskWork::cancellable` when a longer job
can periodically observe its `TaskCancellation` token. Cancellation removes
callback authority immediately and signals the token after the surrounding UI
transaction commits; rollback keeps the old task alive.

```rust
Ok(TaskWork::cancellable(move |cancel| {
    for item in items {
        if cancel.is_cancelled() { return Err("cancelled".to_owned()); }
        process(item)?;
    }
    Ok(UiValue::Null)
}))
```

The retained Rust diff makes GPUI updates efficient, but Rhai still constructs
the declarative tree for every component that actually reruns. Root-dirty
renders bail out unchanged formal component subtrees before Rhai execution;
the retained node diff then handles native changes after rendering. Helper
functions remain part of their owning component boundary.

Guidelines:

- Keep `view` and ordinary callbacks short; 16 ms is the default slow warning.
- Do not parse files, access the network, or perform blocking work in `view`.
- Use `virtual_collection` for large lists and Tables.
- In Table column widths, `fixed` values are pixels, `percent` values are
  percentages of the Table viewport, and `flex` values are positive weights over the remaining viewport
  width; use 1/2 rather than pixel-like values such as 100/200.
  Header and realized rows share one native column plan and horizontal extent.
  Percentage widths never use the growing scroll extent as their basis. Wide
  tables stay clipped and horizontally reachable in LTR and RTL; switching
  loading/empty/data retains the logical offset, clamped to the current range.
  LTR↔RTL locale changes also preserve distance from the logical start; a
  smaller range clamps that distance rather than resetting only one direction.
- Set `resizable_columns: true` for native divider dragging. Per-column
  `resizable`, `min_width`, and `max_width` refine the policy. Pointer movement
  stays on the native signal path; `on_column_resize` runs once on release with
  a fixed pixel descriptor that the caller may persist. Focused dividers use
  logical Left/Right in 8px steps.
- Table dividers sit at the logical column boundary, including the final or
  sole column. Inline `inset_start`/`inset_end` styles resolve in LTR/RTL and accept
  definite, `auto()` or signed `offset_*` values. Each divider has its own stable
  element ref for Host-native focus; its measurement ref still belongs to the
  column header.
- Sortable headers contain independent inset buttons: the `columnheader`
  names the column, while the `button` named `Sort {name}` owns sorting and
  keyboard activation. Header padding and resize handles are not sort targets.
  `header_action`, `sort_icon` and `sort_icon_active` are public parts. The
  `table.*` paint roles are table-local; generic lists use
  `collection.selection`, not the table's authored blue selection fill.
- Treat `virtual_collection` as presentation-only. The owning component defines
  enabled items, keyboard navigation, the single active style, and passes its
  controlled key as `reveal_key`; sticky rows are a separate layout policy.
- Virtual row namespaces supply identity, not a new callback/state owner. Raw
  row callbacks and app/window store path subscriptions belong to the caller's
  executable component; formal row components own their own state/resources.
  Transparent wrappers may return a formal child directly without adding a
  layout container. Nested collections retain their current realized windows
  across parent reuse; deleting a parent also removes its nested targets.
- Raw row reads wake the executable owner, but their subscriptions belong to
  each live data-key contribution. Scrolling removes old contributions without
  clearing sibling collections or the root's own reads. Sequential and batched
  virtual targets have the same final state, callback and resource lifetimes;
  failed batches keep the prior committed target and dependencies.
- Keep large stable row sets in `NativeCollection`; let Rhai declare the Table
  and controlled state while Rust caches sort/group/collapse order and projects
  only visible rows.
- Array and NativeCollection Tables both execute an explicit controlled `sort`
  in Rust. `sort.key` names a declared display column; that column's optional
  `sort_key` names the actual scalar rank field, defaulting to the column key.
  For example, display `duration: "1 h 5 m"` while comparing
  `sort_key: "duration_seconds"`. Missing/null ranks stay last in ascending and
  descending order, equal ranks preserve source order, and invalid non-null
  rank types reject the candidate. String comparison is byte order; supply a
  lowercase rank field for case-insensitive order. Sorting precedes controlled
  filtering, paging and grouping, without changing display fields or header
  event keys. Previously Array rows ignored `sort`: remove duplicate caller
  sorts now, or omit `sort` / pass `()` to preserve caller order. Large Arrays
  still have ordinary Rhai limits; native sorting is not permission to raise them.
- Register names known at mount through the extension. For names discovered
  later, call `ScriptViewHandle::register_native_collection`; a component that
  previously handled the missing-name error is tracked and invalidated exactly
  when registration succeeds. Use `replace_native_collection` only for an
  already registered name. Missing-name dependencies are render-only and
  bounded; do not probe unbounded dynamic names from event callbacks.
- Escape cancellation belongs to the Host interaction domain, so active
  pointer/wheel/drag work remains cancellable when a virtual source goes
  offscreen or keyboard focus moves. With no active interaction, Escape remains
  available to dialogs, overlays and Host shortcuts.
- PanZoom zoom anchors and SelectionArea hit tests use the Canvas drawable
  rectangle after padding/border and pixel snapping. Decorating the viewport
  does not move the point beneath the pointer while zooming. An occluding
  rejecting child inside a virtual destination blocks drag auto-scroll;
  genuine open gaps may continue only that session's accepted destination.
- Use Table `query`/`search_fields` and `page`/`page_size` for controlled
  Rust-side filtering and paging; reset page one when query or sort changes.
- Keep large or rapidly replaced source text in `NativeTextDocument`; direct
  strings remain the simple path. CodeViewer, DiffViewer and MarkdownViewer
  compute from immutable typed snapshots on background workers and commit
  complete matching revisions.
- Use Table's `group_by`, `collapsed_groups`, and `on_group_toggle` contract for
  row grouping. Group values are non-empty strings from a declared column;
  headers count as virtual items, skip selection, and stick by default through
  the generic virtual-collection mechanism.
- Use native pointer/wheel handlers, Motion, and `NativeSignal` for coalesced
  or per-frame values such as playheads and drags.
- Bind native signals directly to supported properties. Sampling a signal with
  `ctx.get_signal` during render disables bailout for that component subtree,
  because signal reads intentionally do not create rerender dependencies.
- A native handler bypasses Rhai dispatch, but if it dirties a Rhai component,
  that component still runs on the update path.
- Use a retained native subtree or signal-bound property when Rhai must leave
  the hot path entirely.
- Measure complete work, including Rhai evaluation, retained reconciliation,
  GPUI layout, prepaint, and paint.

See [Performance](docs/performance.md), [Virtual lists](docs/virtual-list.md),
[Rust-owned collections](docs/native-collections.md), and
[Native document viewers](docs/document-viewers.md), and [Custom
primitives](docs/custom-primitives.md).

### 11.1 Shared scrollbars and growing virtual content

`ScrollbarSource` is the public Rust adapter for authoritative native scroll
state. Built-in adapters cover `ScrollHandle`, `UniformListScrollHandle` and
measured `ListState`. A custom primitive obtains a themed factory from
`PrimitiveContext::scrollbars(theme)`; a Host surface uses
`ScriptViewHost::scrollbars(scope, theme)` under that Host's `container`.
Render the viewport first and its `ScrollbarContext::element(id, source, spec)`
as a sibling overlay in a relative clipped wrapper. `ScrollbarSpec::new` accepts
`"auto"`, `"always"` or `"hidden"` for each axis. `with_part_styles` uses the
same track/thumb/hover parts as official surfaces; keyboard focus and semantic
navigation remain with the viewport, not the thumb.

Keep scope/element IDs stable and unique. Retain and clone a
`ScrollbarSourceHandle` alongside a replaceable native source; constructing a
new wrapper gives a replacement source a new drag lease. Raw GPUI adapters
remain supported, but their scrollbar ID must change on source replacement.
Measured-list dragging calls the native height-freeze hooks and cleans up on
release/cancel, disappearance, suspension, disposal or window deactivation.
The factory never owns a second offset or follow state. `with_scroll_axis`
sets wheel ownership independently of bar visibility; `with_scroll_parent`
adds Host-native ancestors nearest-first for residual wheel routing. SDK
primitives receive their retained ancestor chain automatically. The private
interaction coordinator stays private.

For a growing transcript, set `virtual_collection`'s `alignment: "bottom"`
and `follow_tail: true`, with stable data keys and an estimated row height.
Reader input away from the tail pauses native following; reaching the bottom
or `ctx.scroll_collection_to_end(key)` resumes it. The growing last row may be
taller than the viewport. Payload-only changes and row realization do not
reapply a scroll-to-end or steal the reading position. Keyed insert/delete/
reorder and remeasurement preserve a surviving reader key and intra-item offset.

`on_follow_change: Fn("follow_changed")` receives `#{ following, unseen }`.
Unseen counts distinct newly added keys while paused, not streamed chunks or
rerenders; deletion removes unseen keys and resuming clears them.
`ctx.collection_follow_status(key)` is an event-time query, not a render
dependency. `ctx.scroll_collection_to_start(key)` pauses following, and
`ctx.scroll_collection_to_key(key, item)` reveals one row and pauses it until
the reader returns to the bottom; the scroll commands are component-local and
execute after a successful transaction.
`follow_tail` and controlled `reveal_key` are mutually exclusive. A Rust Host
can resolve `ScriptViewHandle::collection_scroll_handle(id, cx)` for status,
start/end, pixel scrolling or `pause_following`; resolve again after replacement
or unmount. Do not mirror that position/follow policy in another Rhai cache.

### Motion is a native runtime path

Motion uses `motion_transition`, `motion_spring`, `motion_keyframes`, and
`motion_inertia`, attached with `.motion(...)`. The old `.animate(...)`,
`transition`, `spring`, and `loop_transition` API was removed in 0.1.3
(Runtime API 2) and does not exist.

Use stable node keys and keep one source per property. Rhai declares sources,
timelines, trigger intent, and callbacks; Rust samples frames. Do not build a
timer loop that repeatedly calls Rhai to update an animation.

Use `.motion_on_change(content_key)` for finite content entrances that should
start at rest on mount/remount. Only a new key replays from the declared start;
equal keys retain progress and source-only recipe changes settle without an
entrance. `.motion_replay_key(key)` also plays on mount. The last of these
modifiers wins. They share native sampling, budget rollback and lifecycle
ownership; neither implements intrinsic-width morphing or text blur.

Use `motion_hover`, `motion_press`, `motion_focus`, `motion_in_view`,
`motion_viewport`, and `motion_scroll` for native trigger/progress paths. Use
explicit timelines only when sequence/parallel/stagger, seeking, or completion
callbacks are actually needed. Enter and exit require stable keys; exit ghosts
are paint-only and own no callbacks or resources. Layout motion is opt-in so
direct manipulation never lags behind the pointer.

Native press feedback belongs to its declared node, not an implicit ancestor
group. Releasing outside the node still returns it to rest; disabling cancels
its held target. Retained-page hiding, suspension and removal of the binding revoke the
transient press lease, so showing or re-enabling cannot revive an old mouse
press. Unrelated sources retain their own active time. The first presentation
uses each progress source's declared inactive value, not a one-frame style
default. This does not add arbitrary subtree scaling to the pinned backends.

Resolve timeline handles inside the declaring component callback. They are
bound to the current view, component incarnation, script generation, and live
timeline instance; never cache one across remount or reload. `play_motion`
resumes/idempotently keeps the current position, while `restart_motion` is the
only rewind operation. Completion and cancellation callbacks run after the
sampled frame commits. Compatible progress migrates to a newly
generation-bound handle during reload; the previous handle becomes stale.

Mounted motion follows retained NodeId rather than concatenated user keys, so
reorder preserves identity while a true remount restarts it. Keys containing
slashes, numeric text, or `item:` are safe.
If a timeline-owned child remounts while its parent timeline stays compatible,
the runtime preserves the timeline position and rebinds the track to the new
child. Closing a window completely clears its suspended presentation state;
later reuse of the same window/view ID starts cleanly.

Canvas morph/trim/stroke/clip and rotate/scale/skew share one presented geometry
for paint and hit testing.
Do not combine Canvas node affine motion with a command-level axis-aligned
path clip: the runtime rejects that combination because GPUI cannot preserve
the same transformed clip semantics.

Rotation/scale/skew are Canvas motion properties, not arbitrary SVG/Image
element transforms. For a fixed vector design that must adapt to a theme-sized
drawable, use the shared view-box projection instead of guessing its pixel size
in Rhai:

```rhai
let scene = canvas_scene([
    canvas_rect("hand", 11, 3, 2, 9, theme_color("accent"))
]).view_box(0, 0, 24, 24);
canvas(scene)
    .with_style(style().width(theme_length("metrics.icon"))
        .height(theme_length("metrics.icon")))
    .motion(motion_transition("rotate", 0.0, 360.0,
        #{ duration_ms: 900, easing: "linear", iterations: "infinite", intent: "decorative" }))
```

The native drawable boundary resolves size, padding and border before mapping
the source box uniformly and centering it. Stroke widths, path clips and
presented hits use the same mapping. Raw scene hit tests/path samples remain
in source coordinates, and clipping is still an explicit style/path choice.
Bounds must satisfy the limits in the generated API reference.

Canvas pointer hits and SelectionArea invert the ancestor placement before
undoing Canvas motion and view-box projection. Supply SelectionArea targets
in the same source coordinates as its Canvas commands. The marquee is a
window-space rectangle while it is dragged, including under rotation or
mirroring. PanZoom proposals remain viewport-local; the wheel anchor stays
fixed under the pointer. The Gallery's PanZoom page can mirror the canvas
without resetting its controlled transform.

Canvas solid fills and strokes also accept `current_color()`, a deferred
`PaintColor` evaluated against the owner's actual native foreground during
paint. It follows inherited/own hover and focus without a Rhai rerender, and
`current_color().alpha(0.2)` multiplies the foreground's existing alpha.
Style backgrounds, borders, shadows and layered stops accept the same paint
type; a theme palette entry or text-color declaration does not. Rust command
fields accept `ColorValue::… .into()` or
`PaintColor::current_text()`. See [Canvas](docs/canvas.md#native-foreground-paint).

GPUI's inherited `opacity` currently multiplies individual primitives; it is
not CSS isolated-group compositing. Do not preblend against a guessed parent
color to emulate an opacity group. Group-opacity and other rendering gaps are
tracked explicitly during the design migration.

Theme motion values come from `ctx.motion_duration`, `motion_easing`,
`motion_spring`, `motion_distance`, and `motion_stagger`. Hosts set the upper
bound with `MotionPreference` and the effect tier with `MotionQuality`; scripts
may classify intent but cannot relax Host policy. See [Motion Runtime
2](docs/motion.md) for the complete Rhai and Rust APIs.

`width_fraction` samples a layout width between zero and the containing
block's full width. Unlike `width` (logical pixels), it follows a resize
without restarting its motion. Use `style().clip()` when it acts as a reveal.

`ctx.number_text(value, digits, prefix, suffix, share)` creates a localized
text node for a floating-point number, with at most `digits` fraction digits
and optional affixes. Attach a `number_value` transition to sample the
displayed number natively without rerunning Rhai. The accessible text is the
accepted value, not the transient sample. Set `share` for percentages with
`<1` and `>99` thresholds; pass percentages in units of 0–100 and include
`"%"` as the suffix. Other uses pass `false`.

`patterns/compact_breakdown::CompactBreakdown` compares the total and ranked
parts of an accepted snapshot. Keep `series` and each category's `id` stable
across periods. To enable switching, pass `periods`, the accepted `period`
and `on_period_change`. The callback receives `{series, revision, from, to}`;
the Host should validate both identity and revision, then publish the accepted
period, items and delta together. Keep the original snapshot on refusal. The
Gallery's CompactBreakdown story demonstrates both outcomes and the narrow
layout.

`patterns/usage_meter::UsageMeter` displays a finite accepted quota, its
category breakdown, optional daily values, and capped or unlimited plan
limits. A present daily zero means that day was observed; omit its `value`
if it has not been recorded. Omit a sub-limit's `total` for unlimited,
but pass 0 for a real zero capacity. Keep category/day/limit IDs stable.
It uses the same versioned period proposal as CompactBreakdown. Its
`on_action` receives `{series, revision, period, state, kind}`, where
`kind` is `upgrade` or `top_up`. Validate the request in the Host; no
quota update or purchase happens inside the widget. The daily activity
supports arrow-key inspection, Home/End and Escape.

`patterns/change_summary::ChangeSummary` compares a list of accepted
metric changes. Give each change a stable `id`, a display `label` and its
formatted `change` text. `tone` describes whether the result is favourable,
and `direction` describes the physical change; for example, improved
latency uses `tone:"good", direction:"down"`. Equal display labels may
belong to different metrics. Narrow cards stack label and detail.

`patterns/tips_panel::TipsPanel` keeps the header and optional Close
action outside a capped scrollable grid. Give each tip an ID, plain
description and action label; a formatted `detail` node can include Kbd.
`on_action` emits `{series, revision, id}`; `on_close` emits
`{series, revision}`. Use `pending` for an in-flight tip, and publish
`done:true` only when it succeeds. Keep the previous snapshot on refusal.
The Host owns dismissal and completed state, including across reopen.

`patterns/project_card::ProjectCard` shows an expandable project,
its members and one ring segment for each task. Keep task and member IDs
stable, and use the member ID in `assignee`. A ring segment and its
keyboard-accessible task row propose the same `task_change` payload
`{project, revision, id, done}`. `status_change` proposes a status with
the same project and revision. Neither alters the accepted snapshot.
Omit `open` for local disclosure, or handle `open_change` to control it.
Use `trigger:"click"` if pointer hover should not open the card.

`patterns/experiment_card::ExperimentCard` compares readings against a
single baseline. Omit an unknown reading rather than substituting zero.
Relative change is undefined without a nonzero baseline; direction and
`lower_is_better` are evaluated separately. `metric_change` and
`winner_change` nominate values for the Host to accept. An accepted
candidate enables a separate `action` proposal with the candidate and
metric in the current revision. Visitor counts and confidence are
caller-provided facts, not inferred by this component.

`patterns/notification_stack::NotificationStack` presents the first
notice in an accepted queue. The cards behind it are decorative; their
links and dismiss buttons are not mounted. Handle `on_dismiss` with the
proposal `{stack, revision, id}` and remove that notice only after
acceptance. A notice's optional `action.id` comes back in `on_action`;
the Host owns navigation, including external links.

`patterns/away_summary::AwaySummary` is an inline recap, normally placed
in a conversation. Provide `since_ms`, an interval `session` and accepted
items with stable IDs. New tasks are first grouped by initial priority;
changing a task to done does not move it. Use `pending` for in-flight
actions and `on_action` to receive `{session, revision, id, kind}`.
`on_collapsed_change` carries `{session, revision, collapsed}` and can act
as acknowledgement when the person closes the summary.

`patterns/kpi_chart::KpiChart` compares up to six series over 96 labeled
samples on a single card. Give each sample a stable ID and an optional
long label. Each series takes one value per sample in the same order:
`()` means missing and breaks the path; zero is a valid observation.
Choose number, percentage-point, or fractional-percent labels explicitly.
Update `revision` when the data changes to draw the new paths; retain
`metric` while refreshing the same metric, and change it when replacing one.
The chart's pointer and keyboard readouts do not mutate the Host data.

`patterns/breakdown_chart::BreakdownChart` groups row maps by a dimension
and optionally subdivides each group by a second dimension. Levels have
ordered IDs and labels; unknown or missing dimension values do not match a
level. A measure without a field counts matching records. A measure with a
field sums finite signed values, treating a missing field as zero.
The chart and detail table share one rollup. Legend controls hide segments
from the chart and totals, but preserve the original data in the table.
Local choices and focus survive record updates for the same `dataset` ID.

`patterns/running_now::RunningNow` summarizes running, waiting and
paused tasks in an anchored popover. The caller owns task status and progress.
`on_action` supplies `{board, revision, id, kind}` for pause or resume; use
`pending` to suppress repeated mutations until the Host acknowledges.
`on_open_task` and `on_open_all` are separate navigation intents.
The popover can be locally managed or controlled with `open`.

`patterns/scheduled_runs::ScheduledRuns` shows a weekly backlog and
accepted placements using stable day, session, task and piece IDs.
Provide capacities in integer usage units. The supplied current session
marks which earlier cells are read-only. Auto offers a chronological
first fit; a task larger than one session can split, but every piece
must be placed. A targeted placement starts at the specified session.
Moving a booked piece preserves the other pieces.

`on_propose` receives the complete candidate set, the requested mode,
backlog IDs, the board and its current revision. Accept by publishing a
new snapshot, together with a new revision. Set `pending` while deciding.
The component does not execute jobs or silently schedule on mount.
Manual mode returns exclusively automatic tasks to the backlog, and
does not unbook only part of an accepted task.

`patterns/tool_call_card::ToolCallCard` displays an accepted tool
invocation, not an executable tool. Pass its stable ID and status in
`entry`. Arguments and results may be structured JSON, JSON text or
plain text. The expanded card wraps and highlights the data in a
bounded read-only CodeViewer. “Copy” uses the entire displayed
document, not just the visible rows. Content above 128 KiB is explicitly
omitted instead of copying a partial document. `on_copy` reports only
the session, revision, call ID and section.

`on_retry` asks the Host to retry. Publish a new accepted status and
revision when appropriate, passing `pending` while deciding. For a
batch, `patterns/tool_call_group::ToolCallGroup` accepts the same
invocations in `calls` and shares one view clock. The group does not
mount closed code viewers; it remembers each child's disclosure
choice by ID. `now_ms` can be injected for deterministic elapsed time.

Embedded `CodeViewer` surfaces can set `show_gutter: false` to reclaim
the line number gutter. `show_line_numbers: false` by itself still
leaves an empty gutter for layouts that intentionally reserve it.

`patterns/context_window::ContextWindow` shows accepted token usage
as an ordered field. A part's supplied `items` array takes precedence
over its aggregate `tokens`, even if the array is empty. Totals and
overflow text use exact counts. The grid rounds cumulative boundaries,
so a very small part can still appear in the list without occupying
an entire cell. A zero `limit` means the limit is unknown.

Remove and Compact emit a single `on_request` receipt containing
`context`, `revision`, `kind`, `part`, and `item`. Pass `pending` while
deciding, then publish accepted counts with a new revision. The widget
never compacts or removes data itself. If the Host reports
`saved_tokens` and `compacted`, the card can acknowledge these without
guessing from an unrelated change in usage.

`patterns/memory_panel::MemoryPanel` manages the presentation of saved
memories, not the application's storage or policy. Call it with a stable
`key`, a `workspace` ID and a `memories` array of `id`, `text` and
`category` records. Optional `source`, `date` and `paused` fields are
display metadata. Without an `on_change` handler, the array seeds local
state for the current workspace.

When `on_change` is set, the Host owns the accepted list. Each proposal
contains `workspace`, `revision`, `kind`, `id`, `entry` and `memories`.
Validate workspace and revision, then return an incremented revision
with the accepted list, or the unchanged list to decline. Keep the proposed
ID when accepting an add. A confirmed removal offers a time-limited Undo;
restoring the item is a separate proposal. The independent `enabled` and
`on_enabled_change` pair controls use of memory without deleting records.
Memory values and the proposal's data fields are marked sensitive in the
component's diagnostics. Switch to the MemoryPanel page in Gallery to
try Host acceptance, refusal, and the narrow layout.

`patterns/tool_access_map::ToolAccessMap` displays which tools the
current agent can access and the calls already recorded by the Host.
Pass `agents` and optionally a complete ordered `tools` catalog;
unlisted tools are not accessible to that agent. Provide a stable
`session` and advance `revision` when publishing a newer snapshot.
An `agent` prop controls selection; otherwise selection is local.

The inspector is read-only unless you supply `on_replay`. It emits only
an explicit request for a finished call: `session`, `revision`, `agent`,
`tool`, `call_id`, `format`, `input`, and `edited`. `input` is the
original or edited text (up to 8 KiB). Structured input is checked as
strict JSON and sent as JSON text. The Host must verify that the run,
revision and call are still current, recheck permission and validate
the arguments before deciding whether to act. Gallery shows both
acceptance and refusal without executing any tool.

`patterns/trace_waterfall::TraceWaterfall` presents an execution trace
on one elapsed-time axis. Pass an ordered, flat `spans` array with stable
`id` and optional `parent` IDs, plus `name`, `kind`, `status` and
`start_ms`. A nonempty trace must have exactly one root. Supply
`end_ms` for known completion times; only running spans without a
known end follow the clock. `now_ms` injects a deterministic clock.
Alternatively `started_at_ms` enables elapsed wall time while the run
is active; without either, elapsed time stops at the latest supplied time.

Costs are integer `cost_micros` in the declared `currency`; each span
reports its own cost and token counts. Parent rows sum the entire subtree.
Folding does not change those totals, the time domain, or which finished
call was slowest. Supplying `collapsed` and/or `open_id` makes those
controls Host-owned (an explicit null `open_id` closes the inspector).
Their proposals carry the `session` and `revision`. Optional input,
output, error and detail text is sensitive in diagnostics.

`patterns/smart_recommendation::SmartRecommendation` presents a ranked
set of options. Supply a stable `session`, and for each option an `id`,
`label`, `reason` (text or a UI node), and `signal` from 0 to 3. The
first option is the default recommendation. Choosing an alternative
only changes what is shown; it does not accept or execute anything.

Without `accepted_id`, the card can record and undo acceptance locally.
To control the result, pass `accepted_id` (explicit null for none)
and `on_accept` / `on_undo`. The callbacks receive only `session`,
`revision`, and the option's stable `id`. Set `pending` while the Host
decides and publish the confirmed ID or null in the next snapshot.
Leaving the accepted ID unchanged refuses the proposal. The framework
never performs the recommended operation, and the private reason is not
included in either event.

`patterns/rate_limit_planner::RateLimitPlanner` forecasts a shared
tokens-per-minute budget over a 24-hour work window. `start`, `now`,
and every run's `at` are minutes after midnight. `actual` is a
minute-indexed sample array from the start of the window, distinct
from the forecast. The peak and overload intervals use a
minute-resolution model, even when the chart draws fewer points.

Runs that have already started cannot be moved, edited, or removed.
Drag previews are native; releasing proposes an accepted-plan
replacement once. `on_runs_change` receives `session`, `revision`,
`operation`, optional `id` and `runs`. In controlled mode, validate
that revision and publish the accepted snapshot yourself. The
planner never executes a task or changes a provider quota. To allow
purely local planning, set `controlled: false`.

`patterns/root_cause_tree::RootCauseTree` displays a fault tree and its
accepted evidence. Pass an ordered `causes` array with unique `id`
values and `parent` references. Exactly one cause has no parent; it
represents the failure. Use an `and` gate when every child must be
proven, or use `or` when any one child is sufficient. A non-root
internal cause must also be confirmed by the Host. A ruled-out branch
never belongs to a proof, even if one of its descendants is confirmed.

The component displays the investigation but does not perform one.
It offers Fix and Retry only for confirmed leaves on a complete proof
path, and only when `on_action` is supplied. The callback receives
`session`, `revision`, `id`, and `kind`, not the evidence or paths.
Set `pending` while deciding, then publish a new accepted snapshot.

`patterns/judge_bracket::JudgeBracket` shows up to sixteen contenders
and compares their outputs. Pass contenders in seed order and accepted
verdicts keyed by zero-based `round:match` (for example, `"0:0"`).
Each verdict contains the current ordered entrant IDs `a` and `b`,
and a `winner` ID. If an earlier decision changes the pairing, the
old verdict and scores no longer count. Empty seed positions can
produce byes; unresolved matches cannot. Use `capacity` to preserve a
2-, 4-, 8-, or 16-slot bracket before all entrants arrive.

The comparison never runs or applies a draft. When `on_pick` is
supplied, the champion button emits only `session`, `revision`, `id`,
and `match_id`. Validate that proposal in the Host and set `pending`
while waiting for a decision. Match selection can remain local or be
controlled through `selected_match` and `on_selection_change`.

`patterns/collaboration_stream::CollaborationStream` presents a
chronological conversation. Supply stable `session`, participant IDs,
and message IDs. The reserved IDs `you` and `all` represent the local
reader and all recipients, respectively. `live` starts with the newest
message; scrolling back or opening a quote keeps the reader's place when
new messages arrive. The unread button resumes native tail following.

Pass `on_send` to show a composer and `on_change` to accept draft
edits (the callback receives `{ text }`). The send proposal contains
`session`, `revision`, `text`, `to`, and optional `reply_to`. Accept
it by publishing the new message and clearing the draft, or leave the
draft as-is on rejection. An attachment preview uses only supplied text;
its label never opens a path, web link, or tool.

`patterns/conflict_radar::ConflictRadar` displays branch activity by
file and agent. Edits have stable IDs, revisions, and inclusive one-based
`start` and `end` lines. Only edits from distinct agents can overlap.
Separate bands in a file are resolved independently; the component
does not widen overlaps into the surrounding edits.

For controlled resolution, pass `resolved: #{}` and accept proposals
by storing `{ agent, witness }` under their `overlap` ID. A witness
becomes stale if a contributing edit changes its revision or range, or
if the file path changes.
`on_merge` receives `session` and `revision` only when there are edits,
all overlaps are accepted, all agents are done, and the Host is not
pending. Validate the latest snapshot before acting. The component does
not open file paths or merge branches.

## 12. Hot reload, errors, and production

Development reload is transactional:

- changed modules and dependants compile in one uniquely identified preparation
  session;
- success swaps the generation and preserves compatible state;
- failure keeps the last-good AST, callbacks, tree, effects, and state;
- old callbacks and asynchronous deliveries become stale.

The runtime distinguishes program generation, logical component path, mount
incarnation, and per-view presentation domain. These identities are not
interchangeable: two windows may have the same tree-local NodeId, and a
same-key remount may have the same path, without sharing geometry, capture, or
callback authority.

Embedded hosts should read `ScriptViewHandle::last_error()` alongside
`root()`. After a failed callback, delivery, or render, `root()` intentionally
remains the last-good tree and `last_error()` explains why the latest candidate
was not committed.

For a machine-readable failure, read `ScriptViewHandle::last_diagnostic()` in
the same foreground App context. It belongs to the exact same failure as
`last_error()` and, like it, stays until a successful reload or
`ScriptViewHandle::clear_error()`; a later successful event does not clear it. The diagnostic includes
the deepest failing component path/key, Rhai source position and stack, typed
execution timing and operation budget, a bounded termination token, and only
that component's state snapshot. Sensitive fields are redacted before the
snapshot leaves the runtime. Ordinary `Display`/error-banner text remains the
human message and never appends the structured payload.

The default runtime-error banner overlays the last committed view without
moving it or invalidating an active gesture. It uses compact monospace text
and the same native drag-selection/`Cmd-C` path as `text(...).selectable(true)`.
A Host that owns a better error surface may mount with
`ScriptViewConfig::new(id).show_error_banner(false)`; standalone applications
use `ScriptApplication::show_error_banner(false)`. Disabling the banner never
clears `last_error`, so the Host remains responsible for making the failure
visible. `ScriptViewConfig::on_error` calls the Host once for each new failure
(no polling), and `mirror_errors_to_stderr(true)` prints it to stderr, which
`ScriptApplication` does by default.

Use the development inspector (`Command + Option + I` or `F12`) for source
locations, state, semantics, traces, timings, and last errors. Event and action
traces intentionally retain only their semantic name and scope, never the
payload value; inspect application state or add a trusted Host diagnostic when
payload-level debugging is genuinely required.

Production workflow:

```text
gpui-rhai check
cargo build --release
```

`gpui-rhai init` writes a `build.rs` that embeds `ui/` through
`gpui-rhai-build`, and the Host mounts `gpui_rhai::script_view!()`: a debug
build reads the files, a release build the embedded copy, which Cargo
rebuilds whenever `ui/` changes.

See [Hot reload and production](docs/hot-reload-and-production.md) and
[Development inspector](docs/devtools.md).

## 13. Security boundary

Application-owned Rhai UI has the same trust level as that application's source,
but it is not a general system shell.

By default scripts cannot access:

- arbitrary files or module paths;
- URLs, sockets, subprocesses, or environment variables;
- Rust credentials or arbitrary Rust objects;
- GPUI App/Window/Context handles.

Imports use a restricted declared-source resolver. Props, events, stores,
capabilities, assets, Canvas commands, nodes, overlays, timers, signals, and
virtual collections all have schemas or Host budgets.

Only register capabilities and native handlers the application needs. Validate
authorization and opaque-handle ownership again at the service boundary. A
custom primitive is trusted Rust and belongs to the Host boundary, not the
script sandbox.

New application components do not expand that authority. Navigation and
ColumnBrowser emit proposals rather than route/load data; Composer does not
send requests or cancel Host tasks itself; temporal controls do not refresh
queries. Markdown never executes HTML, follows links or fetches images. Native
text registration publishes immutable Host data, not filesystem access. The
Gallery's streaming and loading examples use offline fixtures, not production
services. Generic `ctx.copy_text` is a bounded post-commit clipboard command for
user input or explicit automation, not a render, timer, effect or background
clipboard channel.

See [Security boundary](docs/security-boundary.md) and
[Capabilities](docs/capabilities.md).

## 14. Testing checklist

Run the portable checks before shipping:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo test --manifest-path tests/native-keyboard/Cargo.toml
gpui-rhai check
```

The hosted workflow runs these checks on a standard Linux runner. A private
repository receives a limited monthly allowance; removing macOS runners lowers
the multiplier but does not make hosted compute unconditionally free. If the
allowance is exhausted or billing is blocked, jobs stop before their first
step. A public repository or a configured self-hosted runner is required for a
permanently no-GitHub-charge runner path.

On macOS, also build release targets and run `scripts/release-smoke.sh` with the
Metal Toolchain installed. Screenshot, candidate-window IME, native
accessibility, and real input certification require an unlocked interactive
Mac and are intentionally not claimed by portable CI.

Also exercise:

- keyboard forward/reverse focus and visible focus rings;
- Enter/Space activation and disabled suppression;
- Input/IME, Unicode, clipboard, selection, undo/redo, and read-only behavior;
- overlay open/close, outside click, Escape, modal trap, and focus restoration;
- RTL logical layout and navigation;
- light/dark theme contrast and every component state;
- reduced motion;
- retained-page round trips for native scroll, selection/undo and focus, plus
  application-visible versus active/task policy;
- paused tail following under append/prepend/reorder/remeasure, scrollbar
  cancellation and residual wheel chaining;
- distinct formatted Table ranks and display values on Array and native data;
- composer IME confirmation versus submit, independent card actions, Markdown
  cross-block/plain/source/code copy, and stale native document deliveries;
- wall-clock/timezone determinism, DST gaps/folds, absolute Apply/cancel and
  relative-range resolution at query time;
- release binaries, not only debug builds.

Visual baselines supplement interaction tests; they do not replace them.
`gpui-rhai shot` takes an application's screenshots offscreen on macOS, after
steps such as `--theme`, `--hover`, `--click`, `--key` and `--input`; see
[application shots](docs/visual-testing.md#shots-of-an-application).

## 15. Agent playbook

When an agent modifies a gpui-rhai application:

1. Read the application's intent, this guide, and the relevant component/source
   files before proposing a design.
2. Inspect the pinned gpui-rhai and Rhai versions. Do not rely on remembered
   Rhai APIs when crate source or a small execution probe can decide.
3. Use named, non-capturing callbacks and `UiValue`-convertible curry data;
   never assume an arbitrary closure can survive the render that created it.
4. Keep product composition and business state in Rhai unless the Host boundary
   provides a concrete reason not to.
5. Keep GPUI/platform mechanisms, privileged services, and sustained hot paths
   in Rust.
6. Prefer existing public Box/Text/Style/Overlay/Layer/Canvas primitives before
   inventing a specialized native constructor.
7. Use official source components; do not add `gpui-component` as a hidden
   dependency.
8. Preserve stable keys, controlled props, semantic theme roles, locale/RTL,
   keyboard paths, and accessibility labels.
9. Register NativeHandlerRef descriptors before compiling scripts and declare
   exact event schemas. Never invent an adapter Rhai script merely because the
   Rust path was not inspected.
10. Distinguish tracked `element_bounds(ref)` from event-only untracked
    `event_target_bounds()`; do not create a resize/store feedback channel for
    one click-time geometry read.
11. Evaluate representative script branches. Rhai compilation alone does not
   prove dynamic function overloads exist.
12. Verify changes with logic tests, native interaction tests, release smoke,
    and real visual inspection in proportion to risk.

## 16. Troubleshooting

### `function not found`

Rhai resolves many overloads dynamically. Read the complete missing signature,
then check the registered feature/package surface and the argument's exact
runtime type. Compilation alone may succeed.

### callback works in the entry module but not an imported component

Do not call a stored FnPtr by name against a fresh unrelated scope. Let formal
component props and gpui-rhai retain the callback's module invocation context.
Also replace anonymous/capturing closures with named functions and durable curry
data.

### NativeHandlerRef fails to resolve

Register a namespaced `snake_case` handler before script compilation. The ID has
one namespace separator, for example `host.refresh`, and the descriptor must
declare the actual event name (`click`, `change`, and so on).

### UI state resets unexpectedly

Check stable component keys and state schema compatibility. Changing a key means
mounting a different component instance.

### theme edits do not appear

Validate the `.rhai` theme and `styles.rhai`, confirm family/variant selection,
and check that components use semantic tokens and `ctx.component_style` rather
than literals or the removed global helper. Theme Studio shows the active draft
and contrast diagnostics; `gpui-rhai check` reports invalid component/part
rules.

### overlay is clipped or dismisses the wrong view

Mount every related view under the same `ScriptViewHost::container`, use unique
local overlay IDs, and declare parent overlay IDs for nested ownership.

### click-time bounds are missing or stale

For a node event, read `ctx.event_target_bounds()` or `NativeEvent::target`
inside that callback. Do not cache `ctx.element_bounds(ref)` in state on every
resize merely to service a later click. Custom primitive emissions currently
have no renderer-owned event target. A primitive may accept a validated
`ElementRef` prop and use `PrimitiveContext::element_bounds` for
last-committed layout geometry; event consumers still receive only the
primitive's declared payload.

### application exits immediately in release

Run the release binary with logs, confirm every embedded component/theme/locale
source is included, and use the repository release smoke. Debug-only inspector
registration and source paths must not leak into release artifacts.

## Further reading

The documentation index is [docs/README.md](docs/README.md). The
[module reference](docs/reference/README.md) lists the props, events, slots and
parts of every official module; [Rhai for gpui-rhai](docs/rhai.md) covers the
language as views use it. Start with the architecture and embedding guides,
then follow the component/theme/performance links relevant to the application
you are building.
