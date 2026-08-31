//! Runtime foundations for building GPUI applications from Rhai source.

pub mod component;
pub mod context;
pub mod date;
pub mod dependency;
pub mod devtools;
pub mod diagnostic;
pub mod effect;
pub mod element_ref;
pub mod engine;
mod environment_dependency;
pub mod event;
pub mod font;
pub mod geometry;
pub mod inline_svg;
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
mod script_lint;
pub mod script_source;
pub mod signal;
mod slot_runtime;
pub mod source;
pub mod state;
pub mod store;
pub mod style;
pub mod text_area;
mod text_edit;
pub mod text_input;
pub mod theme;
pub mod timer;
pub mod value;
pub mod virtual_list;
mod virtual_list_element;
pub mod window;

pub mod accessibility;
pub mod action;
pub mod animation;
pub mod app;
pub mod asset;
pub mod async_runtime;
pub mod automation;
pub mod budget;
pub mod canvas;
pub mod capability;
pub mod clock;

pub use gpui;

pub use accessibility::{AccessibilityError, AccessibilityNode, AccessibilityTree};
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
pub use automation::{
    AutomationBounds, AutomationCommand, AutomationDispatchReport, AutomationError,
    AutomationLocator, AutomationNode, AutomationRequest, AutomationResponse, AutomationResult,
    AutomationSnapshot, handle_automation_json_line, run_automation_json_lines,
};
pub use budget::{RuntimeBudgetError, RuntimeBudgets};
pub use canvas::{
    CanvasClipRect, CanvasCommand, CanvasError, CanvasFill, CanvasPathSegment, CanvasScene,
    CanvasTransform,
};
pub use capability::{
    AppManifest, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityError, CapabilityHandler,
    CapabilityId, CapabilityMethod, CapabilityRegistry, SubscriptionCapabilityHandler,
    SubscriptionWork, TaskWork,
};
pub use clock::{ManualRuntimeClock, RuntimeClock, RuntimeClockSource};
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
pub use dependency::{
    DependencyError, ModuleCompileCache, ModuleDependencyGraph, ModuleRefreshReport,
    extract_imports,
};
#[cfg(feature = "dev-reload")]
pub use dependency::{FileChangeBatch, FileWatcher, WatcherError};
pub use devtools::{
    InspectorComponent, InspectorEffect, InspectorElementRef, InspectorNode, InspectorSignal,
    InspectorSnapshot, InspectorTimer, RuntimeTrace, RuntimeTraceKind, TraceBuffer,
};
pub use diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticContext, DiagnosticFrame, DiagnosticSeverity,
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
pub use font::{FontError, FontSource, validate_font_sources};
pub use geometry::{ElementGeometry, GeometryBounds, GeometryError, GeometryRegistry};
pub use inline_svg::{InlineSvg, InlineSvgError};
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
    ImageSourceSpec, LayerNodeSpec, LayerPlacement, NodeKey, OverlayDismissPolicy, OverlayNodeSpec,
    SourceLocation, Span, TooltipDelays, UiNode, UiNodeKind, UiNodeKindTag,
};
pub use overlay::{
    DismissReport, FocusToken, OverlayBounds, OverlayError, OverlayId, OverlayKind, OverlayManager,
    OverlayPlacement, OverlaySpec, PlacementResult, TooltipScheduler, TooltipTransition,
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
    NodeId, ReconcileError, ReconcileMetrics, ReconcileReport, RetainedChildLink, RetainedNode,
    RetainedUiTree,
};
pub use schema::{
    ObjectField, SchemaDefinitionError, SchemaIssue, SchemaValidationError, ValueSchema,
};
pub use script_lint::{KnownCallDiagnostic, KnownCallLintError};
pub use script_source::{
    EmbeddedScriptSource, FileScriptSource, ScriptAsset, ScriptSource, ScriptSourceError,
};
pub use signal::{
    NativeSignal, SignalError, SignalId, SignalKind, SignalProperty, SignalRegistry,
    SignalSnapshot, SignalValue, SignalWriter,
};
pub use source::{ModuleId, ModuleIdError, RestrictedModuleResolver};
pub use state::{
    ComponentInstancePath, ComponentStateSchema, RenderStateTransaction, StateError, StateField,
    StateInstanceSnapshot, StateReconcileReport, StateStore, StateValueSnapshot,
};
pub use store::{StoreError, StoreId, StoreReadSession, StoreRegistry, StoreScope, StoreSnapshot};
pub use style::{
    Align, ColorParseError, ColorValue, CornerLengths, CursorKind, DisplayMode, EdgeLengths,
    FlexDirection, FlexWrapMode, FontSlant, HitTestBehavior, InteractionState, Justify, Length,
    LengthError, LinearGradientSpec, OverflowMode, PositionMode, PseudoState, RadiusToken, Rgba8,
    ShadowSpec, SpacingToken, Style, StyleProperties, StyleValueError, TextAlignMode,
    WhiteSpaceMode,
};
pub use text_area::{TextAreaPrimitiveHandler, init_text_area, text_area_primitive_descriptor};
pub use text_input::{
    TextBuffer, TextInputPrimitiveHandler, init_text_input, text_input_primitive_descriptor,
};
pub use theme::{
    ResolvedTheme, SystemAppearance, ThemeError, ThemeFamily, ThemeManager, ThemeMode,
    ThemePreference, ThemeSelection, ThemeTokenValue, ThemeTokens, ThemeVariant, load_theme_source,
};
pub use timer::{TimerDescriptor, TimerError, TimerId, TimerRegistry, TimerSnapshot};
pub use value::{OpaqueHandle, UiValue, UiValueError};
pub use virtual_list::{
    VariableListSpec, VariableListState, VariableListWindow, VirtualCollectionId,
    VirtualCollectionNodeSpec, VirtualListError, VirtualListMetrics, VirtualListSpec,
    VirtualListState, VirtualRequestRegistry,
};
pub use window::{
    ScriptWindowSpec, WindowCommand, WindowCommandError, WindowCommandPolicy, WindowCommandRegistry,
};

/// The first runtime API generation understood by component source.
pub const RUNTIME_API_VERSION: u32 = 1;
