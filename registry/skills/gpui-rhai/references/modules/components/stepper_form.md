# StepperForm

`components/stepper_form` · export `StepperForm` · version 0.1.0. Generated from
[`registry/components/stepper_form.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/stepper_form.rhai); do not edit.

StepperForm presents a reliable workflow around caller-owned keyed form content.

```rhai
import "components/stepper_form" as stepper_form;

stepper_form::StepperForm(#{key:"setup",steps:[#{id:"team",title:"Team",content:team_fields}],on_submit:Fn("save")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Disables step navigation and submission. |
| `initial_step` | string or `()` | — | Initial and reset step for local navigation; otherwise the first step. |
| `key` | string | required | Stable identity of the form and retained panes. |
| `on_invalid` | callback or `()` | — | A blocked attempt, with step and message. The application can reveal per-field errors in its pane. |
| `on_reset` | callback or `()` | — | Request to reset the Host-owned data/status from the success pane. |
| `on_step_change` | callback or `()` | — | Next step proposal with step, from, and reason. The visible step changes only after acceptance. |
| `on_submit` | callback or `()` | — | Emitted once per accepted submit attempt; payload has step and request. Read your application's values here. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `status` | `"editing"` or `"submitting"` or `"failed"` or `"succeeded"` | `"editing"` | Host-owned submission status. Echo submitting when starting an asynchronous request. |
| `step` | string or `()` | — | Controlled step ID; apply an accepted step_change proposal to this prop. |
| `steps` | array of object (at most 16) | required | Ordered keyed workflow; empty renders no workflow. |
| `style` | style | — | Style merged over the root part. |
| `submission_error` | string or `()` | — | Message on failed submission. Failed requests may be retried. |
| `submit_label` | string or `()` | — | Last action label, defaults to Finish. |
| `submitting_label` | string or `()` | — | Label while waiting for submission. |
| `success` | node or `()` | — | Custom success content; shown only when Host reports succeeded. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node or `()` | — | Caller-owned fields. Keep values and field-validation policy in the application. |
| `description` | string or `()` | — | Quiet description below the heading. |
| `error` | string or `()` | — | Current validation error that blocks forward navigation. It is shown after an invalid attempt. |
| `heading` | string or `()` | — | Pane heading, defaults to title. |
| `id` | string | required | Unique stable step ID. |
| `review` | array of object (at most 32) or `()` | — | Review rows below optional content. |
| `title` | string | required | Short name in progress and compact headings. |

### `steps[].review[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `edit_step` | string or `()` | — | Earlier step to edit. Continue there returns directly to this review. |
| `key` | string | required | Stable identity within the review. |
| `label` | string | required | Label of the reviewed field. |
| `text` | string | required | Readable current review value. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `invalid` | `on_invalid` | object | Forward navigation was rejected by the current step's validation. |
| `reset` | `on_reset` | none | Reset requested from success. The Host must restore the form values, status, and any controlled step. |
| `step_change` | `on_step_change` | object | Navigation proposal. Writing step back is the Host's acceptance. |
| `submit` | `on_submit` | object | A request to submit caller-owned data. Request is monotonic within this mounted form; the Host owns asynchronous results. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `back`, `bar`, `bar_active`, `body`, `card`, `check`, `compact`, `description`, `edit`, `footer`, `header`, `heading`, `marker`, `marker_current`, `marker_done`, `pane`, `primary`, `primary_busy`, `progress`, `review`, `review_label`, `review_row`, `review_value`, `root`, `step`, `step_current`, `step_done`, `step_future`, `step_name`, `submit_error`, `success`, `success_mark`, `validation`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.control`, `typography.stepper_title`
- Environment: `density`

## Dependencies

[`components/accordion`](accordion.md), [`components/button`](button.md), [`components/sliding_panel`](sliding_panel.md)
