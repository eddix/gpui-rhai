//! Ordered, area-proportional layout for a bounded context-allocation surface.
use rhai::{Array, Dynamic, Engine, EvalAltResult, INT, Map, Position};

const MAX_BLOCKS: usize = 64;
pub(crate) const MAX_AXIS: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    /// Project equal gutters into the containing block; never let gutters
    /// overlap adjacent tiles or completely consume a nonempty tile.
    pub(crate) fn inset(self, [w, h]: [f64; 2], gap: f64) -> [f64; 4] {
        let width = self.x1 - self.x0;
        let height = self.y1 - self.y0;
        let gap = gap.min((width * w).min(height * h) * 0.33);
        [
            self.x0 + gap / (2.0 * w),
            self.y0 + gap / (2.0 * h),
            width - gap / w,
            height - gap / h,
        ]
    }

    pub(crate) fn tooltip(self, [w, h]: [f64; 2]) -> [f64; 4] {
        let width = (w - 8.0).clamp(1.0, 310.0);
        let left = (self.x0 * w).clamp(4.0, (w - width - 4.0).max(4.0));
        let top = if self.y0 < 0.45 {
            (self.y1 * h + 5.0).min(h - 88.0).max(4.0)
        } else {
            (self.y0 * h - 95.0).max(4.0)
        };
        [left, top, width, (h - top - 2.0).max(1.0)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tile {
    /// An unused region is not identified by a special business ID.
    pub(crate) block: Option<usize>,
    pub(crate) rect: Rect,
}

#[derive(Debug, PartialEq)]
struct Partition {
    used: INT,
    free: INT,
    overflow: INT,
    tiles: Vec<Tile>,
}

#[allow(clippy::cast_precision_loss)] // Accepted counts are <= 2^53-1; only presentation uses floats.
fn project(counts: &[INT], limit: INT, width: f64, height: f64) -> Option<Partition> {
    use crate::context_capacity::MAX_COUNT;

    if counts.len() > MAX_BLOCKS
        || !(0..=MAX_COUNT).contains(&limit)
        || counts.iter().any(|n| !(0..=MAX_COUNT).contains(n))
        || ![width, height]
            .into_iter()
            .all(|axis| axis.is_finite() && axis > 0.0 && axis <= MAX_AXIS)
    {
        return None;
    }
    // The exact integer totals remain independent of the floating-point layout.
    let used = counts.iter().copied().try_fold(0, INT::checked_add)?;
    let free = (limit - used).max(0);
    let overflow = if limit > 0 { (used - limit).max(0) } else { 0 };
    let tiles = partition_lanes(
        counts.iter().map(|&value| value as f64),
        free as f64,
        [width, height],
    );
    Some(Partition {
        used,
        free,
        overflow,
        tiles,
    })
}

/// Shared layout for accepted and sampled weights. Callers validate the
/// viewport and keep the finite, nonnegative lane sequence bounded.
pub(crate) fn partition_lanes(
    lanes: impl IntoIterator<Item = f64>,
    free: f64,
    [width, height]: [f64; 2],
) -> Vec<Tile> {
    let weights: Vec<_> = lanes
        .into_iter()
        .enumerate()
        .filter_map(|(i, amount)| (amount > 0.0).then_some((Some(i), amount)))
        .chain((free > 0.0).then_some((None, free)))
        .collect();
    let mut tiles = Vec::with_capacity(weights.len());
    partition(
        &weights,
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        },
        width,
        height,
        &mut tiles,
    );
    tiles
}

fn partition(weights: &[(Option<usize>, f64)], rect: Rect, w: f64, h: f64, out: &mut Vec<Tile>) {
    match weights {
        [] => return,
        [(block, _)] => {
            out.push(Tile {
                block: *block,
                rect,
            });
            return;
        }
        _ => {}
    }
    let total: f64 = weights.iter().map(|(_, weight)| weight).sum();
    let mut left_sum = 0.0;
    let mut split = 1;
    for (i, (_, weight)) in weights[..weights.len() - 1].iter().enumerate() {
        split = i + 1;
        left_sum += weight;
        if left_sum >= total * 0.5 {
            break;
        }
    }
    let ratio = left_sum / total;
    let (near, far) = if (rect.x1 - rect.x0) * w >= (rect.y1 - rect.y0) * h {
        let x = rect.x0 + (rect.x1 - rect.x0) * ratio;
        (Rect { x1: x, ..rect }, Rect { x0: x, ..rect })
    } else {
        let y = rect.y0 + (rect.y1 - rect.y0) * ratio;
        (Rect { y1: y, ..rect }, Rect { y0: y, ..rect })
    };
    // Share the split coordinate rather than independently rounding boundaries.
    partition(&weights[..split], near, w, h, out);
    partition(&weights[split..], far, w, h, out);
}

fn invalid() -> EvalAltResult {
    EvalAltResult::ErrorRuntime(
        "context_partition expects up to 64 bounded integer counts, a nonnegative bounded integer limit and a positive finite viewport".into(),
        Position::NONE,
    )
}

fn projection(
    values: Array,
    limit: INT,
    width: f64,
    height: f64,
) -> Result<Map, Box<EvalAltResult>> {
    if values.len() > MAX_BLOCKS {
        return Err(invalid().into());
    }
    let counts = values
        .into_iter()
        .map(|value| value.as_int().map_err(|_| invalid()))
        .collect::<Result<Vec<_>, _>>()?;
    let partition = project(&counts, limit, width, height).ok_or_else(invalid)?;
    let tiles = partition
        .tiles
        .into_iter()
        .map(|tile| {
            let rect = tile.rect;
            let [left, top, tooltip_width, tooltip_height] = rect.tooltip([width, height]);
            Dynamic::from_map(Map::from_iter([
                (
                    "index".into(),
                    tile.block.map_or(Dynamic::UNIT, |index| {
                        Dynamic::from(INT::try_from(index).expect("bounded partition index"))
                    }),
                ),
                ("left".into(), Dynamic::from(rect.x0)),
                ("top".into(), Dynamic::from(rect.y0)),
                ("width".into(), Dynamic::from(rect.x1 - rect.x0)),
                ("height".into(), Dynamic::from(rect.y1 - rect.y0)),
                (
                    "tooltip".into(),
                    Dynamic::from_map(Map::from_iter([
                        ("left".into(), Dynamic::from(left)),
                        ("top".into(), Dynamic::from(top)),
                        ("width".into(), Dynamic::from(tooltip_width)),
                        ("max_height".into(), Dynamic::from(tooltip_height)),
                    ])),
                ),
            ]))
        })
        .collect();
    Ok(Map::from_iter([
        ("used".into(), Dynamic::from(partition.used)),
        ("free".into(), Dynamic::from(partition.free)),
        ("overflow".into(), Dynamic::from(partition.overflow)),
        ("tiles".into(), Dynamic::from_array(tiles)),
    ]))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("context_partition", projection);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_capacity::MAX_COUNT;

    fn area(r: Rect) -> f64 {
        (r.x1 - r.x0) * (r.y1 - r.y0)
    }

    fn overlap(a: Rect, b: Rect) -> f64 {
        (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0) * (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0)
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // Verify the independent area ratio in the presentation domain.
    fn tiles_cover_the_field_in_order_with_exact_relative_areas() {
        const EPS: f64 = 1e-12;
        let mut checked = 0;
        for (w, h) in [
            (60.0, 1200.0),
            (350.0, 254.0),
            (400.0, 400.0),
            (1800.0, 210.0),
        ] {
            for limit in [0, 82, 110, 131, 300] {
                for frame in 0_i64..=400 {
                    let weights = [
                        28 + frame / 7,
                        80 - frame / 5,
                        frame / 13,
                        12,
                        3 + frame / 67,
                    ];
                    let result = project(&weights, limit, w + frame as f64, h).unwrap();
                    let order = weights
                        .iter()
                        .enumerate()
                        .filter_map(|(i, n)| (*n > 0).then_some(Some(i)))
                        .chain((result.free > 0).then_some(None));
                    assert!(result.tiles.iter().map(|tile| tile.block).eq(order));
                    let denominator = result.used.max(limit) as f64;
                    assert!(
                        (result.tiles.iter().map(|t| area(t.rect)).sum::<f64>() - 1.0).abs() < EPS
                    );
                    for (i, tile) in result.tiles.iter().enumerate() {
                        let Rect { x0, y0, x1, y1 } = tile.rect;
                        assert!(0.0 <= x0 && x0 <= x1 && x1 <= 1.0);
                        assert!(0.0 <= y0 && y0 <= y1 && y1 <= 1.0);
                        let expected = tile.block.map_or(result.free, |j| weights[j]) as f64;
                        assert!((area(tile.rect) - expected / denominator).abs() < EPS);
                        for other in &result.tiles[i + 1..] {
                            assert!(overlap(tile.rect, other.rect) < EPS);
                        }
                    }
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 8_020);
    }

    #[test]
    fn empty_unknown_overfull_and_extreme_counts_have_distinct_meaning() {
        let empty = project(&[], 0, 1.0, 1.0).unwrap();
        assert_eq!(empty.tiles, [] as [Tile; 0]);
        let unused = project(&[0, 0], 100, 1.0, 1.0).unwrap();
        assert_eq!((unused.used, unused.free, unused.overflow), (0, 100, 0));
        assert_eq!(unused.tiles.len(), 1);
        assert_eq!(unused.tiles[0].block, None);
        let unknown = project(&[0, 5], 0, 1.0, 1.0).unwrap();
        assert_eq!((unknown.used, unknown.free, unknown.overflow), (5, 0, 0));
        assert_eq!(unknown.tiles[0].block, Some(1));
        let extreme = project(&[MAX_COUNT, 1, 0, 1], MAX_COUNT, 300.0, 150.0).unwrap();
        assert_eq!(
            (extreme.used, extreme.free, extreme.overflow),
            (MAX_COUNT + 2, 0, 2)
        );
        assert_eq!(extreme.tiles.len(), 3);
        assert!(project(&[0; MAX_BLOCKS + 1], 0, 1.0, 1.0).is_none());
        assert!(project(&[], 0, f64::INFINITY, 1.0).is_none());
        assert!(project(&[], 0, 1.0, 0.0).is_none());
        assert!(project(&[MAX_COUNT + 1], 0, 1.0, 1.0).is_none());
    }

    #[test]
    fn script_bridge_preserves_source_indexes_and_normalized_geometry() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    "let a = context_partition([0, 4], 8, 200.0, 100.0); \
                     let b = context_partition([0, 4], 8, 100.0, 200.0); \
                     a.used == 4 && a.free == 4 && a.overflow == 0 \
                     && a.tiles[0].index == 1 && a.tiles[1].index == () \
                     && a.tiles[1].left == 0.5 && b.tiles[1].top == 0.5"
                )
                .unwrap()
        );
        for bad in [
            "context_partition([1.5], 4, 100.0, 100.0)",
            "context_partition([-1], 4, 100.0, 100.0)",
            "context_partition([1], -1, 100.0, 100.0)",
            "context_partition([1], 4, 0.0, 100.0)",
        ] {
            assert!(engine.eval::<Dynamic>(bad).is_err(), "{bad}");
        }
    }
}
