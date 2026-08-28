//! Runtime foundations for building GPUI applications from Rhai source.

pub mod component;
pub mod context;
pub mod dependency;
pub mod devtools;
pub mod diagnostic;
pub mod dropdown;
mod dropdown_element;
pub mod engine;
pub mod event;
pub mod lifecycle;
pub mod locale;
pub mod node;
pub mod overlay;
mod overlay_element;
pub mod primitive;
pub mod reload;
pub mod renderer;
pub mod responsive;
pub mod schema;
pub mod script_source;
pub mod source;
pub mod state;
pub mod store;
pub mod style;
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
    EmbeddedScriptApp, PreparedScriptApp, ScriptApp, ScriptAppError, ScriptAppExtension,
};
pub use asset::{
    AssetData, AssetError, AssetId, AssetProvider, AssetRegistry, DirectoryAssetProvider,
    ImageDecodeHandle, ImageHandle, InMemoryAssetProvider,
};
pub use async_runtime::{
    AsyncDelivery, AsyncRuntimeError, AsyncScope, SubscriptionEmitter, SubscriptionHandle,
    SubscriptionRegistry, TaskHandle, TaskRegistry,
};
pub use capability::{
    AppManifest, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityError, CapabilityHandler,
    CapabilityId, CapabilityMethod, CapabilityRegistry, SubscriptionCapabilityHandler,
    SubscriptionWork, TaskWork,
};
pub use component::{
    ComponentDefinition, ComponentError, ComponentExportCollector, ComponentExportError,
    ComponentHeaderError, ComponentInvocation, ComponentMetadata, ComponentRegistry,
    ComponentRegistryError, ComponentSchema, EventSchema, RuntimeApiRange, SlotSchema,
    parse_component_header,
};
pub use context::{
    ExecutionPhase, PendingEvent, UiContext, UiContextError, UiMutationBatch, UiRuntimeState,
    UiStateSnapshot,
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
    DropdownError, DropdownKey, DropdownMode, DropdownNodeSpec, DropdownOption, DropdownOutcome,
    DropdownState,
};
pub use engine::{
    CompiledUi, ExecutionOperation, ExecutionTiming, RuntimeEngine, RuntimeError, ScriptCallback,
    ScriptGeneration,
};
pub use event::{EventDispatchReport, EventPropagation, EventRouter, UiEvent};
pub use lifecycle::{LifecycleError, LifecycleState, ScriptLifecycle};
pub use locale::{LocaleBundle, LocaleError, LocaleManager, TextDirection, load_locale_source};
pub use node::{
    NodeKey, OverlayDismissPolicy, OverlayNodeSpec, SourceLocation, TooltipDelays, UiNode,
    UiNodeKind,
};
pub use overlay::{
    DismissReport, FocusToken, OverlayBounds, OverlayError, OverlayId, OverlayKind, OverlayManager,
    OverlayPlacement, OverlaySpec, PlacementResult, ToastError, ToastQueue, ToastRegion,
    TooltipScheduler, TooltipTransition,
};
pub use primitive::{
    PrimitiveDescriptor, PrimitiveError, PrimitiveEventEmitter, PrimitiveHandler, PrimitiveId,
    PrimitiveInstance, PrimitiveInstanceId, PrimitiveNode, PrimitiveProps, PrimitiveRegistry,
    PrimitiveValue,
};
pub use reload::{LiveScript, ReloadOutcome};
pub use renderer::{
    ColorResolver, GpuiNodeRenderer, LiteralColorResolver, NodeEventDispatcher, ScriptView,
};
pub use responsive::{ResponsiveError, ResponsiveRuntime, ViewportBreakpoints, ViewportClass};
pub use schema::{ObjectField, SchemaIssue, SchemaValidationError, ValueSchema};
pub use script_source::{
    EmbeddedScriptSource, FileScriptSource, ScriptAsset, ScriptSource, ScriptSourceError,
};
pub use source::{ModuleId, ModuleIdError, RestrictedModuleResolver};
pub use state::{
    ComponentInstancePath, ComponentStateSchema, RenderStateTransaction, StateError, StateField,
    StateInstanceSnapshot, StateReconcileReport, StateStore, StateValueSnapshot,
};
pub use store::{StoreError, StoreId, StoreReadSession, StoreRegistry, StoreScope, StoreSnapshot};
pub use style::{
    Align, ColorValue, EdgeLengths, FlexDirection, InteractionState, Justify, Length, LengthError,
    PseudoState, Rgba8, Style, StyleProperties,
};
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
pub use window::{ScriptWindowSpec, WindowCommand, WindowCommandError, WindowCommandRegistry};

/// The first runtime API generation understood by component source.
pub const RUNTIME_API_VERSION: u32 = 1;
