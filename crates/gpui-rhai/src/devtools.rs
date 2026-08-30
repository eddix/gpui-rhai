use std::collections::{BTreeMap, VecDeque};

#[cfg(feature = "dev-reload")]
use gpui::{
    AnyElement, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    deferred, div, px, rgba,
};

use crate::{
    ComponentRegistry, ExecutionTiming, PrimitiveValue, SourceLocation, StateInstanceSnapshot,
    StoreSnapshot, StyleProperties, ThemeVariant, ToastHostSpec, UiNode, UiNodeKind,
    UiRuntimeState, UiValue,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeTraceKind {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorComponent {
    pub id: String,
    pub export: String,
    pub props: BTreeMap<String, String>,
    pub slots: Vec<String>,
    pub parts: Vec<String>,
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
        }
    }
}

fn inspect_node(node: &UiNode, path: &str) -> InspectorNode {
    let mut props = BTreeMap::new();
    let (kind, descendants): (String, Vec<&UiNode>) = match node.kind() {
        UiNodeKind::Text { text } => {
            props.insert("text".to_owned(), truncate(text, 80));
            ("text".to_owned(), Vec::new())
        }
        UiNodeKind::Container { children } => ("container".to_owned(), children.iter().collect()),
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
        UiNodeKind::Dropdown { spec } => inspect_dropdown(spec, &mut props),
        UiNodeKind::Select { spec } => inspect_select(spec, &mut props),
        UiNodeKind::DatePicker { spec } => inspect_date_picker(spec, &mut props),
        UiNodeKind::Table { spec } => inspect_table(spec, &mut props),
        UiNodeKind::ToastHost { spec } => inspect_toast_host(spec, &mut props),
        UiNodeKind::VirtualList { spec } => inspect_virtual_list(spec, &mut props),
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

fn inspect_virtual_list<'a>(
    spec: &'a crate::VirtualListNodeSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("items".to_owned(), spec.items.len().to_string());
    props.insert("row_height".to_owned(), spec.row_height.to_string());
    (
        "virtual_list".to_owned(),
        spec.items.iter().map(|item| &item.node).collect(),
    )
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

fn inspect_select<'a>(
    spec: &crate::SelectNodeSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("id".to_owned(), spec.choice.id.clone());
    props.insert("options".to_owned(), spec.choice.options.len().to_string());
    props.insert(
        "value".to_owned(),
        spec.choice
            .selected
            .as_ref()
            .and_then(|values| values.first())
            .cloned()
            .unwrap_or_else(|| "<null>".to_owned()),
    );
    ("select".to_owned(), Vec::new())
}

fn inspect_dropdown<'a>(
    spec: &'a crate::DropdownNodeSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("id".to_owned(), spec.id.clone());
    props.insert("options".to_owned(), spec.options.len().to_string());
    props.insert("mode".to_owned(), format!("{:?}", spec.mode));
    props.insert(
        "selected".to_owned(),
        spec.selected
            .as_ref()
            .map_or_else(|| "<uncontrolled>".to_owned(), |value| format!("{value:?}")),
    );
    props.insert(
        "open".to_owned(),
        spec.open
            .map_or_else(|| "<uncontrolled>".to_owned(), |value| value.to_string()),
    );
    let slots = [
        spec.trigger_slot.as_deref(),
        spec.header_slot.as_deref(),
        spec.footer_slot.as_deref(),
        spec.empty_slot.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    ("dropdown".to_owned(), slots)
}

fn inspect_date_picker<'a>(
    spec: &crate::DatePickerNodeSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("id".to_owned(), spec.id.clone());
    props.insert(
        "value".to_owned(),
        spec.value
            .map_or_else(|| "<null>".to_owned(), crate::GregorianDate::to_iso),
    );
    ("date_picker".to_owned(), Vec::new())
}

fn inspect_table<'a>(
    spec: &'a crate::TableNodeSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("key".to_owned(), spec.key.clone());
    props.insert("rows".to_owned(), spec.rows.len().to_string());
    props.insert("columns".to_owned(), spec.columns.len().to_string());
    let descendants = spec
        .columns
        .iter()
        .flat_map(|column| column.custom_cells.iter().flatten())
        .chain(
            [spec.loading_slot.as_deref(), spec.empty_slot.as_deref()]
                .into_iter()
                .flatten(),
        )
        .chain(spec.loading_rows.iter())
        .collect();
    ("table".to_owned(), descendants)
}

fn inspect_image_source(source: &crate::ImageSourceSpec) -> String {
    match source {
        crate::ImageSourceSpec::Handle(handle) => format!("{}#<opaque>", handle.kind()),
        crate::ImageSourceSpec::Asset(asset) => asset.as_str().to_owned(),
    }
}

fn inspect_toast_host<'a>(
    spec: &'a ToastHostSpec,
    props: &mut BTreeMap<String, String>,
) -> (String, Vec<&'a UiNode>) {
    props.insert("key".to_owned(), spec.key.clone());
    props.insert("items".to_owned(), spec.items.len().to_string());
    props.insert("max_visible".to_owned(), spec.max_visible.to_string());
    ("toast_host".to_owned(), Vec::new())
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
    let mut lines = vec![
        format!(
            "GPUI Rhai Inspector — {} / {}",
            snapshot.theme_family, snapshot.theme_name
        ),
        format!(
            "nodes={} state={} stores={} dirty={} traces={} timings={}",
            snapshot.root.as_ref().map_or(0, count_nodes),
            snapshot.state.len(),
            snapshot.stores.len(),
            snapshot.dirty.len(),
            snapshot.traces.len(),
            snapshot.timings.len()
        ),
    ];
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
            "  {:?} {} {:.2?}{}",
            timing.operation,
            timing.source,
            timing.duration,
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
}
