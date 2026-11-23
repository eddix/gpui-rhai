# Modern / Clean visual specification

This is the maintained implementation specification, not a research archive.
The migration is not complete until the functional and native visual gates in
[the work log](migration-modern-clean.md) pass.

## Presets

| Design | Light variant | Dark variant |
|---|---|---|
| modern | Modern / Light | Modern / Dark |
| clean | Clean / Light | Clean / Dark |

A design owns geometry, typography, surfaces, edges, effects and motion.
An appearance changes colors without changing geometry. Alternate palettes
inherit Modern geometry and adapt semantic colors; they are not exact-color
reference configurations.

Full variants are ordinary theme sources. They resolve the token base, preset,
and Host overrides atomically. There is no separate native-only skin selector.
Application component overrides and instance overrides remain available.

## Core metrics

The reference dimensions below use comfortable density at reading scale 100.
They are logical pixels. Do not multiply them
by the display's pixel ratio or round fractional sizes to a 4px grid.

| Part | modern sm / md / lg | clean sm / md / lg |
|---|---|---|
| Button height | 32 / 40 / 48 | 24 / 28 / 32 |
| Button horizontal inset | 12 / 16 / 20 | 8 / 10 / 12 |
| Button icon gap | 6 / 8 / 10 | 4 / 6 / 6 |
| Button icon box | 16 / 18 / 20 | 14 / 16 / 16 |
| Button radius | 10 / 12 / 14 | 6 / 6 / 6 |
| IconButton side, sm / md | 32 / 40 | 24 / 28 |
| IconButton glyph box, sm / md | 16 / 18 | 14 / 15 |
| Field height | 32 / 36 / 40 | 28 / 32 / 36 |
| Field base inset | 10 / 12 / 14 | 8 / 10 / 12 |
| Field radius | 9 / 10 / 12 | 6 / 7 / 8 |

Clean Button moves an icon-side inset inward by 2px. IconButton is always a
square with no text-derived intrinsic width. A field's trailing action has
its own inset; do not blindly mirror its leading padding.

| Element | modern | clean |
|---|---|---|
| Checkbox | 16 square, radius 5 | 16 square, radius 4.5 |
| Switch md / sm | 32×20 / 26×16 | 32×20 / 26×16 |
| Switch thumb | track height − 4, circular | same geometry |
| Spinner default diameter | 14 | 14 |
| Spinner ring / arc | 16-unit view box, center8/radius6, stroke2 | same center/radius, stroke1.8 |
| Spinner track alpha / turn | 20% / 750ms | 18% / 800ms |
| Badge sm / md | height 20 / 22, pill | height 18 / 20, radius 4 / 5 |
| Badge label sm / md | 11/16 / 12/16, weight 500 | same type metrics |
| Derived Tag | height24, radius7, type13/20 | height24, radius5, type12.5/20 |
| Kbd sm / md | height18/22, radius5/6, inset5/6 | height16/20, radius4/5, inset4/5 |
| Card | tray r20/pad4; surface r16/pad16 | single surface r10/pad16 |
| Popover | radius16/pad16 | radius12/pad14 |
| Tooltip | radius8, padding5×9 | radius6, padding4×8 |
| Pagination md target | height32, radius10, inset8 | height28, radius6, inset7 |
| Pagination number gap | 4 | 2 |
| Textarea | radius10, padding8×12, line21 | radius8, padding7×10, line20 |
| Table body / header | 44 / 44, cell inset14 | 40 / 36, cell inset12 |
| Command search / row / footer | 52 / 40 / 38 | 48 / 36 / 40 |

The derived `xs` recipes are explicit smaller sizes, not a fourth reference
size. Buttons are 28px modern / 20px clean; fields are 28px / 24px. Preserve
parent min/max/fill contracts, and grow text and control metrics coherently
for reading scale.

Compact is a separately derived layout preference, not a fifth appearance or
an alias for a smaller size/design. It reduces control/row minima and spacing
without changing typography, icon boxes, choice marks or paint strokes. Main
md controls lose 4px height, subject to their text-fit floor. Table body/header
lose 4px; arrangement spacing uses `space.spacing_unit` (4px comfortable,
3px compact) while geometric `space.unit` remains 4px. Track padding/radii and
total heights remain concentric. The four comfortable reference recipes stay
unchanged.

Preset padding/gaps resolve through `space.spacing_unit`; non-hairline
dimensions and radii use `space.unit` or their named metric. This preserves
the dimensions above at 100% and scales them with typography. Structural
borders and paint-effect offsets remain independent; application-authored
literal pixels remain literal. Slider track thickness belongs to the cross
axis in both orientations, never to a hard-coded horizontal height.
Table's named grid-width references use `without_reading_scale()` so their
Host overrides and theme changes remain live without multiplying hairlines
when only the reading preference changes.

Buttons and IconButtons use a 2px exterior focus ring with a 2px transparent
gap, gated by the host's focus-indicator policy, without changing layout or
hit geometry. This shared SDK decoration is not a filled box-shadow spacer.

## Typography

Inter 400/500/600 is the UI family. The bundled native faces retain the
reference advances and metrics; their OFL-1.1 license is distributed with
them. Explicit CJK/Arabic and missing-glyph fallback is required. Code keeps
its own monospace stack.

