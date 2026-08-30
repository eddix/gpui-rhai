use std::collections::BTreeMap;

use gpui::AnyElement;

use crate::overlay_element::WindowOverlayCoordinator;
use crate::renderer::{GpuiNodeRenderer, OwnedColorResolver, WindowRenderResources};
use crate::{
    AnimationKey, AssetRegistry, InteractionState, NodeEventDispatcher, PrimitiveRegistry, UiNode,
};

#[derive(Clone)]
pub(crate) struct NodeSlotRuntime {
    pub colors: OwnedColorResolver,
    pub primitives: PrimitiveRegistry,
    pub assets: AssetRegistry,
    pub dispatcher: NodeEventDispatcher,
    pub overlays: WindowOverlayCoordinator,
    pub animations: BTreeMap<AnimationKey, f64>,
    pub signals: crate::SignalRegistry,
    pub geometry: crate::GeometryRegistry,
    pub pointer_capture: crate::PointerCaptureRegistry,
    pub focus_handles: BTreeMap<crate::NodeId, gpui::FocusHandle>,
    pub scroll_handles: BTreeMap<crate::NodeId, gpui::ScrollHandle>,
    pub scroll_anchors: BTreeMap<crate::NodeId, gpui::ScrollAnchor>,
    pub virtual_requests: crate::VirtualRequestRegistry,
    pub direction: crate::TextDirection,
    pub base_path: String,
    pub view_id: String,
}

impl NodeSlotRuntime {
    pub(crate) fn render(&self, node: &UiNode, slot: &str) -> AnyElement {
        let resources = WindowRenderResources {
            assets: &self.assets,
            dispatcher: &self.dispatcher,
            overlays: &self.overlays,
            animations: &self.animations,
            signals: &self.signals,
            geometry: &self.geometry,
            pointer_capture: &self.pointer_capture,
            focus_handles: &self.focus_handles,
            scroll_handles: &self.scroll_handles,
            scroll_anchors: &self.scroll_anchors,
            virtual_requests: &self.virtual_requests,
            direction: self.direction,
            root_path: &self.base_path,
            view_id: &self.view_id,
        };
        GpuiNodeRenderer::render_subtree_with_window_runtime(
            node,
            &self.colors,
            &InteractionState::default(),
            &self.primitives,
            &resources,
            &format!("{}/{slot}", self.base_path),
        )
    }
}
