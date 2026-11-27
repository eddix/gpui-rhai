//! Derived text and native layout from a single set of sampled scalar tracks.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use rhai::{Array, CustomType, Engine, EvalAltResult, INT, ImmutableString, Position, TypeBuilder};

use crate::{
    ComponentInstancePath, ElementRef, ElementRefRegistry, MotionKey, MotionProperty, UiNode,
};

const MAX_NUMBER_LANES: usize = 128;

/// A bounded set of component-local peers. Each ref owns one `number_value` track.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MotionNumbers {
    refs: Rc<[ElementRef]>,
    capacity: Option<CapacitySource>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CapacitySource {
    // Canonical finite bits participate in the view-frame cache key.
    Fixed(u64),
    Track(ElementRef),
}

impl MotionNumbers {
    /// Construct a group with stable lane order.
    ///
    /// # Errors
    ///
    /// Requires 1–128 distinct refs in a single component scope.
    pub fn new(refs: Vec<ElementRef>) -> Result<Self, &'static str> {
        let Some(first) = refs.first() else {
            return Err("a numeric motion group cannot be empty");
        };
        if refs.len() > MAX_NUMBER_LANES
            || refs
                .iter()
                .any(|r| r.id().component() != first.id().component())
            || refs.iter().collect::<BTreeSet<_>>().len() != refs.len()
        {
            return Err("numeric motion groups need 1–128 distinct refs in one component");
        }
        Ok(Self {
            refs: refs.into(),
            capacity: None,
        })
    }

    /// Use a fixed, accepted capacity as the minimum denominator for widths.
    ///
    /// # Errors
    ///
    /// Capacity must be finite and nonnegative.
    pub fn with_capacity(mut self, value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() || value < 0.0 {
            return Err("numeric motion capacity must be finite and nonnegative");
        }
        self.capacity = Some(CapacitySource::Fixed(value.max(0.0).to_bits()));
        Ok(self)
    }

    /// Use the capacity track of another element in the same component.
    /// This track never contributes to the sum of the allocation lanes.
    ///
    /// # Errors
    ///
    /// The reference must share the same component and cannot be an allocation lane.
    pub fn with_capacity_source(mut self, source: ElementRef) -> Result<Self, &'static str> {
        if source.id().component() != self.refs[0].id().component() || self.refs.contains(&source) {
            return Err("a capacity track must be a distinct peer in the component");
        }
        self.capacity = Some(CapacitySource::Track(source));
        Ok(self)
    }
}

impl CustomType for MotionNumbers {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("MotionNumbers");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumberProjection {
    Sum,
    WeightedSum,
    Remaining,
    CapacityPercent,
    Amount(usize),
    Percent(usize),
    WidthFraction(usize),
    /// A slot in a flex distribution, not a percentage of the parent's box.
    FlexShare(FlexShare),
    /// The logical midpoint of a lane in a capacity or proportional strip.
    StripHint(StripHint),
    Partition(PartitionPlacement),
}

/// A strip lane and its accepted anchor before all sampled values are ready.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StripHint {
    index: usize,
    fallback: u64,
}

impl StripHint {
    /// Construct a hint with an accepted logical anchor in 0..=1.
    ///
    /// # Errors
    ///
    /// Rejects a non-finite or out-of-range anchor.
    pub fn new(index: usize, fallback: f64) -> Result<Self, &'static str> {
        if !(0.0..=1.0).contains(&fallback) {
            return Err("strip hint fallback must be a finite fraction in 0..=1");
        }
        Ok(Self {
            index,
            fallback: fallback.to_bits(),
        })
    }

    pub(crate) const fn fallback(self) -> f64 {
        f64::from_bits(self.fallback)
    }
}

/// The lane and exclusive visibility threshold of a flex distribution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlexShare {
    index: usize,
    cutoff: u64,
}

