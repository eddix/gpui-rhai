use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use thiserror::Error;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OverlayId(String);

impl OverlayId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlayKind {
    Popover,
    Combobox,
    Tooltip,
    Dialog,
    Sheet,
    Menu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlayPlacement {
    Top,
    Bottom,
    Left,
    Right,
    Start,
    End,
    Center,
}

impl OverlayPlacement {
    const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Start => Self::End,
            Self::End => Self::Start,
            Self::Center => Self::Center,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OverlayBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl OverlayBounds {
    #[must_use]
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    fn clamp_to(self, viewport: Self) -> Self {
        let max_x = (viewport.x + viewport.width - self.width).max(viewport.x);
        let max_y = (viewport.y + viewport.height - self.height).max(viewport.y);
        Self {
            x: self.x.clamp(viewport.x, max_x),
            y: self.y.clamp(viewport.y, max_y),
            width: self.width.min(viewport.width),
            height: self.height.min(viewport.height),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusToken(pub String);

#[derive(Clone, Debug)]
pub struct OverlaySpec {
    pub id: OverlayId,
    pub parent: Option<OverlayId>,
    pub kind: OverlayKind,
    pub anchor: OverlayBounds,
    pub width: f64,
    pub height: f64,
    pub preferred: OverlayPlacement,
    pub gap: f64,
    pub modal: bool,
    pub dismiss_on_escape: bool,
    pub dismiss_on_outside: bool,
    pub restore_focus: Option<FocusToken>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementResult {
    pub bounds: OverlayBounds,
    pub placement: OverlayPlacement,
    pub flipped: bool,
}

#[derive(Clone, Debug)]
struct OverlayEntry {
    spec: OverlaySpec,
    placement: PlacementResult,
    z_index: u64,
}

#[derive(Clone, Debug)]
pub struct OverlayManager {
    viewport: OverlayBounds,
    entries: BTreeMap<OverlayId, OverlayEntry>,
    order: Vec<OverlayId>,
    next_z: u64,
    tooltips: TooltipScheduler,
}

impl OverlayManager {
    /// Create a per-window manager with a valid viewport.
    ///
    /// # Errors
    ///
    /// Returns [`OverlayError::InvalidGeometry`] for invalid bounds.
    pub fn new(viewport: OverlayBounds) -> Result<Self, OverlayError> {
        validate_bounds(viewport)?;
        Ok(Self {
            viewport,
            entries: BTreeMap::new(),
            order: Vec::new(),
            next_z: 1,
            tooltips: TooltipScheduler::default(),
        })
    }

    /// Start a new render frame while preserving tooltip and toast state.
    ///
    /// # Errors
    ///
    /// Returns [`OverlayError::InvalidGeometry`] for invalid viewport bounds.
    pub fn begin_frame(&mut self, viewport: OverlayBounds) -> Result<(), OverlayError> {
        validate_bounds(viewport)?;
        self.viewport = viewport;
        self.entries.clear();
        self.order.clear();
        self.next_z = 1;
        Ok(())
    }

    /// Open and place an overlay above its optional parent.
    ///
    /// # Errors
    ///
    /// Returns duplicate, missing-parent, cycle, or geometry errors.
    pub fn open(&mut self, spec: OverlaySpec) -> Result<PlacementResult, OverlayError> {
        if self.entries.contains_key(&spec.id) {
            return Err(OverlayError::Duplicate(spec.id));
        }
        validate_bounds(spec.anchor)?;
        validate_size(spec.width, spec.height, spec.gap)?;
        if let Some(parent) = &spec.parent {
            if parent == &spec.id {
                return Err(OverlayError::ParentCycle(spec.id));
            }
            if !self.entries.contains_key(parent) {
                return Err(OverlayError::MissingParent(parent.clone()));
            }
        }
        let placement = place(&spec, self.viewport);
        let id = spec.id.clone();
        self.entries.insert(
            id.clone(),
            OverlayEntry {
                spec,
                placement,
                z_index: self.next_z,
            },
        );
        self.next_z = self.next_z.saturating_add(1);
        self.order.push(id);
        Ok(placement)
    }

    /// Update window bounds and recompute every placement.
    ///
    /// # Errors
    ///
    /// Returns [`OverlayError::InvalidGeometry`] for invalid viewport bounds.
    pub fn set_viewport(&mut self, viewport: OverlayBounds) -> Result<(), OverlayError> {
        validate_bounds(viewport)?;
        self.viewport = viewport;
        for entry in self.entries.values_mut() {
            entry.placement = place(&entry.spec, viewport);
        }
        Ok(())
    }

    #[must_use]
    pub fn placement(&self, id: &OverlayId) -> Option<PlacementResult> {
        self.entries.get(id).map(|entry| entry.placement)
    }

    #[must_use]
    pub fn z_index(&self, id: &OverlayId) -> Option<u64> {
        self.entries.get(id).map(|entry| entry.z_index)
    }

    #[must_use]
    pub fn dismiss(&mut self, id: &OverlayId) -> DismissReport {
        let mut removed = BTreeSet::new();
        self.collect_descendants(id, &mut removed);
        removed.insert(id.clone());
        let mut dismissed = self
            .order
            .iter()
            .filter(|candidate| removed.contains(*candidate))
            .cloned()
            .collect::<Vec<_>>();
        dismissed.reverse();
        let restore_focus = dismissed.iter().find_map(|dismissed| {
            self.entries
                .get(dismissed)
                .and_then(|entry| entry.spec.restore_focus.clone())
        });
        for dismissed in &dismissed {
            self.entries.remove(dismissed);
        }
        self.order.retain(|candidate| !removed.contains(candidate));
        DismissReport {
            dismissed,
            restore_focus,
        }
    }

    #[must_use]
    pub fn handle_escape(&mut self) -> DismissReport {
        let candidate = self.order.iter().rev().find(|id| {
            self.entries
                .get(*id)
                .is_some_and(|entry| entry.spec.dismiss_on_escape)
        });
        candidate
            .cloned()
            .map_or_else(DismissReport::default, |id| self.dismiss(&id))
    }

    #[must_use]
    pub fn handle_outside_click(&mut self, x: f64, y: f64) -> DismissReport {
        let containing = self.order.iter().rev().find(|id| {
            self.entries.get(*id).is_some_and(|entry| {
                entry.placement.bounds.contains(x, y) || entry.spec.anchor.contains(x, y)
            })
        });
        if let Some(containing) = containing {
            let ancestors = self.ancestors(containing);
            let candidates = self
                .order
                .iter()
                .rev()
                .filter(|id| !ancestors.contains(*id))
                .filter(|id| {
                    self.entries
                        .get(*id)
                        .is_some_and(|entry| entry.spec.dismiss_on_outside && !entry.spec.modal)
                })
                .cloned()
                .collect::<Vec<_>>();
            return candidates
                .first()
                .map_or_else(DismissReport::default, |id| self.dismiss(id));
        }
        let candidate = self.order.iter().rev().find(|id| {
            self.entries
                .get(*id)
                .is_some_and(|entry| entry.spec.dismiss_on_outside || entry.spec.modal)
        });
        candidate
            .cloned()
            .map_or_else(DismissReport::default, |id| {
                if self
                    .entries
                    .get(&id)
                    .is_some_and(|entry| entry.spec.dismiss_on_outside)
                {
                    self.dismiss(&id)
                } else {
                    DismissReport::default()
                }
            })
    }

    #[must_use]
    pub fn tooltips(&self) -> &TooltipScheduler {
        &self.tooltips
    }

    pub fn tooltips_mut(&mut self) -> &mut TooltipScheduler {
        &mut self.tooltips
    }

    fn collect_descendants(&self, id: &OverlayId, output: &mut BTreeSet<OverlayId>) {
        for (candidate, entry) in &self.entries {
            if entry.spec.parent.as_ref() == Some(id) && output.insert(candidate.clone()) {
                self.collect_descendants(candidate, output);
            }
        }
    }

    fn ancestors(&self, id: &OverlayId) -> BTreeSet<OverlayId> {
        let mut ancestors = BTreeSet::from([id.clone()]);
        let mut current = self
            .entries
            .get(id)
            .and_then(|entry| entry.spec.parent.clone());
        while let Some(parent) = current {
            ancestors.insert(parent.clone());
            current = self
                .entries
                .get(&parent)
                .and_then(|entry| entry.spec.parent.clone());
        }
        ancestors
    }
}

#[derive(Clone, Debug, Default)]
pub struct TooltipScheduler {
    visible: Option<OverlayId>,
    pending_show: Option<(OverlayId, Instant)>,
    pending_hide: Option<(OverlayId, Instant)>,
}

impl TooltipScheduler {
    pub fn pointer_enter(&mut self, id: OverlayId, now: Instant, delay: Duration) {
        self.pending_hide = None;
        if self.visible.as_ref() == Some(&id) {
            self.pending_show = None;
        } else {
            self.pending_show = Some((id, now + delay));
        }
    }

    pub fn pointer_leave(&mut self, id: &OverlayId, now: Instant, delay: Duration) {
        if self
            .pending_show
            .as_ref()
            .is_some_and(|(pending, _)| pending == id)
        {
            self.pending_show = None;
        }
        if self.visible.as_ref() == Some(id) {
            self.pending_hide = Some((id.clone(), now + delay));
        }
    }

    #[must_use]
    pub fn tick(&mut self, now: Instant) -> TooltipTransition {
        let hidden = self
            .pending_hide
            .as_ref()
            .filter(|(_, deadline)| now >= *deadline)
            .map(|(id, _)| id.clone());
        if let Some(hidden) = &hidden {
            if self.visible.as_ref() == Some(hidden) {
                self.visible = None;
            }
            self.pending_hide = None;
        }
        let shown = self
            .pending_show
            .as_ref()
            .filter(|(_, deadline)| now >= *deadline)
            .map(|(id, _)| id.clone());
        if let Some(shown) = &shown {
            self.visible = Some(shown.clone());
            self.pending_show = None;
        }
        TooltipTransition { shown, hidden }
    }

    #[must_use]
    pub fn visible(&self) -> Option<&OverlayId> {
        self.visible.as_ref()
    }

    pub fn remove(&mut self, id: &OverlayId) -> bool {
        let mut removed = false;
        if self.visible.as_ref() == Some(id) {
            self.visible = None;
            removed = true;
        }
        if self
            .pending_show
            .as_ref()
            .is_some_and(|(pending, _)| pending == id)
        {
            self.pending_show = None;
            removed = true;
        }
        if self
            .pending_hide
            .as_ref()
            .is_some_and(|(pending, _)| pending == id)
        {
            self.pending_hide = None;
            removed = true;
        }
        removed
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TooltipTransition {
    pub shown: Option<OverlayId>,
    pub hidden: Option<OverlayId>,
}

fn place(spec: &OverlaySpec, viewport: OverlayBounds) -> PlacementResult {
    let preferred = candidate_bounds(spec, spec.preferred, viewport);
    if fits_primary_axis(preferred, viewport, spec.preferred) {
        return PlacementResult {
            bounds: preferred.clamp_to(viewport),
            placement: spec.preferred,
            flipped: false,
        };
    }
    let opposite = spec.preferred.opposite();
    let flipped = candidate_bounds(spec, opposite, viewport);
    if fits_primary_axis(flipped, viewport, opposite) {
        PlacementResult {
            bounds: flipped.clamp_to(viewport),
            placement: opposite,
            flipped: true,
        }
    } else {
        PlacementResult {
            bounds: preferred.clamp_to(viewport),
            placement: spec.preferred,
            flipped: false,
        }
    }
}

fn fits_primary_axis(
    bounds: OverlayBounds,
    viewport: OverlayBounds,
    placement: OverlayPlacement,
) -> bool {
    match placement {
        OverlayPlacement::Top | OverlayPlacement::Bottom => {
            bounds.y >= viewport.y && bounds.y + bounds.height <= viewport.y + viewport.height
        }
        OverlayPlacement::Left
        | OverlayPlacement::Right
        | OverlayPlacement::Start
        | OverlayPlacement::End => {
            bounds.x >= viewport.x && bounds.x + bounds.width <= viewport.x + viewport.width
        }
        OverlayPlacement::Center => true,
    }
}

fn candidate_bounds(
    spec: &OverlaySpec,
    placement: OverlayPlacement,
    viewport: OverlayBounds,
) -> OverlayBounds {
    match placement {
        OverlayPlacement::Top => OverlayBounds {
            x: spec.anchor.x + (spec.anchor.width - spec.width) / 2.0,
            y: spec.anchor.y - spec.height - spec.gap,
            width: spec.width,
            height: spec.height,
        },
        OverlayPlacement::Bottom => OverlayBounds {
            x: spec.anchor.x + (spec.anchor.width - spec.width) / 2.0,
            y: spec.anchor.y + spec.anchor.height + spec.gap,
            width: spec.width,
            height: spec.height,
        },
        OverlayPlacement::Left => OverlayBounds {
            x: spec.anchor.x - spec.width - spec.gap,
            y: spec.anchor.y + (spec.anchor.height - spec.height) / 2.0,
            width: spec.width,
            height: spec.height,
        },
        OverlayPlacement::Right => OverlayBounds {
            x: spec.anchor.x + spec.anchor.width + spec.gap,
            y: spec.anchor.y + (spec.anchor.height - spec.height) / 2.0,
            width: spec.width,
            height: spec.height,
        },
        OverlayPlacement::Start => candidate_bounds(spec, OverlayPlacement::Left, viewport),
        OverlayPlacement::End => candidate_bounds(spec, OverlayPlacement::Right, viewport),
        OverlayPlacement::Center => OverlayBounds {
            x: viewport.x + (viewport.width - spec.width) / 2.0,
            y: viewport.y + (viewport.height - spec.height) / 2.0,
            width: spec.width,
            height: spec.height,
        },
    }
}

fn validate_bounds(bounds: OverlayBounds) -> Result<(), OverlayError> {
    validate_size(bounds.width, bounds.height, 0.0)?;
    if bounds.x.is_finite() && bounds.y.is_finite() {
        Ok(())
    } else {
        Err(OverlayError::InvalidGeometry)
    }
}

fn validate_size(width: f64, height: f64, gap: f64) -> Result<(), OverlayError> {
    if width.is_finite()
        && width >= 0.0
        && height.is_finite()
        && height >= 0.0
        && gap.is_finite()
        && gap >= 0.0
    {
        Ok(())
    } else {
        Err(OverlayError::InvalidGeometry)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DismissReport {
    pub dismissed: Vec<OverlayId>,
    pub restore_focus: Option<FocusToken>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum OverlayError {
    #[error("overlay geometry must be finite and non-negative")]
    InvalidGeometry,
    #[error("overlay `{0:?}` is already open")]
    Duplicate(OverlayId),
    #[error("overlay parent `{0:?}` is not open")]
    MissingParent(OverlayId),
    #[error("overlay `{0:?}` cannot parent itself")]
    ParentCycle(OverlayId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> OverlayBounds {
        OverlayBounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }

    fn spec(id: &str, parent: Option<&str>, anchor: OverlayBounds) -> OverlaySpec {
        OverlaySpec {
            id: OverlayId::new(id),
            parent: parent.map(OverlayId::new),
            kind: OverlayKind::Popover,
            anchor,
            width: 180.0,
            height: 120.0,
            preferred: OverlayPlacement::Bottom,
            gap: 8.0,
            modal: false,
            dismiss_on_escape: true,
            dismiss_on_outside: true,
            restore_focus: Some(FocusToken(format!("focus-{id}"))),
        }
    }

    #[test]
    fn placement_flips_then_clamps_to_window() {
        let mut manager = OverlayManager::new(viewport()).unwrap();
        let result = manager
            .open(spec(
                "menu",
                None,
                OverlayBounds {
                    x: 700.0,
                    y: 560.0,
                    width: 80.0,
                    height: 30.0,
                },
            ))
            .unwrap();
        assert_eq!(result.placement, OverlayPlacement::Top);
        assert!(result.flipped);
        assert!(result.bounds.x >= 0.0 && result.bounds.x + result.bounds.width <= 800.0);
        assert!(result.bounds.y >= 0.0 && result.bounds.y + result.bounds.height <= 600.0);
    }

    #[test]
    fn centered_dialog_ignores_anchor_and_fits_viewport() {
        let mut manager = OverlayManager::new(viewport()).unwrap();
        let mut dialog = spec(
            "dialog",
            None,
            OverlayBounds {
                x: 710.0,
                y: 570.0,
                width: 40.0,
                height: 20.0,
            },
        );
        dialog.kind = OverlayKind::Dialog;
        dialog.preferred = OverlayPlacement::Center;
        dialog.width = 320.0;
        dialog.height = 200.0;
        let result = manager.open(dialog).unwrap();
        assert_eq!(result.placement, OverlayPlacement::Center);
        assert!((result.bounds.x - 240.0).abs() < f64::EPSILON);
        assert!((result.bounds.y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn clicking_trigger_is_not_an_outside_dismissal() {
        let mut manager = OverlayManager::new(viewport()).unwrap();
        manager
            .open(spec(
                "popover",
                None,
                OverlayBounds {
                    x: 100.0,
                    y: 100.0,
                    width: 80.0,
                    height: 30.0,
                },
            ))
            .unwrap();
        let report = manager.handle_outside_click(120.0, 110.0);
        assert!(report.dismissed.is_empty());
        assert!(manager.z_index(&OverlayId::new("popover")).is_some());
    }

    #[test]
    fn tooltip_delay_cancels_on_leave_and_transitions_deterministically() {
        let start = Instant::now();
        let mut tooltips = TooltipScheduler::default();
        let id = OverlayId::new("help");
        tooltips.pointer_enter(id.clone(), start, Duration::from_millis(300));
        assert_eq!(
            tooltips.tick(start + Duration::from_millis(299)),
            TooltipTransition::default()
        );
        tooltips.pointer_leave(&id, start + Duration::from_millis(299), Duration::ZERO);
        assert_eq!(
            tooltips.tick(start + Duration::from_millis(300)),
            TooltipTransition::default()
        );
        tooltips.pointer_enter(id.clone(), start, Duration::from_millis(100));
        assert_eq!(
            tooltips.tick(start + Duration::from_millis(100)).shown,
            Some(id.clone())
        );
        tooltips.pointer_leave(&id, start, Duration::from_millis(50));
        assert_eq!(
            tooltips.tick(start + Duration::from_millis(50)).hidden,
            Some(id)
        );
    }

    #[test]
    fn dismissing_parent_closes_nested_children_top_down() {
        let mut manager = OverlayManager::new(viewport()).unwrap();
        let anchor = OverlayBounds {
            x: 100.0,
            y: 100.0,
            width: 80.0,
            height: 30.0,
        };
        manager.open(spec("parent", None, anchor)).unwrap();
        manager.open(spec("child", Some("parent"), anchor)).unwrap();
        let report = manager.dismiss(&OverlayId::new("parent"));
        assert_eq!(
            report.dismissed,
            vec![OverlayId::new("child"), OverlayId::new("parent")]
        );
        assert_eq!(
            report.restore_focus,
            Some(FocusToken("focus-child".to_owned()))
        );
    }

    #[test]
    fn escape_closes_only_topmost_dismissible_overlay() {
        let mut manager = OverlayManager::new(viewport()).unwrap();
        let anchor = OverlayBounds {
            x: 100.0,
            y: 100.0,
            width: 80.0,
            height: 30.0,
        };
        manager.open(spec("first", None, anchor)).unwrap();
        manager.open(spec("second", None, anchor)).unwrap();
        assert_eq!(
            manager.handle_escape().dismissed,
            vec![OverlayId::new("second")]
        );
        assert!(manager.placement(&OverlayId::new("first")).is_some());
    }
}
