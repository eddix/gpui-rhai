# Rhai for gpui-rhai

gpui-rhai scripts are [Rhai](https://rhai.rs/book/) 1.26. This page covers the
part of the language a view uses, the conventions the runtime and components
expect, and the mistakes that make `gpui-rhai check` fail. Every snippet marked
`check` below is run by the test suite: `pass` snippets pass `check` (which
parses, validates known calls and renders one headless first frame), `error`
and `warning` snippets produce the quoted message.

## A view

An entry script defines `view(ctx)`, which returns one node. State is declared
by `state_schema()`; callbacks are named functions that receive `ctx` and the
event payload.

<!-- check: pass -->
```rhai
import "components/button" as button;

fn state_schema() {
    #{ fields: #{ count: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } } } }
}

fn clicked(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }

fn view(ctx) {
    column([
        text(`Clicked ${ctx.get_state("count")} times`),
        button::Button(#{ key: "add", text: "Add one", on_click: Fn("clicked") }),
    ])
}
```

`view` describes the interface and has no side effects. Files, network and
platform services are capabilities a Rust Host registers.

## Values

| Value | Literal | `type_of` |
|---|---|---|
| nothing | `()` | `"()"` |
| integer, float | `42`, `0.5` | `"i64"`, `"f64"` |
| string | `"text"`, `` `Hello ${name}` `` | `"string"` |
| array | `[1, 2, 3]` | `"array"` |
| map | `#{ key: 1, "two words": 2 }` | `"map"` |
| function pointer | `Fn("clicked")` | `"Fn"` |
| runtime values | `px(8)`, `style()`, `text("hi")` | `"Length"`, `"Style"`, `"UiNode"` |

- `()` stands for "no value": an absent optional prop, a function without a
  result, a missing map key.
- Integers and floats mix in arithmetic (`1 + 0.5` is `1.5`); integer division
  truncates (`5 / 2` is `2`). A `number` prop accepts either.
- A template string in backticks interpolates any expression: `` `${n * 2} items` ``.
- Reading a key a map does not have gives `()`; test for a key with `in`.

<!-- check: pass -->
```rhai
fn view(ctx) {
    let props = #{ label: "Name" };
    let size = if "size" in props && props.size != () { props.size } else { "md" };
    if props.missing != () { throw "a missing key reads as ()"; }
    text(`${props.label} (${size})`)
}
```

### Strings and arrays change in place

Several methods modify their receiver and return `()`: on strings `trim`,
`make_upper`, `make_lower`, `pad`, `truncate`, `clear`; on arrays `push`,
`insert`, `sort`, `reverse`, `clear`. Methods such as `to_upper`, `split`,
`filter`, `map` and `contains` return a new value.

<!-- check: pass -->
```rhai
fn view(ctx) {
    let raw = " a,b ";
    raw.trim();                        // raw is now "a,b"; trim returns ()
    let parts = raw.split(",");
    let items = [3, 1, 2];
    items.sort();
    let doubled = items.map(|x| x * 2);
    text(`${parts.len()} parts, first ${doubled[0]}`)
}
```

<!-- check: error "Function not found: split" -->
```rhai
fn view(ctx) {
    let cleaned = " a,b ".trim();      // () : trim works in place
    text(`${cleaned.split(",").len()}`)
}
```

Arrays hold at most 10,000 items and maps 100,000 entries in a script; larger
data belongs in a native collection the Host provides
([native collections](https://github.com/eddix/gpui-rhai/blob/main/docs/native-collections.md)).

## Reserved words

Rhai reserves words it may use later, and they cannot name a variable, a
function or an unquoted map key: `async`, `await`, `case`, `default`, `go`,
`goto`, `is`, `match`, `module`, `new`, `nil`, `null`, `package`, `protected`,
`public`, `shared`, `spawn`, `static`, `super`, `sync`, `thread`, `use`, `var`,
`void`, `with`, `yield`, and the built-ins `call`, `curry`, `debug`, `eval`,
`print`, `type_of`, `is_def_var`, `this`. Quote such a key in a map: component
schemas write `"default": #{ ... }` and metadata writes `"export"`.

<!-- check: error "'default' is a reserved keyword" -->
```rhai
fn view(ctx) {
    let options = #{ default: 1 };
    text("never rendered")
}
```

## Functions

- The last expression of a function is its result; `return` also works.
- A function sees only its parameters. It cannot read a `let` variable of the
  script; a top-level `const` is reachable as `global::NAME`.

<!-- check: pass -->
```rhai
const LIMIT = 3;

fn clamp(value) { if value > global::LIMIT { global::LIMIT } else { value } }

fn view(ctx) { text(`${clamp(5)}`) }
```

<!-- check: error "Variable not found: limit" -->
```rhai
let limit = 3;

fn clamp(value) { if value > limit { limit } else { value } }

fn view(ctx) { text(`${clamp(5)}`) }
```

- `if` and `switch` are expressions: `let tone = if error { "danger" } else { "neutral" };`.
- `for item in items`, `for (item, index) in items` and `for i in 0..count`
  loop; `break` and `continue` work as usual.
- `throw "message"` fails the current render or callback; the runtime rolls the
  transaction back and reports the error.

## Callbacks

A callback prop takes a function pointer to a named function. `curry` binds
leading arguments, so one function can serve many rows:

<!-- check: pass -->
```rhai
import "components/button" as button;

fn state_schema() {
    #{ fields: #{ picked: #{ schema: #{ type: "string" },
        "default": #{ type: "string", value: "" } } } }
}

// Called as pick(id, ctx, payload).
fn pick(id, ctx, payload) { ctx.set_state("picked", id); }

fn view(ctx) {
    let buttons = [];
    for id in ["alpha", "beta"] {
        buttons.push(button::Button(#{ key: id, text: id, on_click: Fn("pick").curry(id) }));
    }
    column(buttons)
}
```

A closure (`|ctx, payload| ...`) is fine inside one render, for example in
`items.map(|x| ...)`, but it cannot be stored as a callback: the runtime keeps
callbacks across renders and a closure belongs to one evaluation.

<!-- check: error "anonymous or capturing functions cannot escape" -->
```rhai
import "components/button" as button;

fn view(ctx) {
    button::Button(#{ key: "save", text: "Save", on_click: |ctx, payload| () })
}
```

### Do not name a function like a built-in

Rhai resolves a script function before a built-in of the same name and arity,
also for method calls inside imported components: `fn index_of(values, key)`
takes over every `x.index_of(a, b)`. `check` warns:

<!-- check: warning "builtin-shadow" -->
```rhai
fn index_of(values, key) { 0 }

fn view(ctx) { text("ok") }
```

## Modules

`import "<module id>" as <alias>;` at the top of a script makes a module's
exports reachable as `alias::Name`. The id is the registry path
(`components/button`, `layouts/region`, `patterns/app_shell`), not a file path.
A module cannot see the functions of the script that imports it, so callbacks
are passed as `Fn("name")` pointers to the importer's functions.

## Nodes and styles

- `row([...])` and `column([...])` are flex containers; `box([...])` has no
  direction, so center and align inside `row` or `column`.
- Styles chain: `style().padding_x(theme_length("metrics.inset")).gap(theme_length("space.related"))`,
  applied with `.with_style(...)`. Read lengths and colors from tokens rather
  than writing pixels; see [the token layers](user-guide.md#9-design-tokens-themes-and-theme-studio).
- Lengths are never negative: `px(-4)` fails; use `offset_px(-4)` for a
  negative offset such as a margin or an inset.

<!-- check: error "length must be finite and non-negative" -->
```rhai
fn view(ctx) { box([]).with_style(style().margin_top(px(-4))) }
```

<!-- check: pass -->
```rhai
fn view(ctx) { box([]).with_style(style().margin_top(offset_px(-4))) }
```

## Limits

The runtime sets these limits for every script:

| Limit | Value |
|---|---|
| expression depth | 64 at the top level, 32 inside a function |
| call depth | 64 |
| array items | 10,000 |
| map entries | 100,000 |
| string length | 1 MiB |

A call, an array and a method chain each cost depth, so a function body nests
only a few containers: five `column([...])` levels pass, six do not, and five
with a style on each do not either. Build a screen from small functions, one
per region or section, each returning a node; it reads better too.

<!-- check: pass -->
```rhai
fn view(ctx) {
    column([column([column([column([column([text("five levels")])])])])])
}
```

<!-- check: error "Expression exceeds maximum complexity" -->
```rhai
fn view(ctx) {
    column([column([column([column([column([column([text("six levels")])])])])])])
}
```

<!-- check: pass -->
```rhai
fn header(title) { row([text(title)]).with_style(style().gap(theme_length("space.related"))) }

fn body(lines) { column(lines.map(|line| text(line))) }

fn view(ctx) {
    column([header("Hosts"), body(["api-01", "api-02"])])
        .with_style(style().gap(theme_length("space.group")))
}
```