impl FlexShare {
    /// A zero lane or one at or below the cutoff does not occupy a slot.
    ///
    /// # Errors
    ///
    /// The cutoff must be a finite fraction in 0..=1.
    pub fn new(index: usize, cutoff: f64) -> Result<Self, &'static str> {
        if !(0.0..=1.0).contains(&cutoff) {
            return Err("flex share cutoff must be a finite fraction in 0..=1");
        }
        Ok(Self {
            index,
            cutoff: cutoff.to_bits(),
        })
    }
}

/// A projection into one frame of a jointly laid out partition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PartitionPlacement {
    lane: Option<usize>,
    viewport: [u64; 2],
    gap: u64,
    role: PartitionRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PartitionRole {
    Tile,
    Tooltip,
    Visibility([u64; 2]),
}

impl PartitionPlacement {
    /// Build a tile placement or an anchored, bounded tooltip placement.
    /// A missing lane denotes unused capacity and cannot anchor a tooltip.
    ///
    /// # Errors
    ///
    /// Requires a positive finite viewport and a gap between 0 and 32 pixels.
    pub fn new(
        lane: Option<usize>,
        viewport: [f64; 2],
        gap: f64,
        tooltip: bool,
    ) -> Result<Self, &'static str> {
        if !viewport
            .into_iter()
            .all(|v| v.is_finite() && v > 0.0 && v <= crate::context_partition::MAX_AXIS)
            || !gap.is_finite()
            || !(0.0..=32.0).contains(&gap)
            || (tooltip && lane.is_none())
        {
            return Err("invalid numeric partition viewport, gap, or tooltip lane");
        }
        Ok(Self {
            lane,
            viewport: viewport.map(f64::to_bits),
            gap: gap.to_bits(),
            role: if tooltip {
                PartitionRole::Tooltip
            } else {
                PartitionRole::Tile
            },
        })
    }

    /// Gate a decorative node by the currently presented tile's inner dimensions.
    ///
    /// # Errors
    ///
    /// Requires a valid viewport, gap, and finite nonnegative minimum dimensions.
    pub fn visibility(
        lane: Option<usize>,
        viewport: [f64; 2],
        gap: f64,
        minimum: [f64; 2],
    ) -> Result<Self, &'static str> {
        let mut placement = Self::new(lane, viewport, gap, false)?;
        if !minimum.into_iter().all(|v| v.is_finite() && v >= 0.0) {
            return Err("invalid numeric partition visibility threshold");
        }
        placement.role = PartitionRole::Visibility(minimum.map(f64::to_bits));
        Ok(placement)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SampledPartition {
    Tile([f64; 4]),
    Tooltip([f64; 4]),
    Visibility(bool),
    Collapsed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumberMotionBinding {
    pub(crate) group: MotionNumbers,
    pub(crate) projection: NumberProjection,
}

impl NumberMotionBinding {
    /// Bind a projection to a captured set of numerical peers.
    ///
    /// # Errors
    ///
    /// A lane projection must address an existing lane.
    pub fn new(group: MotionNumbers, projection: NumberProjection) -> Result<Self, &'static str> {
        match projection {
            NumberProjection::Sum | NumberProjection::WeightedSum => {}
            NumberProjection::Remaining | NumberProjection::CapacityPercent => {
                if group.capacity.is_none() {
                    return Err("this numeric projection requires a capacity");
                }
            }
            NumberProjection::Amount(index)
            | NumberProjection::Percent(index)
            | NumberProjection::WidthFraction(index)
                if index < group.refs.len() => {}
            NumberProjection::FlexShare(share) if share.index < group.refs.len() => {}
            NumberProjection::StripHint(hint) if hint.index < group.refs.len() => {}
            NumberProjection::Partition(placement)
                if placement.lane.is_none_or(|index| index < group.refs.len())
                    && group.capacity.is_some() => {}
            _ => return Err("numeric motion lane is out of range"),
        }
        Ok(Self { group, projection })
    }
}

