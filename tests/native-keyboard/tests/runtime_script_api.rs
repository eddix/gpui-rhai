//! Script API contracts of the runtime itself: per-handler payload values,
//! parent signal bindings and focus declarations.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
use gpui_rhai::*;

type Records = Rc<RefCell<Vec<(String, UiValue)>>>;

#[derive(Clone)]
struct Capture {
    records: Records,
}

impl CapabilityHandler for Capture {
    fn call(&mut self, _: &str, input: UiValue) -> Result<UiValue, String> {
        let UiValue::Map(mut input) = input else {
            return Err("record takes #{ handler, payload }".into());
        };
        let Some(UiValue::String(handler)) = input.remove("handler") else {
            return Err("record needs a handler name".into());
        };
        let payload = input.remove("payload").unwrap_or(UiValue::Null);
        self.records.borrow_mut().push((handler, payload));
        Ok(UiValue::Null)
    }
}

impl ScriptViewExtension for Capture {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .capabilities
            .register(
                CapabilityDescriptor {
                    id: CapabilityId::parse("app.capture").unwrap(),
                    version: semver::Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "run".into(),
                        CapabilityMethod {
                            input: ValueSchema::UiValue,
                            output: ValueSchema::Null,
                        },
                    )]),
                },
                self.clone(),
            )
            .map_err(|error| error.to_string())
    }
}

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn mount(cx: &mut TestAppContext, source: &str) -> (VisualTestContext, ScriptViewHandle, Records) {
    cx.update(gpui_rhai::install);
    let records = Records::default();
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), source.to_owned())])),
        r#"fn theme(){#{family:"Api",name:"Dark",mode:"dark",tokens:#{}}}"#,
    )
    .manifest(
        AppManifest::new(entry)
            .with_capability("app.capture", "*")
            .unwrap(),
    )
    .extension(Capture {
        records: records.clone(),
    })
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("api", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("api"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view, records)
}

fn center(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
) -> gpui::Point<gpui::Pixels> {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let bounds = tree
        .find_by_role_and_name("button", name)
        .next()
        .and_then(|node| node.geometry)
        .unwrap()
        .visual;
    #[allow(clippy::cast_possible_truncation)]
    point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    )
}

const VALUE_HANDLERS: &str = r#"
fn record(ctx, handler, payload) {
    ctx.call_capability("app.capture", "run", #{ handler: handler, payload: payload });
}
fn plain(ctx, payload) { record(ctx, "plain", payload); }
fn first(ctx, payload) { record(ctx, "first", payload); }
fn second(ctx, payload) { record(ctx, "second", payload); }
fn hover(ctx, payload) { record(ctx, "hover", payload); }
fn hover_value(ctx, payload) { record(ctx, "hover_value", payload); }
fn view(ctx) {
    column([
        box([text("Go")]).accessibility_role("button").accessibility_label("Go")
            .on_click(Fn("plain"))
            .on_click_value(Fn("first"), 1)
            .on_click_value(Fn("second"), 2)
            .on_hover_change(Fn("hover"))
            .on_hover_value(Fn("hover_value"), "row")
            .with_style(style().width(px(80)).height(px(32))),
        box([text("Away")]).with_style(style().width(px(80)).height(px(32))),
    ])
}
"#;

fn take(records: &Records, names: &[&str]) -> Vec<(String, UiValue)> {
    let taken = records
        .borrow()
        .iter()
        .filter(|(name, _)| names.contains(&name.as_str()))
        .cloned()
        .collect();
    records.borrow_mut().clear();
    taken
}

fn clicks(plain: UiValue, first: UiValue, second: UiValue) -> Vec<(String, UiValue)> {
    vec![
        ("plain".to_owned(), plain),
        ("first".to_owned(), first),
        ("second".to_owned(), second),
    ]
}

#[gpui::test]
fn each_value_handler_receives_its_own_value(cx: &mut TestAppContext) {
    let (mut visual, view, records) = mount(cx, VALUE_HANDLERS);
    let go = center(&mut visual, &view, "Go");

    visual.simulate_mouse_move(go, None, Modifiers::none());
    settle(&mut visual);
    assert_eq!(
        take(&records, &["hover", "hover_value"]),
        vec![
            ("hover".to_owned(), UiValue::Bool(true)),
            (
                "hover_value".to_owned(),
                UiValue::Map(BTreeMap::from([
                    ("hovered".to_owned(), UiValue::Bool(true)),
                    ("value".to_owned(), UiValue::String("row".to_owned())),
                ]))
            ),
        ],
        "a plain hover handler keeps the bool payload beside a value handler"
    );

    visual.simulate_mouse_down(go, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(go, MouseButton::Left, Modifiers::none());
    settle(&mut visual);
    assert_eq!(
        take(&records, &["plain", "first", "second"]),
        clicks(UiValue::Null, UiValue::Integer(1), UiValue::Integer(2)),
        "a pointer click gives each handler its own payload"
    );

    visual.simulate_keystrokes("enter");
    settle(&mut visual);
    assert_eq!(
        take(&records, &["plain", "first", "second"]),
        clicks(UiValue::Null, UiValue::Integer(1), UiValue::Integer(2)),
        "Enter on the focused node gives each handler its own payload"
    );

    let dispatch = |payload: Option<UiValue>| AutomationCommand::Dispatch {
        locator: AutomationLocator::RoleName {
            role: "button".into(),
            name: "Go".into(),
        },
        event: "click".into(),
        payload,
    };
    visual
        .update(|window, cx| view.automate(dispatch(None), window, cx))
        .unwrap();
    settle(&mut visual);
    assert_eq!(
        take(&records, &["plain", "first", "second"]),
        clicks(UiValue::Null, UiValue::Integer(1), UiValue::Integer(2)),
        "an automation click gives each handler its own payload"
    );
    visual
        .update(|window, cx| view.automate(dispatch(Some(UiValue::Integer(9))), window, cx))
        .unwrap();
    settle(&mut visual);
    assert_eq!(
        take(&records, &["plain", "first", "second"]),
        clicks(
            UiValue::Integer(9),
            UiValue::Integer(9),
            UiValue::Integer(9)
        ),
        "an explicit automation payload reaches every handler"
    );
}

const PARENT_SIGNAL: &str = r#"
define_component(#{metadata:#{id:"test/owner","export":"Owner",version:"0.0.1",
 runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},
 schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},
  signal_key:#{schema:#{type:"string"},required:true,sensitive:false}},
 state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("owner")});
