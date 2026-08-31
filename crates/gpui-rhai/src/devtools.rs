use std::collections::{BTreeMap, VecDeque};

#[cfg(feature = "dev-reload")]
use gpui::{
    AnyElement, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    deferred, div, px, rgba,
};

use crate::{
    ComponentRegistry, ExecutionTiming, PrimitiveValue, SourceLocation, StateInstanceSnapshot,
    StoreSnapshot, StyleProperties, ThemeVariant, UiNode, UiNodeKind, UiRuntimeState, UiValue,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeTraceKind {
    Reconcile,
    Event,
    Action,
    Capability,
    Task,
    Subscription,
    State,
    Store,
    Signal,
    Theme,
    Locale,
    Reload,
    Window,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeTrace {
    pub sequence: u64,
    pub kind: RuntimeTraceKind,
    pub scope: String,
    pub message: String,
    pub payload: Option<UiValue>,
    pub sensitive: bool,
}

#[derive(Clone, Debug)]
pub struct TraceBuffer {
    capacity: usize,
    next_sequence: u64,
    traces: VecDeque<RuntimeTrace>,
}

impl Default for TraceBuffer {
    fn default() -> Self {
        Self::new(200)
    }
}

impl TraceBuffer {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            next_sequence: 1,
            traces: VecDeque::new(),
        }
    }

    pub fn push(
        &mut self,
        kind: RuntimeTraceKind,
        scope: impl Into<String>,
        message: impl Into<String>,
        payload: Option<UiValue>,
        sensitive: bool,
    ) {
        if self.traces.len() == self.capacity {
            self.traces.pop_front();
        }
        self.traces.push_back(RuntimeTrace {
            sequence: self.next_sequence,
            kind,
            scope: scope.into(),
            message: message.into(),
            payload,
            sensitive,
        });
        self.next_sequence = self.next_sequence.saturating_add(1);
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<RuntimeTrace> {
        self.traces.iter().cloned().collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorNode {
    pub path: String,
    pub kind: String,
    pub key: Option<String>,
    pub source: Option<SourceLocation>,
    pub props: BTreeMap<String, String>,
    pub attributes: BTreeMap<String, String>,
    pub handlers: Vec<String>,
    pub animations: Vec<String>,
    pub style: StyleProperties,
    pub children: Vec<InspectorNode>,
}

#[derive(Clone, Debug)]
pub struct InspectorSnapshot {
    pub root: Option<InspectorNode>,
    pub state: Vec<StateInstanceSnapshot>,
    pub stores: Vec<StoreSnapshot>,
    pub theme_family: String,
    pub theme_name: String,
    pub theme_colors: BTreeMap<String, String>,
    pub traces: Vec<RuntimeTrace>,
    pub timings: Vec<ExecutionTiming>,
    pub dirty: Vec<String>,
    pub components: Vec<InspectorComponent>,
    pub signals: Vec<InspectorSignal>,
    pub effects: Vec<InspectorEffect>,
    pub timers: Vec<InspectorTimer>,
    pub element_refs: Vec<InspectorElementRef>,
    pub geometry_nodes: usize,
    pub pointer_captures: BTreeMap<u64, u64>,
    pub locale_readers: usize,
    pub viewport_readers: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorComponent {
    pub id: String,
    pub export: String,
    pub props: BTreeMap<String, String>,
    pub slots: Vec<String>,
    pub parts: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorSignal {
    pub id: String,
    pub kind: String,
    pub value: String,
    pub revision: u64,
    pub last_writer: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorEffect {
    pub id: String,
    pub activation: u64,
    pub dependencies: String,
    pub start: String,
    pub cleanup: String,
    pub generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorTimer {
    pub id: String,
    pub delay_ms: u64,
    pub remaining_ms: u64,
    pub declaration_paused: bool,
    pub interaction_paused: bool,
    pub callback: String,
    pub generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorElementRef {
    pub id: String,
    pub node: u64,
}

struct InspectorRuntimeMechanisms {
    signals: Vec<InspectorSignal>,
    effects: Vec<InspectorEffect>,
    timers: Vec<InspectorTimer>,
    element_refs: Vec<InspectorElementRef>,
    geometry_nodes: usize,
    pointer_captures: BTreeMap<u64, u64>,
    locale_readers: usize,
    viewport_readers: usize,
}

impl InspectorSnapshot {
    #[must_use]
    pub fn capture(
        root: Option<&UiNode>,
        runtime: &UiRuntimeState,
        theme: &ThemeVariant,
        components: &ComponentRegistry,
        timings: Vec<ExecutionTiming>,
    ) -> Self {
        let mechanisms = inspect_runtime_mechanisms(runtime);
        Self {
            root: root.map(|root| inspect_node(root, "root")),
            state: runtime.component_state.inspect(),
            stores: runtime.stores.inspect(),
            theme_family: theme.family.clone(),
            theme_name: theme.name.clone(),
            theme_colors: theme
                .tokens
                .colors
                .iter()
                .map(|(name, color)| (name.clone(), format!("#{:08x}", color.as_rgba_hex())))
                .collect(),
            traces: runtime.traces.snapshot(),
            timings,
            dirty: runtime
                .dirty_components()
                .iter()
                .map(ToString::to_string)
                .collect(),
            components: components
                .iter()
                .map(|(id, component)| InspectorComponent {
                    id: id.to_string(),
                    export: component.metadata.export.clone(),
                    props: component
                        .schema
                        .props
                        .iter()
                        .map(|(name, field)| {
                            (
                                name.clone(),
                                if field.sensitive {
                                    "<sensitive>".to_owned()
                                } else {
                                    format!("{:?}", field.schema)
                                },
                            )
                        })
                        .collect(),
                    slots: component.schema.slots.keys().cloned().collect(),
                    parts: component.schema.parts.iter().cloned().collect(),
                })
                .collect(),
            signals: mechanisms.signals,
            effects: mechanisms.effects,
            timers: mechanisms.timers,
            element_refs: mechanisms.element_refs,
            geometry_nodes: mechanisms.geometry_nodes,
            pointer_captures: mechanisms.pointer_captures,
            locale_readers: mechanisms.locale_readers,
            viewport_readers: mechanisms.viewport_readers,
        }
    }
}

fn inspect_runtime_mechanisms(runtime: &UiRuntimeState) -> InspectorRuntimeMechanisms {
    InspectorRuntimeMechanisms {
        signals: runtime
            .signals
            .inspect()
            .into_iter()
            .map(|signal| InspectorSignal {
                id: format!(
                    "{}/{}:{}",
                    signal.id.component(),
                    signal.id.key(),
                    signal.id.kind().as_str()
                ),
                kind: signal.id.kind().as_str().to_owned(),
                value: format!("{:?}", signal.value),
                revision: signal.revision,
                last_writer: format!("{:?}", signal.last_writer),
            })
            .collect(),
        effects: runtime
            .effects
            .iter_active()
            .map(|(id, effect, activation)| InspectorEffect {
                id: format!("{}/{}", id.component(), id.key()),
                activation,
                dependencies: display_value(effect.dependencies(), false),
                start: effect.start().name().to_owned(),
                cleanup: effect.cleanup().name().to_owned(),
                generation: effect.generation().get(),
            })
            .collect(),
        timers: runtime
            .timers
            .inspect(runtime.clock.now())
            .into_iter()
            .map(|timer| InspectorTimer {
                id: format!("{}/{}", timer.id.component(), timer.id.key()),
                delay_ms: u64::try_from(timer.delay.as_millis()).unwrap_or(u64::MAX),
                remaining_ms: u64::try_from(timer.remaining.as_millis()).unwrap_or(u64::MAX),
                declaration_paused: timer.declaration_paused,
                interaction_paused: timer.interaction_paused,
                callback: timer.callback,
                generation: timer.generation.get(),
            })
            .collect(),
        element_refs: runtime
            .element_refs
            .iter()
            .map(|(id, node)| InspectorElementRef {
                id: format!("{}/{}", id.component(), id.key()),
                node: node.get(),
            })
            .collect(),
        geometry_nodes: runtime.geometry.len(),
        pointer_captures: runtime
            .pointer_capture
            .snapshot()
            .into_iter()
            .map(|(pointer, node)| (pointer, node.get()))
            .collect(),
        locale_readers: runtime.environment_dependencies.locale_reader_count(),
        viewport_readers: runtime.environment_dependencies.viewport_reader_count(),
    }
}

fn inspect_node(node: &UiNode, path: &str) -> InspectorNode {
    let mut props = BTreeMap::new();
    let (kind, descendants): (String, Vec<&UiNode>) = match node.kind() {
        UiNodeKind::Text { text } => {
            props.insert("text".to_owned(), truncate(text, 80));
            ("text".to_owned(), Vec::new())
        }
        UiNodeKind::RichText { text, spans } => {
            (inspect_rich_text(text, spans, &mut props), Vec::new())
        }
        UiNodeKind::Canvas { scene } => (inspect_canvas(scene, &mut props), Vec::new()),
        UiNodeKind::Svg { source } => {
            props.insert("bytes".to_owned(), source.len().to_string());
            ("svg".to_owned(), Vec::new())
        }
        UiNodeKind::Box { children } => ("box".to_owned(), children.iter().collect()),
        UiNodeKind::Fragment { children } => ("fragment".to_owned(), children.iter().collect()),
        UiNodeKind::Custom { primitive } => {
            for (name, value) in primitive.props.iter() {
                props.insert(name.to_owned(), primitive_value(value));
            }
            (
                format!("primitive:{}", primitive.primitive.as_str()),
                Vec::new(),
            )
        }
        UiNodeKind::Image { source } => inspect_image(source, &mut props),
        UiNodeKind::DirectionalImage {
            left_to_right,
            right_to_left,
        } => inspect_directional_image(left_to_right, right_to_left, &mut props),
        UiNodeKind::Overlay {
            trigger,
            content,
            spec,
        } => {
            props.insert("id".to_owned(), spec.id.as_str().to_owned());
            props.insert("kind".to_owned(), format!("{:?}", spec.kind));
            props.insert("placement".to_owned(), format!("{:?}", spec.placement));
            props.insert("open".to_owned(), spec.open.to_string());
            (
                "overlay".to_owned(),
                vec![trigger.as_ref(), content.as_ref()],
            )
        }
        UiNodeKind::Layer { content, spec } => {
            props.insert("id".to_owned(), spec.id.as_str().to_owned());
            props.insert("placement".to_owned(), format!("{:?}", spec.placement));
            props.insert("priority".to_owned(), spec.priority.to_string());
            ("layer".to_owned(), vec![content.as_ref()])
        }
        UiNodeKind::VirtualCollection { spec } => inspect_virtual_collection(spec, &mut props),
        UiNodeKind::ErrorBoundary { child, fallback } => (
            "error_boundary".to_owned(),
            vec![child.as_ref(), fallback.as_ref()],
        ),
    };
    let children = descendants
        .into_iter()
        .enumerate()
        .map(|(index, child)| {
            let segment = child
                .key()
                .map_or_else(|| index.to_string(), |key| format!("key:{}", key.as_str()));
            inspect_node(child, &format!("{path}/{segment}"))
        })
        .collect();
    InspectorNode {
        path: path.to_owned(),
        kind,
        key: node.key().map(|key| key.as_str().to_owned()),
        source: node.source().cloned(),
        props,
        attributes: node
            .attributes()
            .iter()
            .map(|(name, value)| (name.clone(), display_value(value, false)))
            .collect(),
        handlers: node
            .handlers()
            .iter()
            .flat_map(|(event, bindings)| {
                bindings.iter().map(move |binding| {
                    format!(
                        "{event}:{:?}={}",
                        binding.phase(),
                        binding.handler().diagnostic_label()
                    )
                })
            })
            .collect(),
        animations: node
            .animations()
            .iter()
            .map(|animation| format!("{animation:?}"))
            .collect(),
        style: node.style().resolve(&crate::InteractionState::default()),
        children,
    }
}

fn inspect_virtual_collection<'a>(
    spec: &'a crate::VirtualCollectionNodeSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("items".to_owned(), spec.data.len().to_string());
    props.insert("realized".to_owned(), spec.realized.len().to_string());
    (
        "virtual_collection".to_owned(),
        spec.realized.values().collect(),
    )
}

fn inspect_rich_text(
    text: &str,
    spans: &[crate::Span],
    props: &mut BTreeMap<String, String>,
) -> String {
    props.insert("text".to_owned(), truncate(text, 80));
    props.insert("span_count".to_owned(), spans.len().to_string());
    "rich_text".to_owned()
}

fn inspect_canvas(scene: &crate::CanvasScene, props: &mut BTreeMap<String, String>) -> String {
    props.insert(
        "command_count".to_owned(),
        scene.commands().len().to_string(),
    );
    "canvas".to_owned()
}

fn inspect_image<'a>(
    source: &crate::ImageSourceSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("source".to_owned(), inspect_image_source(source));
    ("image".to_owned(), Vec::new())
}

fn inspect_directional_image<'a>(
    left_to_right: &crate::ImageSourceSpec,
    right_to_left: &crate::ImageSourceSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert(
        "left_to_right".to_owned(),
        inspect_image_source(left_to_right),
    );
    props.insert(
        "right_to_left".to_owned(),
        inspect_image_source(right_to_left),
    );
    ("directional_image".to_owned(), Vec::new())
}

fn inspect_image_source(source: &crate::ImageSourceSpec) -> String {
    match source {
        crate::ImageSourceSpec::Handle(handle) => format!("{}#<opaque>", handle.kind()),
        crate::ImageSourceSpec::Asset(asset) => asset.as_str().to_owned(),
    }
}

fn primitive_value(value: &PrimitiveValue) -> String {
    match value {
        PrimitiveValue::Data(value) => display_value(value, false),
        PrimitiveValue::Node(_) => "<node>".to_owned(),
        PrimitiveValue::Nodes(nodes) => format!("<{} nodes>", nodes.len()),
        PrimitiveValue::Callback(callback) => callback.diagnostic_label(),
        PrimitiveValue::Style(_) => "<style>".to_owned(),
        PrimitiveValue::Length(length) => format!("{length:?}"),
        PrimitiveValue::Asset(asset) => asset.as_str().to_owned(),
    }
}

fn display_value(value: &UiValue, sensitive: bool) -> String {
    if sensitive {
        return "<redacted>".to_owned();
    }
    match value {
        UiValue::Null => "null".to_owned(),
        UiValue::Bool(value) => value.to_string(),
        UiValue::Integer(value) => value.to_string(),
        UiValue::Float(value) => value.to_string(),
        UiValue::String(value) => format!("{:?}", truncate(value, 80)),
        UiValue::Array(values) => format!("[{} items]", values.len()),
        UiValue::Map(values) => format!("{{{} fields}}", values.len()),
        UiValue::Handle(handle) => format!("{}#<opaque>", handle.kind()),
    }
}

fn truncate(value: &str, max: usize) -> String {
    let mut output = value.chars().take(max).collect::<String>();
    if value.chars().count() > max {
        output.push('…');
    }
    output
}

#[must_use]
#[cfg(feature = "dev-reload")]
pub(crate) fn inspector_element(snapshot: &InspectorSnapshot) -> AnyElement {
    let mut lines = inspector_header(snapshot);
    if let Some(root) = &snapshot.root {
        append_node_lines(root, 0, &mut lines);
    }
    lines.push("Components".to_owned());
    for component in &snapshot.components {
        lines.push(format!("  {}::{}", component.id, component.export));
        lines.push(format!("    props={:?}", component.props));
        lines.push(format!("    slots={:?}", component.slots));
        lines.push(format!("    parts={:?}", component.parts));
    }
    append_mechanism_lines(snapshot, &mut lines);
    lines.push("State".to_owned());
    for instance in &snapshot.state {
        lines.push(format!("  {}", instance.path));
        for (name, value) in &instance.fields {
            lines.push(format!(
                "    {name}: {}",
                display_value(&value.value, value.sensitive)
            ));
        }
    }
    lines.push("Stores".to_owned());
    for store in &snapshot.stores {
        lines.push(format!("  {:?}:{}", store.id.scope, store.id.name));
        for (name, value) in &store.fields {
            lines.push(format!(
                "    {name}: {}",
                display_value(&value.value, value.sensitive)
            ));
        }
    }
    lines.push("Theme tokens".to_owned());
    for (name, value) in &snapshot.theme_colors {
        lines.push(format!("  {name}: {value}"));
    }
    lines.push("Invalidations".to_owned());
    if snapshot.dirty.is_empty() {
        lines.push("  none pending".to_owned());
    } else {
        lines.extend(snapshot.dirty.iter().map(|path| format!("  dirty: {path}")));
    }
    lines.push("Script timings".to_owned());
    for timing in snapshot.timings.iter().rev().take(20) {
        lines.push(format!(
            "  {:?} {} {:.2?} {} ops{}",
            timing.operation,
            timing.source,
            timing.duration,
            timing.operations,
            if timing.slow { " SLOW" } else { "" }
        ));
    }
    lines.push("Recent traces".to_owned());
    for trace in snapshot.traces.iter().rev().take(20) {
        lines.push(format!(
            "  #{} {:?} {} — {} {}",
            trace.sequence,
            trace.kind,
            trace.scope,
            trace.message,
            trace
                .payload
                .as_ref()
                .map_or_else(String::new, |value| display_value(value, trace.sensitive))
        ));
    }
    deferred(
        div()
            .id("gpui-rhai-inspector")
            .absolute()
            .top_0()
            .right_0()
            .w(px(420.0))
            .h_full()
            .overflow_y_scroll()
            .p_3()
            .bg(rgba(0x1717_1bee))
            .text_color(rgba(0xf4f4_f5ff))
            .text_size(px(11.0))
            .children(lines.into_iter().map(|line| div().child(line))),
    )
    .with_priority(10_000)
    .into_any_element()
}

#[cfg(feature = "dev-reload")]
fn inspector_header(snapshot: &InspectorSnapshot) -> Vec<String> {
    vec![
        format!(
            "GPUI Rhai Inspector — {} / {}",
            snapshot.theme_family, snapshot.theme_name
        ),
        format!(
            "nodes={} state={} stores={} signals={} effects={} timers={} refs={} geometry={} captures={} dirty={} traces={} timings={}",
            snapshot.root.as_ref().map_or(0, count_nodes),
            snapshot.state.len(),
            snapshot.stores.len(),
            snapshot.signals.len(),
            snapshot.effects.len(),
            snapshot.timers.len(),
            snapshot.element_refs.len(),
            snapshot.geometry_nodes,
            snapshot.pointer_captures.len(),
            snapshot.dirty.len(),
            snapshot.traces.len(),
            snapshot.timings.len()
        ),
    ]
}

#[cfg(feature = "dev-reload")]
fn append_mechanism_lines(snapshot: &InspectorSnapshot, lines: &mut Vec<String>) {
    lines.push(format!(
        "Dependencies — locale={} viewport={}",
        snapshot.locale_readers, snapshot.viewport_readers
    ));
    lines.push("Signals".to_owned());
    for signal in &snapshot.signals {
        lines.push(format!(
            "  {} {}={} rev={} writer={}",
            signal.id, signal.kind, signal.value, signal.revision, signal.last_writer
        ));
    }
    lines.push("Effects".to_owned());
    for effect in &snapshot.effects {
        lines.push(format!(
            "  {} activation={} deps={} start={} cleanup={} gen={}",
            effect.id,
            effect.activation,
            effect.dependencies,
            effect.start,
            effect.cleanup,
            effect.generation
        ));
    }
    lines.push("Timers".to_owned());
    for timer in &snapshot.timers {
        lines.push(format!(
            "  {} remaining={}ms/{}ms paused={}/{} callback={} gen={}",
            timer.id,
            timer.remaining_ms,
            timer.delay_ms,
            timer.declaration_paused,
            timer.interaction_paused,
            timer.callback,
            timer.generation
        ));
    }
    lines.push("Element refs".to_owned());
    for reference in &snapshot.element_refs {
        lines.push(format!("  {} -> node {}", reference.id, reference.node));
    }
    if !snapshot.pointer_captures.is_empty() {
        lines.push(format!("Pointer captures: {:?}", snapshot.pointer_captures));
    }
}

#[cfg(feature = "dev-reload")]
fn count_nodes(node: &InspectorNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

#[cfg(feature = "dev-reload")]
fn append_node_lines(node: &InspectorNode, depth: usize, lines: &mut Vec<String>) {
    let source = node.source.as_ref().map_or_else(String::new, |source| {
        format!(" @ {}:{}:{}", source.module, source.line, source.column)
    });
    lines.push(format!(
        "{}{} {}{}",
        "  ".repeat(depth),
        node.path,
        node.kind,
        source
    ));
    let indent = "  ".repeat(depth.saturating_add(1));
    if let Some(key) = &node.key {
        lines.push(format!("{indent}key={key}"));
    }
    if !node.props.is_empty() {
        lines.push(format!("{indent}props={:?}", node.props));
    }
    if !node.attributes.is_empty() {
        lines.push(format!("{indent}attributes={:?}", node.attributes));
    }
    if !node.handlers.is_empty() {
        lines.push(format!("{indent}handlers={:?}", node.handlers));
    }
    if !node.animations.is_empty() {
        lines.push(format!("{indent}animations={:?}", node.animations));
    }
    lines.push(format!("{indent}computed_style={:?}", node.style));
    for child in &node.children {
        append_node_lines(child, depth.saturating_add(1), lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentInstancePath, ComponentStateSchema, EventPropagation, HostCallback, StateField,
        ValueSchema,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn inspector_reports_host_handler_label_without_opaque_details() {
        let root = UiNode::text("Host").with_host_handler(
            "click",
            HostCallback::new("widget.select", |_, _, _| EventPropagation::Handled),
        );
        let inspected = inspect_node(&root, "root");
        assert_eq!(inspected.handlers, vec!["click:Target=host:widget.select"]);
    }

    #[test]
    fn snapshot_captures_source_and_redacts_sensitive_state() {
        let mut runtime = UiRuntimeState::new();
        let path = ComponentInstancePath::root("Login", "root");
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "token".to_owned(),
            StateField::new(ValueSchema::string(), UiValue::String("secret".to_owned()))
                .sensitive(true),
        )]))
        .unwrap();
        let mut transaction = runtime.component_state.begin_render();
        transaction.mount(path, &schema).unwrap();
        runtime.component_state.commit_render(transaction);
        let root = UiNode::text("Login").with_source(SourceLocation {
            module: "ui/main.rhai".to_owned(),
            line: 4,
            column: 9,
        });
        let engine = crate::RuntimeEngine::new();
        let theme = crate::load_theme_source(
            engine.engine(),
            "theme.rhai",
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        let snapshot = InspectorSnapshot::capture(
            Some(&root),
            &runtime,
            &theme,
            &ComponentRegistry::default(),
            Vec::new(),
        );
        assert_eq!(snapshot.root.unwrap().source.unwrap().line, 4);
        assert!(snapshot.state[0].fields["token"].sensitive);
        assert_eq!(
            display_value(&snapshot.state[0].fields["token"].value, true),
            "<redacted>"
        );
    }

    #[test]
    fn trace_buffer_is_bounded_and_ordered() {
        let mut traces = TraceBuffer::new(2);
        for index in 0..3 {
            traces.push(
                RuntimeTraceKind::Event,
                "root",
                format!("event-{index}"),
                None,
                false,
            );
        }
        let snapshot = traces.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].sequence, 2);
        assert_eq!(snapshot[1].sequence, 3);
    }

    #[test]
    fn snapshot_includes_runtime_mechanism_identity_and_state() {
        let component = ComponentInstancePath::root("View", "main");
        let mut runtime = UiRuntimeState::new();
        let signal_id =
            crate::SignalId::new(component.clone(), "progress", crate::SignalKind::Float).unwrap();
        runtime.signals.reconcile(
            &component,
            BTreeMap::from([(
                signal_id.clone(),
                crate::signal::SignalDescriptor::new(crate::SignalValue::Float(0.25)),
            )]),
        );
        let signal = crate::NativeSignal::new(signal_id);
        runtime
            .signals
            .write_from(
                &signal,
                crate::SignalValue::Float(0.5),
                crate::SignalWriter::Script,
            )
            .unwrap();

        let mut callback = crate::ScriptCallback::try_from_fn_ptr(
            rhai::FnPtr::new("fired").unwrap(),
            crate::ScriptGeneration::initial(),
        )
        .unwrap();
        callback.bind_component_if_unset(component.clone(), BTreeMap::new());
        let timer = crate::TimerDescriptor::new(
            crate::TimerId::new(component.clone(), "refresh").unwrap(),
            Duration::from_secs(1),
            false,
            callback,
            UiValue::Null,
        )
        .unwrap();
        let now = Instant::now();
        runtime.timers.reconcile(
            &component,
            BTreeMap::from([(timer.id().clone(), timer)]),
            now,
        );

        let mut retained = crate::RetainedUiTree::new();
        retained.reconcile(UiNode::text("target")).unwrap();
        let node = retained.root_id().unwrap();
        let reference = crate::ElementRefId::new(component.clone(), "target").unwrap();
        runtime
            .element_refs
            .reconcile(&component, BTreeMap::from([(reference, node)]));
        runtime.pointer_capture.capture(7, node);
        let bounds = crate::GeometryBounds::new(0.0, 0.0, 10.0, 10.0).unwrap();
        runtime.geometry.update(
            node,
            crate::ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            },
        );

        let engine = crate::RuntimeEngine::new();
        let theme = crate::load_theme_source(
            engine.engine(),
            "theme.rhai",
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        let snapshot = InspectorSnapshot::capture(
            retained.root(),
            &runtime,
            &theme,
            &ComponentRegistry::default(),
            Vec::new(),
        );
        assert_eq!(snapshot.signals[0].revision, 1);
        assert_eq!(snapshot.signals[0].last_writer, "Script");
        assert_eq!(snapshot.timers[0].callback, "fired");
        assert_eq!(snapshot.element_refs[0].node, node.get());
        assert_eq!(snapshot.geometry_nodes, 1);
        assert_eq!(snapshot.pointer_captures.get(&7), Some(&node.get()));
    }
}