/// The immutable source values of one view frame. Every consumer uses the
/// same sum and lane value, regardless of render or measure order.
#[derive(Default)]
pub(crate) struct NumberMotionFrame {
    sources: BTreeMap<crate::ElementRefId, NumberLane>,
    sums: RefCell<BTreeMap<MotionNumbers, Option<f64>>>,
    partitions: RefCell<BTreeMap<PartitionGroup, Option<PartitionRects>>>,
}

type PartitionGroup = (MotionNumbers, [u64; 2]);
type PartitionRects = BTreeMap<Option<usize>, crate::context_partition::Rect>;

struct NumberLane {
    amount: f64,
    weight: f64,
}

impl NumberMotionFrame {
    fn capacity(&self, group: &MotionNumbers) -> Option<f64> {
        match &group.capacity {
            None => Some(0.0),
            Some(CapacitySource::Fixed(bits)) => Some(f64::from_bits(*bits)),
            Some(CapacitySource::Track(source)) => {
                self.sources.get(source.id()).map(|lane| lane.amount)
            }
        }
    }

    pub(crate) fn capture(
        values: &BTreeMap<MotionKey, f64>,
        refs: &ElementRefRegistry,
        owner: &ComponentInstancePath,
        domain: &str,
    ) -> Self {
        let sources = if values
            .keys()
            .any(|key| key.property == MotionProperty::NumberValue)
        {
            refs.iter()
                .filter(|(reference, _)| reference.component().is_within(owner))
                .filter_map(|(reference, id)| {
                    let path = crate::motion::retained_node_path(domain, *id);
                    let key = MotionKey::for_node(&path, MotionProperty::NumberValue);
                    let amount = values
                        .get(&key)
                        .copied()
                        .filter(|v| v.is_finite())?
                        .max(0.0);
                    let weight = values
                        .get(&MotionKey::for_node(&path, MotionProperty::NumberWeight))
                        .copied()
                        .filter(|v| v.is_finite())
                        .unwrap_or(1.0)
                        .clamp(0.0, 1.0);
                    Some((reference.clone(), NumberLane { amount, weight }))
                })
                .collect()
        } else {
            BTreeMap::new()
        };
        Self {
            sources,
            ..Self::default()
        }
    }

    pub(crate) fn resolve(&self, binding: &NumberMotionBinding) -> Option<f64> {
        let lane = |index: usize| self.sources.get(binding.group.refs[index].id());
        let get = |index| lane(index).map(|lane| lane.amount);
        let mut lanes = 0..binding.group.refs.len();
        let mut sums = self.sums.borrow_mut();
        let sum = (*sums.entry(binding.group.clone()).or_insert_with(|| {
            let sum = lanes
                .clone()
                .try_fold(0.0, |sum, index| Some(sum + get(index)?))?;
            sum.is_finite().then_some(sum)
        }))?;
        match binding.projection {
            NumberProjection::Sum => Some(sum),
            NumberProjection::WeightedSum => lanes.try_fold(0.0, |sum, index| {
                let lane = lane(index)?;
                Some(sum + lane.amount * lane.weight)
            }),
            NumberProjection::Remaining => Some((self.capacity(&binding.group)? - sum).max(0.0)),
            NumberProjection::CapacityPercent => {
                if sum == 0.0 {
                    return Some(0.0);
                }
                let percent = sum / self.capacity(&binding.group)? * 100.0;
                percent.is_finite().then_some(percent)
            }
            NumberProjection::Amount(index) => get(index),
            NumberProjection::Percent(index) if sum > 0.0 => Some((get(index)? / sum) * 100.0),
            NumberProjection::FlexShare(share) if sum > 0.0 => {
                let fraction = get(share.index)? / sum;
                // Use percent as the grow factor. Normalized weights would
                // make Taffy leave free space if any small lane is omitted.
                Some(if fraction > f64::from_bits(share.cutoff) {
                    fraction * 100.0
                } else {
                    0.0
                })
            }
            NumberProjection::WidthFraction(index) => {
                let width_base = self.capacity(&binding.group)?.max(sum);
                if width_base > 0.0 {
                    Some(get(index)? / width_base)
                } else {
                    Some(0.0)
                }
            }
            NumberProjection::StripHint(hint) => {
                let amount = get(hint.index)?;
                let base = self.capacity(&binding.group)?.max(sum);
                if amount <= 0.0 || base <= 0.0 {
                    return None;
                }
                let prefix = (0..hint.index).try_fold(0.0, |sum, index| Some(sum + get(index)?))?;
                Some(((prefix + amount * 0.5) / base).clamp(0.0, 1.0))
            }
            NumberProjection::Percent(_) | NumberProjection::FlexShare(_) => Some(0.0),
            NumberProjection::Partition(_) => None,
        }
    }