| Role | modern | clean |
|---|---|---|
| Button sm / md / lg | 13 / 14 / 15, weight500 | 12 / 13 / 13, weight500 |
| Button line sm / md / lg | 13 / 14 / 15 | 16 / 20 / 20 |
| Field value sm / md / lg | 13 / 13.5 / 14 | 13 / 13 / 14 |
| Label | 13/20, weight500 | same |
| Choice label / first line | 13.5/20, weight400 | 13/20, weight400 |
| Description | 12.5/17 | 12/16 |
| Card title | 14/20, weight500 | 13/20, weight500 |
| Card body | 13/20 | same |
| Dialog title | 15/22, weight500 | same |
| Pagination | 13.5/20, normal400/current500 | 13/20, normal400/current500 |
| Tooltip body / key legend | 12.5/18 / 11/16, weight500 | 12/16 / 12/16, weight500 |
| Kbd sm / md | 11/11 / 12/12, weight500 | 10.5/10.5 / 12/12, weight500 |
| Table body / header | 14/20 weight400 / 14/20 weight450 | 13/20 weight400 / 12/16 weight500 |
| Command query / row / footer | 15/20 / 13.5/20 / 12.5/18 | 13/20 / 13/20 / 12/16 |

Modern uses −.01em tracking in the control/title recipes that specify it;
Clean uses zero tracking there. Tracking must affect measured advances and
paint together. Padding is not a substitute. Do not force uppercase or mono
on structural labels, flatten all weights to 400/600, or impose a 12px floor
on all roles.

## Paint and color

- Preserve sRGB values, alpha and compositing order. Semantic foreground and
  soft background are pairs, not one color used everywhere.
- Canvas solid paint may use public `current_color()`/`PaintColor`. It resolves
  the actual owner/inherited native foreground during paint, not a theme
  snapshot from script render. Button/IconButton loading paths use this same
  context so Host root and spinner-part foreground overrides remain effective.
  Style fills, border ink, shadows and layered gradient stops resolve through
  the same paint expression using the winning native state. This does not
  imply support for full gradient path/typography effects.
- Distinguish strongest/body/secondary/subtle/placeholder/icon roles. Primary
  actions use ink/reverse ink, not the information-blue accent.
- Pagination groups page size and the visible item range before its page
  controls. Range/total use body ink and the connector uses its muted role.
  Its outer wrapped layout uses independent horizontal 16 / vertical 8 spacing,
  with 16 inside the info group and 8 between the size label and field. These
  are reading/spacing-token-scaled design values, not one uniform gap.
- Modern primary actions have a gradient fill and gradient edge. A single
  flat fill or two-stop approximation is not an accepted replacement for a
  multi-stop edge.
