//! Runtime foundations for building GPUI applications from Rhai source.

pub mod component;
pub mod context;
pub mod date;
pub mod date_picker;
mod date_picker_element;
pub mod dependency;
pub mod devtools;
pub mod diagnostic;
pub mod dropdown;
mod dropdown_element;
pub mod effect;
pub mod element_ref;
pub mod engine;
pub mod event;
pub mod geometry;
mod invocation;
pub mod lifecycle;
pub mod locale;
pub mod native_handler;
pub mod node;
pub mod overlay;
mod overlay_element;
pub mod primitive;
pub mod reload;
pub mod renderer;
pub mod responsive;
pub mod retained;
pub mod schema;
pub mod script_source;
pub mod signal;
pub mod source;
pub mod state;
pub mod store;
pub mod style;
pub mod table;
mod table_element;
pub mod text_area;
mod text_edit;
pub mod text_input;
pub mod theme;
pub mod toast;
mod toast_element;
pub mod value;
pub mod virtual_list;
mod virtual_list_element;
pub mod window;

pub mod action;
pub mod animation;
pub mod app;
pub mod asset;
pub mod async_runtime;
pub mod budget;
pub mod capability;

pub use gpui;

pub use action::{
    ActionError, ActionId, ActionInvocation, ActionRegistry, DispatchScriptAction, KeyBindingSpec,
};
pub use animation::{
    AnimationError, AnimationFrame, AnimationKey, AnimationProperty, AnimationRuntime,
    AnimationSpec, Easing, MotionPreference, SpringSpec, TransitionSpec,
};
pub use app::{
    EmbeddedScriptView, FileScriptView, PreparedScriptView, ScriptApplication, ScriptViewConfig,
    ScriptViewError, ScriptViewExtension, ScriptViewHandle, ScriptViewHost, install,
};
pub use asset::{
    AssetData, AssetError, AssetId, AssetProvider, AssetRegistry, DirectoryAssetProvider,
    ImageDecodeHandle, ImageHandle, InMemoryAssetProvider,
};
pub use async_runtime::{
    AsyncDelivery, AsyncRuntimeError, AsyncScope, SubscriptionCloseReason, SubscriptionEmitter,
    SubscriptionHandle, SubscriptionRegistration, SubscriptionRegistry, TaskHandle, TaskRegistry,
};
pub use budget::{RuntimeBudgetError, RuntimeBudgets};
pub use capability::{
    AppManifest, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityError, CapabilityHandler,
    CapabilityId, CapabilityMethod, CapabilityRegistry, SubscriptionCapabilityHandler,
    SubscriptionWork, TaskWork,
};
pub use component::{
    ComponentDefinition, ComponentError, ComponentExportCollector, ComponentExportError,
    ComponentHeaderError, ComponentInvocation, ComponentMetadata, ComponentPropConversionError,
    ComponentPropValue, ComponentProps, ComponentRegistry, ComponentRegistryError, ComponentSchema,
    EventSchema, RuntimeApiRange, SlotSchema, parse_component_header,
};
pub use context::{
    ExecutionPhase, PendingEvent, UiContext, UiContextError, UiMutationBatch, UiRuntimeState,
    UiStateSnapshot,
};
pub use date::{CalendarClock, CalendarClockSource, DateError, GregorianDate, Weekday};
pub use date_picker::{
    DatePickerCell, DatePickerCellState, DatePickerKey, DatePickerNodeSpec, DatePickerOutcome,
    DatePickerPreset, DatePickerState,
};
pub use dependency::{
    DependencyError, ModuleCompileCache, ModuleDependencyGraph, ModuleRefreshReport,
    extract_imports,
};
#[cfg(feature = "dev-reload")]
pub use dependency::{FileChangeBatch, FileWatcher, WatcherError};
pub use devtools::{
    InspectorComponent, InspectorNode, InspectorSnapshot, RuntimeTrace, RuntimeTraceKind,
    TraceBuffer,
};
pub use diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticContext, DiagnosticFrame, DiagnosticSeverity,
};
pub use dropdown::{
    ChoiceBehavior, DropdownError, DropdownKey, DropdownMode, DropdownNodeSpec, DropdownOption,
    DropdownOutcome, DropdownState, DropdownVisibleRow, SelectNodeSpec,
};
pub use effect::{EffectDescriptor, EffectError, EffectId, EffectRegistry};
pub use element_ref::{ElementRef, ElementRefError, ElementRefId, ElementRefRegistry};
pub use engine::{
    CompiledUi, ComponentInvocationRecipe, ExecutionOperation, ExecutionTiming, RuntimeEngine,
    RuntimeError, ScriptCallback, ScriptCallbackDefinitionError, ScriptGeneration,
};
pub use event::{
    EventDispatchReport, EventModifiers, EventPhase, EventPropagation, EventResponse, EventRouter,
    HostCallback, LogicalPoint, PointerCaptureDirective, PointerCaptureRegistry, PointerEventData,
    PropagationControl, UiEvent, UiEventBinding, UiEventHandler, WheelEventData,
};
pub use geometry::{ElementGeometry, GeometryBounds, GeometryError, GeometryRegistry};
pub use lifecycle::{LifecycleError, LifecycleState, ScriptLifecycle};
pub use locale::{
    CalendarMetadata, CalendarNames, DatePatterns, DateStyle, LocaleBundle, LocaleError,
    LocaleManager, NumberFormatOptions, NumberMetadata, TextDirection, format_date_with_metadata,
    format_integer_with_metadata, format_number_with_metadata, load_locale_source,
};
pub use native_handler::{
    NativeEvent, NativeHandlerDescriptor, NativeHandlerError, NativeHandlerId, NativeHandlerRef,
    NativeHandlerRegistry,
};
pub use node::{
    ImageSourceSpec, NodeKey, OverlayDismissPolicy, OverlayNodeSpec, SourceLocation, Span,
    TooltipDelays, UiNode, UiNodeKind, UiNodeKindTag,
};
pub use overlay::{
    DismissReport, FocusToken, OverlayBounds, OverlayError, OverlayId, OverlayKind, OverlayManager,
    OverlayPlacement, OverlaySpec, PlacementResult, ToastError, ToastQueue, ToastRegion,
    TooltipScheduler, TooltipTransition,
};
pub use primitive::{
    PrimitiveDescriptor, PrimitiveError, PrimitiveEventEmitter, PrimitiveHandler, PrimitiveId,
    PrimitiveInstance, PrimitiveInstanceId, PrimitiveNode, PrimitiveProps, PrimitiveRegistry,
    PrimitiveTheme, PrimitiveValue,
};
pub use reload::{LiveScript, ReloadOutcome};
pub use renderer::{
    ColorResolver, GpuiNodeRenderer, LiteralColorResolver, NodeEventDispatcher, StaticUiView,
};
pub use responsive::{ResponsiveError, ResponsiveRuntime, ViewportBreakpoints, ViewportClass};
pub use retained::{
    NodeId, ReconcileError, ReconcileReport, RetainedChildLink, RetainedNode, RetainedUiTree,
};
pub use schema::{
    ObjectField, SchemaDefinitionError, SchemaIssue, SchemaValidationError, ValueSchema,
};
pub use script_source::{
    EmbeddedScriptSource, FileScriptSource, ScriptAsset, ScriptSource, ScriptSourceError,
};
pub use signal::{
    NativeSignal, SignalError, SignalId, SignalKind, SignalProperty, SignalRegistry, SignalValue,
};
pub use source::{ModuleId, ModuleIdError, RestrictedModuleResolver};
pub use state::{
    ComponentInstancePath, ComponentStateSchema, RenderStateTransaction, StateError, StateField,
    StateInstanceSnapshot, StateReconcileReport, StateStore, StateValueSnapshot,
};
pub use store::{StoreError, StoreId, StoreReadSession, StoreRegistry, StoreScope, StoreSnapshot};
pub use style::{
    Align, ColorValue, EdgeLengths, FlexDirection, InteractionState, Justify, Length, LengthError,
    OverflowMode, PseudoState, RadiusToken, Rgba8, SpacingToken, Style, StyleProperties,
};
pub use table::{
    TableAlign, TableCellFormat, TableColumnSpec, TableColumnWidth, TableError, TableLayout,
    TableNodeSpec, TableRowSpec, TableSelectionMode, TableSort, TableSortDirection, TableState,
};
pub use text_area::{TextAreaPrimitiveHandler, init_text_area, text_area_primitive_descriptor};
pub use text_input::{
    TextBuffer, TextInputPrimitiveHandler, init_text_input, text_input_primitive_descriptor,
};
pub use theme::{
    ResolvedTheme, SystemAppearance, ThemeError, ThemeFamily, ThemeManager, ThemeMode,
    ThemePreference, ThemeSelection, ThemeTokens, ThemeVariant, load_theme_source,
};
pub use toast::{ToastHostSpec, ToastItemSpec, ToastVariant};
pub use value::{OpaqueHandle, UiValue, UiValueError};
pub use virtual_list::{
    VirtualListError, VirtualListItem, VirtualListMetrics, VirtualListNodeSpec, VirtualListSpec,
    VirtualListState,
};
pub use window::{
    ScriptWindowSpec, WindowCommand, WindowCommandError, WindowCommandPolicy, WindowCommandRegistry,
};

/// The first runtime API generation understood by component source.
pub const RUNTIME_API_VERSION: u32 = 1;