    pub(crate) fn resolve_partition(
        &self,
        binding: &NumberMotionBinding,
    ) -> Option<SampledPartition> {
        let NumberProjection::Partition(placement) = binding.projection else {
            return None;
        };
        let viewport = placement.viewport.map(f64::from_bits);
        let mut groups = self.partitions.borrow_mut();
        let rects = groups
            .entry((binding.group.clone(), placement.viewport))
            .or_insert_with(|| {
                let amounts: Vec<_> = binding
                    .group
                    .refs
                    .iter()
                    .map(|r| self.sources.get(r.id()).map(|lane| lane.amount))
                    .collect::<Option<_>>()?;
                let sum: f64 = amounts.iter().sum();
                if !sum.is_finite() {
                    return None;
                }
                let free = (self.capacity(&binding.group)? - sum).max(0.0);
                Some(
                    crate::context_partition::partition_lanes(amounts, free, viewport)
                        .into_iter()
                        .map(|tile| (tile.block, tile.rect))
                        .collect(),
                )
            })
            .as_ref()?;
        let rect = rects.get(&placement.lane).copied();
        let inset = |rect: crate::context_partition::Rect| {
            rect.inset(viewport, f64::from_bits(placement.gap))
        };
        if let PartitionRole::Visibility(minimum) = placement.role {
            let [min_w, min_h] = minimum.map(f64::from_bits);
            return Some(SampledPartition::Visibility(rect.is_some_and(|rect| {
                let [_, _, w, h] = inset(rect);
                w > 0.0 && h > 0.0 && w * viewport[0] >= min_w && h * viewport[1] >= min_h
            })));
        }
        let Some(rect) = rect else {
            return Some(SampledPartition::Collapsed);
        };
        if placement.role == PartitionRole::Tooltip {
            Some(SampledPartition::Tooltip(rect.tooltip(viewport)))
        } else {
            Some(SampledPartition::Tile(inset(rect)))
        }
    }
}

fn error(message: &'static str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.into(), Position::NONE)
}

fn partition_lane(index: INT) -> Result<Option<usize>, EvalAltResult> {
    if index == -1 {
        Ok(None)
    } else {
        usize::try_from(index)
            .map(Some)
            .map_err(|_| error("numeric partition lane is out of range"))
    }
}