- For these sRGB recipes, preserve the declared gradient line in rectangular
  boxes and interpolate transparent stops in premultiplied color, following
  [the gradient model](https://www.w3.org/TR/css-images-3/#linear-gradients).
  Each layer has an independent gradient-origin box and rounded clip: a
  padding-box fill does not reset the border-box edge's color progression.
  Matching another use of the same native two-stop shader alone does not
  establish these contracts. Gradient-plane motion is also not glyph masking
  or complete loading-label acceptance.
- Clean's subtle outlines are fractional shadow strokes, not layout borders.
  Fields use shadow edges without reserving a thick internal border.
- Shadow lists are topmost-first, support inset and signed spread, and never
  consume layout space. Hover/focus/selected have separate recipes.
- Keep component-local values when they differ. Card, field, button and
  overlay surfaces must not be collapsed into one universal raised fill.
- Table has its own card surface: white / `#232326` in Modern, white /
  `#1c1c1e` in Clean. Modern's card radius is16 with a layout edge; Clean's
  radius is10 with a shadow hairline. The ordinary header is on that card,
  not a generic hover wash. Modern header ink is `#8f8f8f`; Clean header ink
  is black61% / white62%. Modern has 1px row/column grid lines; Clean has
  0.5px horizontal lines without a vertical grid. Its selected-row surface is
  `#f5f8ff` / blue `#4fb0fe` at12% in Modern, and blue `#0060e2` at5% /
  `#4d8dff` at14% in Clean. Generic collection selection is a separate design
  policy even when it shares selection mechanics. Actual array/native rows,
  header/body sizing, inset sort actions, resize handles, controlled commits,
  RTL, density and reading scale pass the current native Table suite on both
  backends. This is not full pixel acceptance: native450-weight matching and
  the complete visual/state matrix remain open.
- Tooltip is inverse: Modern light uses `#171717` with white ink; dark uses
  `#fafafa` with `#171717` ink. Clean light uses `#18181b` at92% with
  `#fcfcfc` ink; dark uses `#fafafa` at94% with `#111113` ink. Preserve
  those alpha values rather than preblending against an assumed parent.
  Key legends use one authored color alpha, not an extra subtree opacity.
  Modern has a small key chip; Clean uses a bare legend. Clean's required
  8px backdrop blur is still a backend integration gate, not an accepted
  opaque approximation.
- The 19 alternate palettes use opaque paired tonal status surfaces, calibrated
  against their foreground. A translucent wash that passes on plain paper
  may fail on a selected row or colored card. This alternate-palette policy
  does not recolor the four exact presets or waive their contrast findings.
- Exterior focus uses `#0060e2` at 70% in light appearances, `#4d8dff` at 80%
  in dark appearances, with a 2px stroke and a 2px transparent gap. Checkbox
  and Switch decorate the box/track, not the whole label row. Their first
  line is 20px with a 10px text gap; disabled content has 50% opacity.
  Checked dark Switch thumbs drop their shadow, while unchecked thumbs keep
  the design-specific stack. Checked/unchecked hover washes remain separate.
- A source-matched low-contrast placeholder is not certified accessible merely
  because the source matches. The accepted default for the four exact presets
  is to preserve authored colors and report measured risks, not silently recolor
  them or advertise a WCAG pass. Enhanced contrast must be an explicit choice.
  Keep implementation mistakes (wrong colors, duplicate alpha, lost scopes)
  distinct from verified authored-color risks. Alternate palettes retain their
  own strict readability contract; this decision does not waive arbitrary
  low-contrast regressions in components or Host overrides.

## Interaction and topology

Switch settings rows accept a secondary `description`, logical `placement`
(`start`/`end`) and explicit `sm`/`md` size. End rows use the available width
and a16px gap; start rows keep10px. Both mirror in RTL. The description uses
12.5/17 Modern or12/16 Clean type and the original soft `choice.description`
ink, with its contrast risk reported. The track stays on the first20px line,
not vertically centered against both text lines. Description updates and
placement changes retain the keyed control, text and accessible switch owner.
Clicking either text line proposes the same controlled change. Instance root
and part overrides remain above both common and end-state preset recipes.

Checkbox's 1.8px round-cap/round-join mark draws by stroke length, not alpha:
checked uses 260ms after a 40ms delay, mixed uses 200ms without delay, both at
`cubic_bezier(0.22,1,0.36,1)`. The `checkbox` namespace's `stroke_width`,
`draw_ms`, `draw_delay_ms` and `mixed_draw_ms` tokens configure that shared Canvas path.
Initial mount is settled; only an accepted controlled state change starts a
draw. Unchecking clears the mark immediately. None/Reduced settle statically
and native frame sampling executes no Rhai. This mark implementation does not
close the separate box press-scale/color-transition or Switch motion gates.

Keep hover, pressed, focus, active navigation, selection, invalid, read-only,
disabled and loading distinct. Loading blocks duplicate actions but does not
receive the disabled-opacity recipe. Focus follows the native owner.

Component default styles resolve below controlled state refinements, then
application and instance styles. State changes must not accidentally erase
Host overrides. A design switch may reflow, but must not remount a keyed
editor, reset undo/selection, change callback ownership or cancel unrelated
work. Keep presentation topology stable when only surface nesting changes.

Names that address one element share one part-array cascade, not nested
resolutions: Input/TimeInput root+input, Progress root+track, Spinner root+indicator,
CommandDialog root+dialog, Toast common+variant, and AppShell drawer+side region.
The temporal editor delegates Input's native normal/error/focus/disabled surface;
it must not repaint it with a second border/shadow recipe. Input's native
20-character intrinsic preference remains distinct from fixed, fill and
min/max constraints; an unbounded row must not collapse it to padding alone.

Pagination's current number uses its selection surface and strongest ink,
not a primary-action or generic ghost-button foreground. Reserve each page's
width using the longest page number at both normal/current weights, measured
by native text layout. Previous/next are square icon-only targets with localized
accessible names. Controlled page proposals never update the selection until
the caller accepts them. The page-size field and visible-item range form one
info group before the controls; horizontal and wrapped-row gaps resolve
independently through the public style API. Number windows reserve a stable
bounded slot count at their ends as well as in the middle. Explicit compact
mode omits numbers/ellipses, without changing the density preference. Disabled
controls dim once in their own groups while range text stays undimmed. Key
targets by business page and keep the current target focusable, so native
identity preserves keyboard focus without deferred imperative focus requests.
Automatic width-based number collapse uses the public intrinsic overflow row.
Numeric slots and ellipses share one optional cohort; Previous/Next are required
and keep their native identity. Allocation growth restores the numbers without
a script resize loop. The same hierarchy and measured geometry apply in RTL,
both densities and reading scales. Continuous color/composition acceptance is
still open; this is not a declaration that the entire Pagination reference has
passed acceptance.

Button content/width morph is 480ms, icon entry 520ms; common color changes
are 140–180ms. Button/IconButton activity rotates in 900ms using their own
24-unit view box, center12/radius8.5/stroke2 and 20% track. Standalone Spinner
uses the 750/800ms design tokens above. Both share one retained Canvas outline
construction, not duplicated painters. Solid paint inherits the actual native
foreground; explicit root/indicator overrides keep their normal precedence.
`metrics.spinner` controls its late-bound diameter (xs12, sm/md14, lg18);
`spinner.stroke_width`, `spinner.track_alpha` and `spinner.speed_ms` remain
ordinary theme settings. The standalone component does not hard-code an accent
or muted color. Native motion owns frame sampling;
None/Reduced preferences and lifecycle cancellation still apply.
Looping indicators settle at their visible midpoint under None/Reduced and
stop requesting frames; the SDK's stricter policy intentionally does not keep
them slowly rotating. An unrelated commit retains phase; suspension excludes
wall time from the active clock.
These are target contracts. The current position-only `layout_motion` is not
an implementation of the intrinsic-width morph, and must not be attached to
Button as a substitute. Width/content entry, tracking and the remaining paint
gates are not yet accepted.

Derived strong Badge paint owns `badge.<tone>_strong_bg` and `_strong_fg`.
Its fill is opaque and its label is calibrated against that fill; ordinary
subtle Badge colors and alpha remain the exact authored pair. Derived status
actions and collection metadata use named, theme-prepared ink roles rather
than performing per-row contrast searches or borrowing a solid-action inverse.
`collection.metadata` is an explicit derived opaque role, not a replacement for
the original subtle/placeholder roles in source-backed fields.

Textarea's ordinary counter uses the field placeholder role. A full but valid
value uses `field.counter_full` and the `limit` part; it is not an error.
Controlled values beyond `max_length` remain rejected, so the source's
over-limit presentation is not advertised as a supported input state.
Counter/limit are one part-array cascade, preserving a Host counter override
at the maximum. Structural row scopes keep their declaring component's preset;
formal incremental renders refresh preset and application rules without
replacing instance props or callback/state identity.

Changed-label alpha now uses the shared `motion_on_change` trigger: 480ms at
`cubic_bezier(0.25,1,0.5,1)`, with an idle first mount and no restart on parent
reflow. A stable structural parent applies entry alpha without replacing the
label part's loading/Host opacity. This closes only the label-alpha portion;
relative text entry translation, blur, icon entry and actual width morph remain
separate, unaccepted requirements.

## Derived widgets

Use the same row/type/surface recipes for Tree, List and ColumnBrowser; field
geometry for compatible date/time controls; ordinary soft surfaces for panels
and document viewers. Keep code/document content semantics and chart numerical
meaning unchanged.

Radio derives its 16px circular mark and 6px dot from the common choice metric,
uses the shared checked/unchecked surface and inverse-ink pair, and aligns to
the first 20px line of a wrapped label. Only RadioGroup's enabled keyboard
cursor borrows the group's exterior focus ring; inactive options do not.
The mark, dot, label and first-line wrapper have separate public parts.

Tag derives the removable-chip geometry rather than retaining the old pill/tape
signature. Its optional facets keep semantic fill and foreground paired; the
value stays normal ink. The close target is 18px with a 10px glyph and an inset
keyboard cue, with no layout change on focus. Disabled chips dim once to 55%
and block close callbacks. These are derived contracts, not new TagInput editing
or tokenization functionality.

Kbd draws a whole combination in one chip, keeping key order left-to-right
inside RTL layouts. Mac key gaps are 1px, other-platform gaps 3px; single
symbolic keys use 110% type. Modern combinations sit 0.5px above mathematical
center, Clean combinations have no such offset. These metrics follow reading
scale, not compact density. Literal `text` is not binding syntax; named `keys`
share the SDK's platform/display/spoken-name vocabulary. Empty content draws
no chip. The inline appearance remains bare metadata, not a nested keycap.
The contextual `menu` appearance is independently20×20 minimum, radius5,
horizontal inset5, type11/16 weight500, before reading scale; standalone size
does not change it. It inherits menu/command ink, uses a Modern inset edge
plus bottom depth or Clean fractional inset edge, and has no enlarged-symbol
or optical-y adjustment. Command consumes the SDK's parsed chord/key
descriptors to draw individual caps. Literal legends remain one unparsed chip.
Its normal/active ink and Modern85%/100% opacity remain explicit; preserving
them does not establish native isolated-group opacity or an accessibility pass.
Modern has its own surface, ink and inset depth edges. Clean restores inherited
foreground color and uses it for the chip fill and the inset edge: 7% and 6%
respectively in light mode, 10% and 8% in dark mode. This does not imply
complete acceptance of text effects or a blanket contrast waiver.

The authored Modern Table header retains weight450. The current pinned Apple
font matcher can select400 when the family supplies400/500/600, unlike the
[continuous CSS weight-search order](https://www.w3.org/TR/css-fonts-4/#font-style-matching).
Do not silently replace the authored weight with500 or call the current pixels
exact. A local matcher candidate is separate from published-backend integration.

Panel focus is one non-layout inset decoration, not a negative-offset child
that assumes a 1px border. Region applies its common inset only once: `bleed`
and `inset:false` must remain effective under every preset. SplitPane's flush
capture zone stays transparent; its separate grip decoration owns feedback.

Command uses a stable outer/card/search/results/footer topology in both designs.
Modern has a4px concentric tray with radius20, an inner radius16 white/#19191b
card and its footer on the tray. Clean has one radius12 white/#1c1c1e panel with
fractional shadow edges and an internal footer separator. Embedded mode clears
the root/card chrome, not the internal search/list spacing. Caller and instance
overrides are above those clearing defaults; footer nodes are wrapped rather
than reskinned as footer containers.

Search spacing, the native editor, placeholder, glyphs and Enter hint have
independent public parts. Its heading is52/48 high; the20px editor uses15/13
type and no second field edge. The list has horizontal6/4, top4/0 and bottom8/4
padding outside the bounded, measured viewport. Rows use inset10/8 and radius10/6.
Headings have independent12/14.5 and10/16 type, with top10/bottom4 spacing;
Clean paints uppercase while retaining the original semantic group name.
Short results fit actual row heights, including headings and wrapped labels;
large results remain virtualized under the declared cap.
Selection uses command-local#f0f0f1/#26262a or black6%/white8%, not a generic
collection tint. Rejected roving proposals do not acquire a second hover fill.
Command rows reserve a26/20 leading slot even without a glyph. Plain icons use
17/16 boxes; logo tiles have radius7/5 and14/12 inner glyphs with their own
surface/edge/ink roles. Their hint follows the label on one baseline with8px
spacing. String assets and retained Array nodes share that presentation; logo
takes priority and neither decoration adds a semantic control. Original hint
ink remains deliberately soft, with its contrast risk still reported.
At the default480px committed inner-card boundary, rows grow to44/40,
the Enter badge, keycaps and visual hints leave layout, and Clean's default
footer is omitted. Modern keeps navigation; custom footer nodes keep their
own visibility and key scope. This uses a scoped discrete geometry observation,
not the window viewport or a per-pixel Rhai resize handler. It is a
last-committed, controller-driven transition with an initial wide fallback,
not an in-measure CSS query. Input and row identities are retained.
Logical image IDs fall back to the label's first Unicode letter/number, uppercased,
using11/10 type at weight600 in the existing command ink. Native pending/error/ready
states share the image's content lease and do not rerun Rhai. Caller-composed nodes
set `image_fallback_text` and its `image_fallback` part themselves; the slot does not
take over arbitrary descendant lifecycles. Avatar's image failures use its ordinary
initials. A fallback neither changes the accessible owner nor suppresses diagnostics.
Source state transitions still need completion; current panel/keyboard acceptance
does not close that gate. The default SDK footer
advertises its actual arrow/Enter behavior, not unsupported section shortcuts.

DateRangePicker uses a continuous background band under the shared Calendar's
day cells. The first and last day have independent chips. Modern has blue
primary and neutral comparison bands on a 4px outer tray; Clean has a neutral
primary and blue comparison band on one hairline panel. A comparison that
overlaps the primary range retains a separate visible strip. Internal month,
week and day geometry is shared by painting, pointer selection and keyboard
navigation. Compare and Apply do not add a second selection owner.

At a measured width of 600px or less, the panel shows one centered month,
horizontal scrolling presets and a stacked footer. The grid can shrink;
the footer remains reachable in a short viewport through a real scroll region.
Presets and date cells edit only the draft. The trigger shows the accepted
range and optional comparison, not a speculative value while editing.
Retain the authored light-preset contrast warnings rather than changing the
chip or weekday inks to satisfy an audit silently.

TagInput keeps one native editor in its wrapping field. The field, error well,
focus-within edge and chips are independent parts: Modern uses a 36px base well
with a 168px cap and soft invalid fill; Clean uses 32/156px and a thinner
outline. Chips have a 24px row, optional 18px avatar, and a separate 18px
remove action. Their minimum padding is logical, including in a right-to-left
field. A batch that overfills the visible well scrolls the existing editor
back into view instead of moving the keyboard focus or replacing the input.

The dim outline, hint, count and icon tokens keep the original color hierarchy
in the four presets. Keep low-contrast findings visible; do not treat softness
as a substitute for meeting a contrast threshold. Paint transitions, enter
and duplicate feedback use the runtime's normal motion preference. They do
not delay or serialize tag list proposals.

FilterMenu presents one keyboard route through group headers and open options.
The design keeps the title, active chips, group label, total, and selected
count separate. Modern has a 6px tray inset, a 36px group header, and wrapping
30px chips. Clean uses one hairline card, a 32px group header, and 24px chips
on one measured line. Clean's `+N` reserves actual native text width in the
shared overflow row, with unpresented chip buttons removed from the keyboard
route. It does not count stale measurements or measure cloned controls.

The shared disclosure height observes size rather than position. Options keep
a bounded inner scroll and close off input and accessibility while folded.
The authoring palette remains intact: faint totals are still audit evidence,
and any accepted stock contrast risk is tied to its component role and colors.

StepperForm uses one presentation of each keyed pane, keeping inactive
editors out of layout, focus and presented accessibility while retaining their
identity. Modern separates the progress tray from the raised form card. Clean
has a single hairline surface with a progress rail. The compact progress label
takes over at a measured width of 420px or less. Progress numbers use authored
faint ink and continue to produce independent contrast findings.

Viewport height and footer folding share the disclosure renderer; without
geometry motion they take the current content's natural height, not a stale
measurement. Directional entrance, validation shake and check writing are
presentation only. Step changes and submit/reset are host decisions. The active
pane alone owns native focus transfer; disabled fields, hidden pages and
descendant overlay panels are not alternative focus targets.

ProgressTracker and SetupGuide share keyed completion, the progress meter,
and check and strike feedback. Modern uses a thicker neutral meter and a
raised tray; Clean uses a slim accent rail and a hairline card. A narrow
Clean tracker hides its decorative trailing icons, without hiding labels or
controls. Clean places the SetupGuide count alongside the rail; Modern keeps
the rail bare. Completion proposals do not change the meter until accepted.
Collapsing a guide or a group removes its tasks from focus and accessibility.

Conversation controls keep application configuration separate from native
input and transient motion. Modern ReactionBar uses a single floating pill
and distinct active inks; Clean uses individual hairline chips with blue
selected treatment. Both keep one keyed action and an exact accessible count
while compact digits and glyphs animate. MessageActions uses a 30px Modern
or 28px Clean action row. The Modern feedback body is a raised card, while
the Clean body opens inline. Feedback reason fields are application payloads,
not audit labels. ModelPicker places its active candidate and committed model
in distinct states; the menu uses the existing overlay surface, fitted
viewport and keyboard path. On narrow panels, ratings stay in accessible
descriptions even when their visual bars disappear.

CitedAnswer and AIAnswer use native Markdown for the reading surface.
An inline tag is still a native link span; the annotation wash uses the same
UTF-8 geometry as the text, without inserting extra controls into a paragraph.
The filled overhead tray and rounded card belong to standalone Modern;
Clean keeps citations compact with inset hairlines, and the embedded reply
does not add another surrounding card. A focused source chip can mark
statements and its excerpt. Drawing those annotations does not rebuild the
prepared document or alter selection. The reply's stop and completion states
track the application, not an animation clock.

ThinkingLoader measures the constrained width of its one visible label,
then hands the transition to MotionRuntime. The eight independently faded
rays, sweeping text highlight and completion stroke are ornamental: the
stable base label is the only status exposed to accessibility. A broad
Modern capsule and a compact Clean chip share the same geometry topology.
ThinkingState keeps the header independent of its disclosure body; timeline
markers, rails, source chips and elapsed captions use the respective
presets. A collapsed body cannot focus or activate its hidden steps.

RunControls pairs a fixed-height, native following transcript with an
independent native editor. Modern has separate surfaces on a tonal tray;
Clean uses a single hairline frame and inset composer. Prompt bubbles
mirror their trailing corner, steering notes attach to accepted answer
positions, and queued rows have their own bounded viewport. Mode and
permission requests never change selected presentation until accepted.
The menu uses the native dismissal/focus boundary, and alternate submit
uses the same native composition guard as ordinary submit.

PromptLibrary and IntegrationPicker share catalog matching, key movement
and the ListDetail pane geometry. Columns stack at the component's measured
breakpoint, without duplicating the selected form. Modern uses a card on a
tonal tray; Clean uses finer separators and smaller controls. Prompt
variables remain native inline markers in the preview and have a separate
retained editor. Integration logos, connection art and detail transitions
are decorative; a proposal cannot change the committed switch or group
selection on its own.

IntegrationSettings keeps the edit transaction separate from credential
actions. Modern uses a tonal tray with an inset raised form; Clean uses one
hairline frame and denser fields. Both reuse the native text editor,
searchable Select, Switch, Button loading state and measured disclosure.
Pausing removes the form from interaction and preserves the local draft.
Status and an explicit conflict action distinguish a pending edit from
what the Host has accepted.

CommentThread and CommentsPanel reuse a bounded, native virtual collection
for wrapping rows, the same metadata lane and the same controlled editor.
Modern uses an inset card and circular send control; Clean uses a single
fine-edged surface, tighter text, an indented reading lane and a small
rounded send tile. The thread can follow the tail without moving a user
who is reading earlier messages. Resolve, run and undo remain accepted
application states, independent of navigation or filter state.

WritingAssistant keeps the diffuse pink, violet and blue glow behind the
editor and conversation. The three Gaussian shapes are cached as transparent
artwork and move with the existing motion policy; glass chips and the Modern
answer surface use live backdrop blur. Clean lowers the glow and uses a smaller heading,
flat answer text and compact mic/send buttons. Dictation heat is shown by a
separate interim preview and a mic activity mark, without replacing the
controlled editor or treating preview text as a message.

ApprovalCard and PermissionRequest share the review heading, risk treatments
and compact receipt. The footer and context lanes keep distinct geometry
from the pending decision controls. The Modern permission duration selector
uses raised chips on a track; Clean uses compact chips on its surface. Both
retain the same native radio group and focus/keyboard semantics. Timed
receipts show a Host-controlled track, not a locally enforced permission.

DecisionCard keeps the task and pager on the head and the decision itself
on a stable inset surface in Modern. Clean uses a single compact surface.
Wireframe picks use three preview lanes at desktop widths and reflow to
trailing previews in narrow rows, with the same focused option identity.
DecisionInbox distinguishes a folded pending row, an active request, and
an accepted receipt. Only the last changes the pending count.

DecisionTree keeps the question, selectable branch and evidence in three
separate columns when space permits. Clean uses a tighter card radius and
a hairline around the evidence; Modern uses a larger raised surface.
At narrow widths the cards stack and the connector runs through their
start-side inset. Lines are decorative, measured from the placed cards,
and cannot steal pointer input. Selected paths reflect accepted Host
receipts, with the result shown beside or beneath its branch. A hover or
a keyboard-focused branch is only a preview.

ReviewCenter separates the prioritized queue, the evidence and the action
strip. In wide layouts the evidence has its own scroll viewport and the
action strip stays fixed below it. At narrow widths the bounded queue moves
above the details, which expand in the surrounding page's scroll surface.
Modern uses an inset raised active row and padded
frame; Clean uses a tighter radius and hairline dividers. Risk, wait time,
focus and the selected row are separate cues. File evidence uses the
native document surface, and undo is offered only for a live Host receipt.

DelegateBoard pairs compact draggable cards with agent queues and an
accepted workload summary. Modern uses a raised frame and an inset pile;
Clean keeps the dividing rules light and the agent cards tighter. In a
narrow layout the pile appears before the agents without changing the
task ID or drop namespace. Suggestions and keyboard destinations are
distinct from accepted membership; after the Host commits, the card's position
moves through shared-layout motion while its size adopts the destination's row.

AgentQueue separates queue pressure, waiting tiles, worker slots and outcome
bins. Modern presents them in a raised card; Clean uses a flatter tray and
hairline divisions. The compact layout stacks waiting work above workers.
The depth plot and spoken statistics come from the same Host clock: exact
step changes, a weighted mean, overlapping throttle windows, and the
configured threshold. Inspection and proposed commands are independent of
the accepted worker/job state.

AgentLanes uses one projected time axis for each activity lane and for the
handoff connectors. The header and agents stack in the compact layout;
short task hit areas do not distort the exact time marker. An overlap
gains another track. Modern uses an inset tray with softly tinted task
blocks; Clean uses a flat board with hairline divisions. State is also
given in the task's accessible description.

AgentHandoff anchors its delivery paths to the actual agent cards and
handoff history columns. The compact layout interleaves those columns
with the agents instead of shrinking their hit areas. Recent delivery
motion runs on the same viewport as the standing paths. Lost evidence has
its own state, reason and explicit restoration control; it is not hidden
or shown as delivered until the Host accepts an update.

AgentInbox uses a single keyed set of cards for a wide board, a narrow
stack or an explicitly selected list. The state pill, optional usage
counts and most recent steps use accepted task data. Expanded disclosure
and keyboard focus belong to the card, not to its current column
position; an accepted state change moves the same task identity.

AgentPlan separates draft ordering and text editing from the accepted
execution state. The execution rail comes from the centers of the actual
step markers. Completed links, active indicators, errors and retry
controls represent the Host's current snapshot rather than a simulated
animation driving business state.

AgentCheckpoints puts a chronological branch graph on a bounded scrolling
card. Its links connect the measured checkpoint markers, with a stronger
marker for the accepted head and a quieter rail for earlier attempts. A
narrow layout can hide secondary metrics from sight without removing them
from the accessibility description. Modern uses a nested tray; Clean uses
a hairline and less elevation. Navigation is local; starting a new attempt
always requires a Host decision.
Modern Light's original muted task and summary labels measure 4.16:1 on the
tray; the strict contrast audit continues to report those two labels.

AgentTrail presents accepted execution in a single compact card. A live
branch links the running step to its current substep, using the same
measured geometry as the standing links. The pulse uses the motion policy
and does not advance the Host clock. Long branches show the current
context first, with explicit controls for the omitted rows. A paused
approval opens inline without removing the step or its history. Modern
uses a nested tray and raised card; Clean uses the shell hairline and
lower elevation.
The original Modern Light progress count on the tray measures 4.16:1;
the strict audit still reports it.

AgentTree centers dispatcher cards over their subtrees when there is room
for every track. At narrow widths, the same retained cards enter a
single indented list. Links connect the presented bounds; the supervisor
is a separate peer linked with a dashed line. The summary retains costs
and wait/failure counts from the full tree, even across collapsed or
unselected branches. Review controls use the common connected Popover.
The original Modern Light agent count on the tray is 4.16:1. Both light
presets report 4.21:1 for waiting text on its tint and 3.91:1 for done
text on its tint. The soft-blue branch line is decorative; the hierarchy
is also present in the layout and the tree items' level and parent text.

AgentGraph distinguishes standing channels, recent traffic and accepted
replies. The lead sits at the center, other agents share an inner orbit,
and tools and people prefer an outer position near their standing links.
A dense graph can use the same native targets and routes in a grid. The
short bright request trail is solid; the return head is hollow. Pulses
use the graph's presented route and the existing activity clock instead
of moving or replacing input targets. Inspector, panel and legend derive
from the same Modern/Clean surfaces as neighboring cards.
Modern Light's original node and calls-per-minute counts on the tray
measure 4.16:1 and are reported by the strict audit.

AgentFleet keeps one focusable overview per lead and shows the open lead's
sub-agents beside the column or under that lead, according to the actual
viewport. The same keyed controls and measured connections support both
layouts. Status pills, live timers, branch costs and mini progress marks
summarize accepted data. The task inspector uses the connected Popover
and shares AgentTree's versioned resolution actions; a refusal keeps the
inspector and its evidence open.
Modern Light's muted fleet counts measure 4.16:1; waiting and done pills
measure 4.21:1 and 3.91:1 in both light presets. These original combinations
remain visible in the strict contrast audit.

AgentMonitor pairs an activity well with a short rolling transition, a
proportional time strip, and a scrolling recent-step list. Its compact
bar keeps the same disclosure focus owner and shows recently used tools.
The activity clock and line widths use accepted data; a decorative slide
does not change duration or execution state. Each strip segment targets
the corresponding native history row and its connected Popover.
The original Modern Light counter and quiet log label on the tray are
4.16:1 and remain in the strict contrast report.

AgentSteering places the run and ordered steps on a card with a thin,
measured rail. Queued notes occupy their own compact band; accepted
notes use the same shared motion identity alongside their destination
step. Textarea owns both submission chords and selection/IME. The
composer stays outside the run card and uses compact shortcut hints.
In Modern Light, the original muted shortcut descriptions on the composer
tray measure 4.16:1; the strict contrast audit continues to report them.

AgentRecap pairs a quiet time curve with three event lanes. Modern places
the raised plot and cards on one tray; Clean applies the quiet fill to
each lane inside a single panel. Narrow layouts stack lanes without
reordering events. Changing an accepted outcome changes the original
card in place; a dot and its card refer to the same event. MotionRuntime
clips the curve horizontally and advances the summary at bounded
checkpoints, so resize does not reset the replay clock.
The stock faint timestamps measure 1.94–2.52:1 across the four presets.
Quiet labels and some light success pills also fall below 4.5:1. Retain
the original ink and report the actual foreground/background pair in the
strict contrast audit; this is not an accessibility pass.

AgentFindings shows a progress table and newest-first activity within a
bounded surface. Modern separates the agent and finding cards on a tray;
Clean uses one card with hairline section boundaries. Selection dims
the other agent rows without removing them, and the inline filter chip
can clear the filter. Numbers and step captions move in place, pulse
and sheen stay inside the accepted progress fill, and stable finding
IDs retain their hit and focus owners as the feed grows.
The original faint timestamps and quiet counts/details are retained
and measured against their actual panel or tray in the strict contrast
audit, including the low-contrast Modern Light warning text on the tray.

AgentLedger leaves time and Undo or Redo in the same trailing slot; hovering
or focusing a command does not resize the row. The before/after line, logo
and stable action keep their identities across Host decisions and theme
switches. A preview shows reversible and irreversible dependents separately.
Only an accepted undo snapshot starts the staggered strike across each
affected row. Narrow rows use icons for commands and retain their full spoken
labels.
The original before-value arrows and undone timestamps measure as low as
1.78:1 in Modern Light. Quiet metadata and the faded system glyph on an
undone row can also miss 4.5:1; preserve these combinations and keep them
visible in the strict contrast audit.

CompactBreakdown keeps a stable segment order and palette while the ranked
list follows accepted amounts. Period previews never change the accepted
selection; hovering or focusing a category temporarily replaces the total.
Modern uses a raised panel over a tray and a continuous period track. Clean
uses a single panel with individual period chips and a hairline footer.
Numeric and segment motion use the shared native runtime. At 250 logical
pixels and below, the periods move to their own line and the visible share
column disappears, not its accessible description. Decorative tints and
muted metadata keep the original palette.
The Modern Light caption measures 3.23:1 on the panel and the footer 2.84:1
on its tray. The Clean Light muted caption measures about 3.36:1 on white.
These intentionally unchanged pairs fall below the 4.5:1 text target.

UsageMeter uses a 460px maximum card with period controls, a shared
animated quota track, category legend, daily bars, plan limits and an
action footer. Modern places its footer on the tray; Clean puts it under
a hairline inside the card. Modern's warning dot stays amber at the
limit; Clean differentiates warning and over-limit states. Below 301
logical pixels the period choices get their own row, the quota cells
stack, and category shares remain accessible even when visually hidden.
Observed zero and future data stay distinct. The bounded qualifier track
leaves real unfilled space under capacity and fills without overflowing
when accepted or interpolated usage exceeds capacity.

ChangeSummary fits a 480px maximum card. Modern places the metric list
inside its tray; Clean keeps the heading and metric rows in one card.
The wide rows use 40:60 columns and the narrow rows wrap below the label.
The decorative mark describes direction independently of its colour;
the accessibility description includes both the accepted detail and
the outcome. Native entry motion staggers a bounded number of rows.
The Modern Light accent heading on the tray keeps its original 2.05:1
contrast; the outcome is not communicated by the heading or colour alone.

TipsPanel fits a 760px maximum card. Only the tip grid scrolls, with
3/2/1 columns at the authored viewport breakpoints and stable item
identity across them. Modern uses a tray with a raised tile area; Clean
has hairline dividers and inset tile borders. It uses the common Button
and motion pipeline. Pending, rejected and accepted actions go through
the same component, without optimistic completion in the panel.
The Modern Light completed label retains the blue on the completed tile
(2.24:1). Both light-mode cases are below the 4.5:1 text target.

ProjectCard and ExperimentCard keep the summary header in place as
their bodies unfold. Modern uses a tray; Clean uses a single panel and
a hairline above its action footer. ProjectCard's 32px ring draws and
hit-tests the same native paths. Task rows remain keyboard-accessible.
Member preview and pinned attention dim other members' work without
altering task identity. ExperimentCard gives the collapsed view a small
column chart and the expanded view keyed variant rows with numeric and
bar motion. Missing data never produces a false zero or relative lift.
Visibility, comparison and action callbacks remain separate.

NotificationStack reserves the front notice's measured height and up to
two 12px peeks. Rear shells have no text, focus or hit handlers. A
dismissal animates a non-interactive paint copy after Host acceptance;
the new notice text enters after the old copy has mostly faded, rather
than crossfading two full sets of text. The corner artwork uses native
SVG clip geometry and a diagonal fade, not the exact radial mask.

AwaySummary uses a tray with an inset list in Modern and a single card
with hairline separators in Clean. Both keep the noticed task's first
priority and arrival rank. The folded divider keeps the disclosure
control's native identity and can return focus to the row that was
active before folding. Duration/clock labels use the view locale and
the caller's time zone.

KpiChart pairs a headline with a compact multi-series plot. Modern uses
a bordered raised card, inline delta, and open footer; Clean uses a
shadow edge, tinted delta, and a separate hairline footer. Both keep the
three-value y-axis inside the card, place the sample tooltip within the
measured plot, and align its guide and native pointer sampling with the same projected
paths. A narrow card tightens spacing without dropping observations.

BreakdownChart uses a framed tray and dashed guides in Modern, and a
single card with hairline guides in Clean. The heading total, signed
stacks and detail table use one ordered set of grouped values. Hidden
legend segments are muted in the table and omitted from the total; at
least one remains visible. Mouse and keyboard attention link the chart
and row without altering the records. The detail table scrolls
horizontally when its columns cannot fit.

RunningNow is a compact task counter and nonmodal popover. Modern puts the
task list on an inset surface inside a tray; Clean keeps a single bordered
surface and a hairline footer. The same 52-pixel row geometry hosts a task's
progress, status and action across accepted phase changes. Unknown total
steps use a dash; requesting a mutation does not alter the progress ring.

ScheduledRuns pairs a waiting stack with a weekly session grid and a
visible capacity measure. A hairline marks the current session and past
sessions remain readable but cannot receive tasks. The board, filter
and calendar all retain business identities across a theme change.
After acceptance, shared layout geometry animates a task entering or
moving between sessions; an unaccepted proposal leaves the cards in place.

ToolCallCard uses a mono name, compact status and disclosure header.
Modern gives the glyph an inset tile; Clean places the glyph directly
on the card. Arguments and results appear as native, gutterless code
sections. Long sections fade at the initial cap and may expand to a
scrollable viewport. ToolCallGroup presents stacked tool glyphs and an
aggregate status without retaining hidden code viewers.

ContextWindow uses a colored grid of equal token slices, a compact
figure with exact semantic totals and an expandable list of parts.
Modern puts the field and rows on an inset card in a soft tray; Clean
uses a single surface and a hairline footer. Narrow cards fit fewer
columns without changing the number of cells or the accepted total.
Compaction uses a paint-only light wave while pending, rather than
showing an unaccepted reduction.

AgentCursors gives Modern a raised document card in a tray and Clean a
single document surface under a hairline header. Both display controlled
tabs, a compact presence roster, a scrollable document, and anchored review
controls. Modern's presence faces are 28 logical pixels across and Clean's
are 24. A participant outside the document is painted as one group at 40%
opacity, including when followed. The follow button's pressed state and
member-colored backing follow the accepted Host value, not the request.
The same shaped text owns removed and inserted spans, caret and
selection geometry, and interactive suggestion regions. Figure and cell
coordinates are derived from their presented bounds. A present reviewer
glides to the new anchor; fast typing has its own shorter motion, and
selection paint stays on the currently published text. A held image corner
draws the frame and pointer inside that image's current native layout, including
while its width changes. The table's card grows in layout as accepted rows
arrive. Reduced/None expose the final position and size without transitions.
Host acceptance, not a local visual change, authoritatively changes the document.
An isolated alpha mask fades the paper and selection ink from 50% to 97%
of the sheet height. It reveals the real backdrop, including through
overlapping translucent children. The viewport, scrolling, focus and
review hit owners keep their geometry; comment and suggestion editors
paint above the paper at full coverage. Decorations follow the sheet's
fade without becoming input owners.
Consecutive prose now shares the figure's measured border box. Each text
line that intersects its exclusion wraps in the remaining width; lines
below return to the full measure even within the same paragraph. A heading,
list, or code block clears the figure. Logical placement mirrors in RTL.
At narrower widths the same figure and paragraph nodes stack instead.

ApprovalGates separates its run summary, active review stack, blocked steps
and expandable history. The peeking cards are decorative; the front card
owns the only active input. Code and file comparisons use the shared native
document surfaces. AutonomyDial keeps both group headings and keyed actions
in one retained parent, allowing accepted reclassification to move the
same row without replacing its pin control. Modern has an inset segmented
track and raised rows; Clean uses compact chips and a thin divider.

SplitPane, Resizable, PanZoom, drag/drop, selection and rotation wrappers do
not acquire decorative backgrounds merely to look migrated. Their visible
handles, affordances and feedback consume design recipes; their content and
size policy remain application-owned. Layouts and patterns compose these same
public controls. See [component coverage](component-coverage.md).