fn rows(){let data=[];for i in 0..40{data.push(#{key:`row-${i}`});}data}
fn owner(ctx,props){
 let width=optional_float_signal("width");
 virtual_collection(#{key:"rows",label:"Rows",data:rows(),height:96,estimated_height:24,
  overdraw_pixels:0},Fn("row").curry(props.signal_key))
}
fn row(signal_key,ctx,p){text(p.key).with_key(p.key).bind_parent_signal(ctx,"width_override",signal_key)}
"#;

fn mount_owner(signal_key: &str) -> Result<(RuntimeEngine, ScriptLifecycle), String> {
    let mut engine = RuntimeEngine::new();
    let compiled = engine
        .compile(&format!(
            "{PARENT_SIGNAL}\nfn view(ctx){{render_component(\"test/owner\",#{{key:\"owner\",signal_key:\"{signal_key}\"}})}}"
        ))
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut life = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "owner"),
        Some("main".into()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    life.start(&mut engine).map_err(|error| error.to_string())?;
    Ok((engine, life))
}

fn row_bindings(node: &UiNode, found: &mut Vec<(usize, NativeSignal)>) {
    match node.kind() {
        UiNodeKind::VirtualCollection { spec } => {
            for (index, row) in &spec.realized {
                if let Some((_, signal)) = row.signal_bindings().next() {
                    found.push((*index, signal.clone()));
                }
            }
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                row_bindings(child, found);
            }
        }
        _ => {}
    }
}

fn collection_id(node: &UiNode) -> Option<VirtualCollectionId> {
    match node.kind() {
        UiNodeKind::VirtualCollection { spec } => Some(spec.id.clone()),
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(collection_id)
        }
        _ => None,
    }
}

#[test]
fn virtual_items_bind_the_signal_their_owner_declares() {
    let owner = ComponentInstancePath::root("App", "owner").child("Owner", "owner");
    let (mut engine, mut life) = mount_owner("width").unwrap();
    let mut initial = Vec::new();
    row_bindings(life.root().unwrap(), &mut initial);
    assert!(!initial.is_empty(), "the initial render realizes rows");
    for (_, signal) in &initial {
        assert_eq!(
            signal.id().component(),
            &owner,
            "an initial row binds the owner's signal"
        );
        assert_eq!(signal.id().key(), "width");
    }

    // A row realized after commit resolves the committed signal.
    let id = collection_id(life.root().unwrap()).unwrap();
    life.runtime().borrow().virtual_requests.request(id, [30]);
    life.realize_virtual_requests(&mut engine).unwrap();
    let mut later = Vec::new();
    row_bindings(life.root().unwrap(), &mut later);
    let (_, signal) = later.iter().find(|(index, _)| *index == 30).unwrap();
    assert_eq!(signal, &initial[0].1, "a later row binds the same signal");

    let error = mount_owner("wdith").err().unwrap();
    assert!(
        error.contains("no mounted native signal `wdith`"),
        "a key no ancestor declares is an error: {error}"
    );
}

const TAB_GROUP: &str = r#"
fn pressed(ctx, label) {
    ctx.call_capability("app.capture", "run", #{ handler: "pressed", payload: label });
}
fn stop(label) {
    box([text(label)]).with_key(label).on_click_value(Fn("pressed"), label)
        .with_style(style().width(px(80)).height(px(24)))
}
fn view(ctx) {
    let group = column([stop("a"), stop("b")]).with_key("group").tab_group();
    column([stop("before"), if GROUP_TAB_STOP { group } else { group.tab_stop(false) }])
}
"#;

/// What each Tab then Enter activates, `""` when focus lands on a node
/// without a click handler.
fn tab_walk(cx: &mut TestAppContext, group_tab_stop: bool) -> Vec<String> {
    let source = TAB_GROUP.replace("GROUP_TAB_STOP", &group_tab_stop.to_string());
    let (mut visual, view, records) = mount(cx, &source);
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    let mut walk = Vec::new();
    for _ in 0..4 {
        visual.simulate_keystrokes("tab enter");
        settle(&mut visual);
        let pressed = records.borrow_mut().drain(..).next();
        walk.push(match pressed {
            Some((_, UiValue::String(label))) => label,
            _ => String::new(),
        });
    }
    walk
}

#[gpui::test]
fn a_tab_group_is_a_tab_stop_unless_tab_stop_false(cx: &mut TestAppContext) {
    assert_eq!(
        tab_walk(cx, true),
        ["before", "", "a", "b"],
        "tab_group() alone puts the group itself in the Tab order"
    );
}

#[gpui::test]
fn a_tab_group_with_tab_stop_false_passes_tab_to_its_items(cx: &mut TestAppContext) {
    assert_eq!(tab_walk(cx, false), ["before", "a", "b", "before"]);
}