pub(crate) fn register_number_motion_api(engine: &mut Engine) {
    engine.build_type::<MotionNumbers>();
    engine.register_fn(
        "with_capacity",
        |group: &mut MotionNumbers,
         reference: ElementRef|
         -> Result<MotionNumbers, Box<EvalAltResult>> {
            group
                .clone()
                .with_capacity_source(reference)
                .map_err(|message| error(message).into())
        },
    );
    engine.register_fn(
        "with_capacity",
        |group: &mut MotionNumbers,
         value: rhai::FLOAT|
         -> Result<MotionNumbers, Box<EvalAltResult>> {
            group
                .clone()
                .with_capacity(value)
                .map_err(|message| error(message).into())
        },
    );
    engine.register_fn(
        "motion_numbers",
        |refs: Array| -> Result<MotionNumbers, Box<EvalAltResult>> {
            if refs.len() > MAX_NUMBER_LANES {
                return Err(error("numeric motion groups support at most 128 refs").into());
            }
            let refs = refs
                .into_iter()
                .map(|value| {
                    value
                        .try_cast::<ElementRef>()
                        .ok_or_else(|| error("expected an ElementRef"))
                })
                .collect::<Result<_, _>>()?;
            MotionNumbers::new(refs).map_err(|message| error(message).into())
        },
    );
    engine.register_fn("number_motion_source", |node: &mut UiNode| {
        node.clone().with_number_motion_source()
    });
    engine.register_fn(
        "sample_numbers",
        |node: &mut UiNode,
         group: MotionNumbers,
         projection: ImmutableString,
         index: INT|
         -> Result<UiNode, Box<EvalAltResult>> {
            let index =
                usize::try_from(index).map_err(|_| error("numeric motion lane is out of range"))?;
            let projection = match projection.as_str() {
                "sum" => NumberProjection::Sum,
                "weighted_sum" => NumberProjection::WeightedSum,
                "remaining" => NumberProjection::Remaining,
                "capacity_percent" => NumberProjection::CapacityPercent,
                "amount" => NumberProjection::Amount(index),
                "percent" => NumberProjection::Percent(index),
                "width_fraction" => NumberProjection::WidthFraction(index),
                _ => return Err(error("unknown numeric motion projection").into()),
            };
            let binding = NumberMotionBinding::new(group, projection).map_err(error)?;
            node.clone()
                .with_number_motion_binding(binding)
                .map_err(|message| error(message).into())
        },
    );
    register_strip_projection_api(engine);
    register_partition_projection_api(engine);
}

fn register_strip_projection_api(engine: &mut Engine) {
    engine.register_fn(
        "sample_flex_share",
        |node: &mut UiNode,
         group: MotionNumbers,
         index: INT,
         cutoff: rhai::FLOAT|
         -> Result<UiNode, Box<EvalAltResult>> {
            let index =
                usize::try_from(index).map_err(|_| error("numeric motion lane is out of range"))?;
            let projection =
                NumberProjection::FlexShare(FlexShare::new(index, cutoff).map_err(error)?);
            let binding = NumberMotionBinding::new(group, projection).map_err(error)?;
            node.clone()
                .with_number_motion_binding(binding)
                .map_err(|message| error(message).into())
        },
    );
    engine.register_fn(
        "sample_strip_hint",
        |node: &mut UiNode,
         group: MotionNumbers,
         index: INT,
         fallback: rhai::FLOAT|
         -> Result<UiNode, Box<EvalAltResult>> {
            let index =
                usize::try_from(index).map_err(|_| error("numeric motion lane is out of range"))?;
            let projection =
                NumberProjection::StripHint(StripHint::new(index, fallback).map_err(error)?);
            let binding = NumberMotionBinding::new(group, projection).map_err(error)?;
            node.clone()
                .with_number_motion_binding(binding)
                .map_err(|message| error(message).into())
        },
    );
}

fn register_partition_projection_api(engine: &mut Engine) {
    engine.register_fn(
        "sample_partition",
        |node: &mut UiNode,
         group: MotionNumbers,
         index: INT,
         width: rhai::FLOAT,
         height: rhai::FLOAT,
         gap: rhai::FLOAT,
         tooltip: bool|
         -> Result<UiNode, Box<EvalAltResult>> {
            let lane = partition_lane(index)?;
            let placement =
                PartitionPlacement::new(lane, [width, height], gap, tooltip).map_err(error)?;
            let binding = NumberMotionBinding::new(group, NumberProjection::Partition(placement))
                .map_err(error)?;
            node.clone()
                .with_number_motion_binding(binding)
                .map_err(|message| error(message).into())
        },
    );
    engine.register_fn(
        "sample_partition_visibility",
        |node: &mut UiNode,
         group: MotionNumbers,
         index: INT,
         width: rhai::FLOAT,
         height: rhai::FLOAT,
         gap: rhai::FLOAT,
         min_width: rhai::FLOAT,
         min_height: rhai::FLOAT|
         -> Result<UiNode, Box<EvalAltResult>> {
            let placement = PartitionPlacement::visibility(
                partition_lane(index)?,
                [width, height],
                gap,
                [min_width, min_height],
            )
            .map_err(error)?;
            let binding = NumberMotionBinding::new(group, NumberProjection::Partition(placement))
                .map_err(error)?;
            node.clone()
                .with_number_motion_binding(binding)
                .map_err(|message| error(message).into())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementRefId, NodeId};

    fn reference(scope: &ComponentInstancePath, key: &str) -> ElementRef {
        ElementRef::new(ElementRefId::new(scope.clone(), key).unwrap())
    }

    #[test]
    fn partition_capacity_is_a_separate_peer_and_visibility_follows_the_inset_frame() {
        let owner = ComponentInstancePath::root("View", "sample");
        let used = reference(&owner, "used");
        let limit = reference(&owner, "limit");
        let peers = MotionNumbers::new(vec![used.clone()]).unwrap();
        assert!(peers.clone().with_capacity_source(used.clone()).is_err());
        let g = peers.with_capacity_source(limit.clone()).unwrap();
        let viewport = [120.0, 80.0];
        let bind = |placement| {
            NumberMotionBinding::new(g.clone(), NumberProjection::Partition(placement)).unwrap()
        };
        let tile = bind(PartitionPlacement::new(Some(0), viewport, 3.0, false).unwrap());
        let name =
            bind(PartitionPlacement::visibility(Some(0), viewport, 3.0, [50.0, 20.0]).unwrap());
        let frame = |capacity| NumberMotionFrame {
            sources: [
                (
                    used.id().clone(),
                    NumberLane {
                        amount: 30.0,
                        weight: 1.0,
                    },
                ),
                (
                    limit.id().clone(),
                    NumberLane {
                        amount: capacity,
                        weight: 1.0,
                    },
                ),
            ]
            .into(),
            ..Default::default()
        };
        let small = frame(75.0);
        let large = frame(60.0);
        for (sample, expected_width, visible) in [(small, 45.0, false), (large, 57.0, true)] {
            let Some(SampledPartition::Tile([_, _, width, _])) = sample.resolve_partition(&tile)
            else {
                panic!("missing allocation tile");
            };
            assert!((width * viewport[0] - expected_width).abs() < 1e-10);
            assert!(
                matches!(sample.resolve_partition(&name), Some(SampledPartition::Visibility(value)) if value == visible)
            );
            assert_eq!(sample.partitions.borrow().len(), 1);
        }
        assert!(
            frame(0.0).resolve_partition(&tile).is_some(),
            "unknown capacity still lays out used blocks"
        );
    }

    #[test]
    fn partition_tiles_and_tooltip_resolve_once_per_viewport_from_the_same_lanes() {
        let owner = ComponentInstancePath::root("View", "map");
        let refs = [reference(&owner, "lane-0"), reference(&owner, "lane-1")];
        let group = MotionNumbers::new(refs.to_vec())
            .unwrap()
            .with_capacity(100.0)
            .unwrap();
        let frame = |amounts: [f64; 2]| NumberMotionFrame {
            sources: refs
                .iter()
                .zip(amounts)
                .map(|(r, amount)| {
                    (
                        r.id().clone(),
                        NumberLane {
                            amount,
                            weight: 1.0,
                        },
                    )
                })
                .collect(),
            ..NumberMotionFrame::default()
        };
        let bind = |lane, viewport, tooltip| {
            NumberMotionBinding::new(
                group.clone(),
                NumberProjection::Partition(
                    PartitionPlacement::new(lane, viewport, 0.0, tooltip).unwrap(),
                ),
            )
            .unwrap()
        };
        let valid = frame([30.25, 50.75]);
        for viewport in [[700.0, 210.0], [240.0, 640.0]] {
            let mut rects: Vec<[f64; 4]> = Vec::new();
            for (lane, share) in [(Some(1), 0.5075), (None, 0.19), (Some(0), 0.3025)] {
                let Some(SampledPartition::Tile(rect)) =
                    valid.resolve_partition(&bind(lane, viewport, false))
                else {
                    panic!("the sampled lane {lane:?} must have a native box");
                };
                let [x, y, w, h] = rect;
                assert!(x >= 0.0 && y >= 0.0 && x + w <= 1.0 && y + h <= 1.0);
                assert!((w * h - share).abs() < 1e-10);
                for [a, b, u, v] in &rects {
                    let overlap = ((x + w).min(a + u) - x.max(*a)).max(0.0)
                        * ((y + h).min(b + v) - y.max(*b)).max(0.0);
                    assert!(overlap < 1e-10);
                }
                rects.push(rect);
            }
            let Some(SampledPartition::Tooltip([x, y, w, h])) =
                valid.resolve_partition(&bind(Some(0), viewport, true))
            else {
                panic!("a live lane anchors its readout");
            };
            assert!(x >= 0.0 && y >= 0.0 && x + w <= viewport[0] && y + h <= viewport[1]);
        }
        assert_eq!(valid.partitions.borrow().len(), 2);
        let zero = frame([0.0, 100.0]);
        assert!(matches!(
            zero.resolve_partition(&bind(Some(0), [200.0, 100.0], false)),
            Some(SampledPartition::Collapsed)
        ));
        assert!(matches!(
            zero.resolve_partition(&bind(None, [200.0, 100.0], false)),
            Some(SampledPartition::Collapsed)
        ));
        assert!(
            NumberMotionFrame::default()
                .resolve_partition(&bind(Some(0), [200.0, 100.0], false))
                .is_none()
        );
    }

    fn assert_bounded_projections(
        frame: &NumberMotionFrame,
        group: &MotionNumbers,
        no_tail: &NumberMotionFrame,
    ) {
        let binding = |projection| NumberMotionBinding::new(group.clone(), projection).unwrap();
        let capacity = group.clone().with_capacity(400.0).unwrap();
        let bounded = |projection| NumberMotionBinding::new(capacity.clone(), projection).unwrap();
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::WidthFraction(0))),
            Some(0.15)
        );
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::FlexShare(
                FlexShare::new(0, 0.29).unwrap()
            ))),
            Some(30.0)
        );
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::FlexShare(
                FlexShare::new(0, 0.30).unwrap()
            ))),
            Some(0.0)
        );
        assert!(FlexShare::new(0, f64::NAN).is_err());
        assert!(FlexShare::new(0, -0.1).is_err());
        assert!(FlexShare::new(0, 1.1).is_err());
        let hint = NumberProjection::StripHint(StripHint::new(1, 0.4).unwrap());
        assert_eq!(frame.resolve(&bounded(hint)), Some(0.325));
        assert_eq!(frame.resolve(&binding(hint)), Some(0.65));
        assert_eq!(NumberMotionFrame::default().resolve(&binding(hint)), None);
        assert_eq!(no_tail.resolve(&binding(hint)), None);
        assert!(StripHint::new(1, f64::NAN).is_err());
        assert!(StripHint::new(1, -0.1).is_err());
        assert!(StripHint::new(1, 1.1).is_err());
        assert!(
            NumberMotionBinding::new(
                group.clone(),
                NumberProjection::StripHint(StripHint::new(2, 0.5).unwrap())
            )
            .is_err()
        );
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::Percent(0))),
            Some(30.0)
        );
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::CapacityPercent)),
            Some(50.0)
        );
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::Remaining)),
            Some(200.0)
        );
        let over = group.clone().with_capacity(100.0).unwrap();
        let bounded = |projection| NumberMotionBinding::new(over.clone(), projection).unwrap();
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::WidthFraction(0))),
            Some(0.3)
        );
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::CapacityPercent)),
            Some(200.0)
        );
        assert_eq!(
            frame.resolve(&bounded(NumberProjection::Remaining)),
            Some(0.0)
        );
        assert!(group.clone().with_capacity(f64::NAN).is_err());
        assert!(
            NumberMotionBinding::new(group.clone(), NumberProjection::CapacityPercent).is_err()
        );
    }

    #[test]
    fn a_numeric_frame_derives_amount_share_width_and_attention_from_the_same_samples() {
        let root = ComponentInstancePath::root("View", "one");
        let owner = root.child("Breakdown", "cost");
        let other = ComponentInstancePath::root("View", "other").child("Breakdown", "cost");
        let a = reference(&owner, "a");
        let b = reference(&owner, "b");
        let c = reference(&other, "a");
        let mut refs = ElementRefRegistry::new();
        refs.reconcile(
            &root,
            [
                (a.id().clone(), NodeId::from_raw(1)),
                (b.id().clone(), NodeId::from_raw(2)),
            ]
            .into(),
        );
        // Retained ids are only local to one tree, so overlapping ids must not
        // make a foreign ref sample this view's track.
        refs.reconcile(
            c.id().component(),
            [(c.id().clone(), NodeId::from_raw(1))].into(),
        );
        let domain = "window:main/view:one/root";
        let track = |id, kind| {
            MotionKey::for_node(
                &crate::motion::retained_node_path(domain, NodeId::from_raw(id)),
                kind,
            )
        };
        let values = [
            (track(1, MotionProperty::NumberValue), 60.0),
            (track(2, MotionProperty::NumberValue), 140.0),
            (track(2, MotionProperty::NumberWeight), 0.5),
        ]
        .into();
        let group = MotionNumbers::new(vec![a.clone(), b.clone()]).unwrap();
        let binding = |projection| NumberMotionBinding::new(group.clone(), projection).unwrap();
        let frame = NumberMotionFrame::capture(&values, &refs, &root, domain);
        assert_eq!(frame.resolve(&binding(NumberProjection::Sum)), Some(200.0));
        assert_eq!(
            frame.resolve(&binding(NumberProjection::Amount(0))),
            Some(60.0)
        );
        assert_eq!(
            frame.resolve(&binding(NumberProjection::Percent(0))),
            Some(30.0)
        );
        assert_eq!(
            frame.resolve(&binding(NumberProjection::WidthFraction(0))),
            Some(0.3)
        );
        assert_eq!(
            frame.resolve(&binding(NumberProjection::WeightedSum)),
            Some(130.0)
        );
        assert_eq!(frame.sums.borrow().len(), 1);
        let no_tail = [
            (track(1, MotionProperty::NumberValue), 60.0),
            (track(2, MotionProperty::NumberValue), 0.0),
        ]
        .into();
        let no_tail = NumberMotionFrame::capture(&no_tail, &refs, &root, domain);
        assert_bounded_projections(&frame, &group, &no_tail);
        let foreign = NumberMotionBinding::new(
            MotionNumbers::new(vec![c.clone()]).unwrap(),
            NumberProjection::Sum,
        )
        .unwrap();
        assert_eq!(frame.resolve(&foreign), None);
        assert!(MotionNumbers::new(vec![a.clone(), c]).is_err());
        assert!(MotionNumbers::new(vec![a.clone(), a]).is_err());
        assert!(NumberMotionBinding::new(group.clone(), NumberProjection::Amount(2)).is_err());

        let partial = [(track(1, MotionProperty::NumberValue), 100.0)].into();
        let partial = NumberMotionFrame::capture(&partial, &refs, &root, domain);
        assert_eq!(
            partial.resolve(&binding(NumberProjection::Percent(0))),
            None
        );
        let zero = [
            (track(1, MotionProperty::NumberValue), 0.0),
            (track(2, MotionProperty::NumberValue), 0.0),
        ]
        .into();
        let zero = NumberMotionFrame::capture(&zero, &refs, &root, domain);
        assert_eq!(
            zero.resolve(&binding(NumberProjection::Percent(0))),
            Some(0.0)
        );
        assert_eq!(
            zero.resolve(&binding(NumberProjection::WidthFraction(0))),
            Some(0.0)
        );
    }
}
