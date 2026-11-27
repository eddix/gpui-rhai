//! Contrast evidence for the paint surfaces the shared Style API can express.
//!
//! Opaque two-stop sRGB gradients are affine color lines. Their luminance is
//! convex, so checking only endpoints can miss a dark interior. More complex
//! alpha/gradient stacks use a conservative RGB enclosure, never a claimed
//! exact contrast or an automatic accessibility exemption.

use crate::Rgba8;

type Rgb = [f64; 3];
const WEIGHTS: Rgb = [0.2126, 0.7152, 0.0722];

#[derive(Clone, Copy)]
enum Shape {
    Solid,
    Line,
    Box,
}

#[derive(Clone, Copy)]
pub(super) struct ContrastSurface {
    // Premultiplied sRGB. An isolated group's local destination starts clear,
    // so chroma alone is insufficient until it has been placed over its parent.
    from: Rgb,
    to: Rgb,
    alpha: [f64; 2],
    shape: Shape,
}

pub(super) struct Evidence {
    pub ratio: f64,
    pub conservative: bool,
}

impl ContrastSurface {
    /// A conservative enclosure for a stack of clipped premultiplied
    /// gradients. Apply element opacity once to the composed stack, exactly
    /// as the Metal layer primitive does.
    pub(super) fn layered<C: AsRef<[gpui::Rgba]>>(
        self,
        layers_bottom_up: impl Iterator<Item = (C, bool)>,
        opacity: f64,
    ) -> Self {
        if opacity == 0. {
            return self;
        }
        let (backing_low, backing_high) = self.channel_bounds();
        let [backing_alpha_min, backing_alpha_max] = self.alpha_bounds();
        let (mut low, mut high) = (backing_low, backing_high);
        let (mut alpha_min, mut alpha_max) = (backing_alpha_min, backing_alpha_max);
        for (stops, optional_coverage) in layers_bottom_up {
            let mut layer_low = if optional_coverage { low } else { [1.; 3] };
            let mut layer_high = if optional_coverage { high } else { [0.; 3] };
            let mut layer_alpha_min = if optional_coverage { alpha_min } else { 1. };
            let mut layer_alpha_max = if optional_coverage { alpha_max } else { 0. };
            for color in stops.as_ref() {
                let a = f64::from(color.a);
                // Premultiplied RGB and alpha interpolate together. At a
                // fixed backing bound, their over-composition is affine.
                // Taking RGB and alpha extrema independently admits paint
                // that cannot occur, e.g. black between two light layers.
                let channels = [color.r, color.g, color.b].map(f64::from);
                for i in 0..3 {
                    layer_low[i] = layer_low[i].min(channels[i] * a + low[i] * (1. - a));
                    layer_high[i] = layer_high[i].max(channels[i] * a + high[i] * (1. - a));
                }
                layer_alpha_min = layer_alpha_min.min(over_alpha(a, alpha_min));
                layer_alpha_max = layer_alpha_max.max(over_alpha(a, alpha_max));
            }
            low = layer_low;
            high = layer_high;
            alpha_min = layer_alpha_min;
            alpha_max = layer_alpha_max;
        }
        // Scaling the premultiplied group by opacity is equivalent to
        // interpolating backing -> fully composed result once.
        let from = std::array::from_fn(|i| low[i] * opacity + backing_low[i] * (1. - opacity));
        let to = std::array::from_fn(|i| high[i] * opacity + backing_high[i] * (1. - opacity));
        Self::surface(
            from,
            to,
            [
                backing_alpha_min * (1. - opacity) + alpha_min * opacity,
                backing_alpha_max * (1. - opacity) + alpha_max * opacity,
            ],
            Shape::Box,
        )
    }

    pub fn solid(color: Rgba8) -> Self {
        let from = rgb(color);
        Self {
            from,
            to: from,
            alpha: [1.; 2],
            shape: Shape::Solid,
        }
    }

    pub(super) fn transparent() -> Self {
        Self::surface([0.; 3], [0.; 3], [0.; 2], Shape::Solid)
    }

    /// Transform the isolated sRGB source, never its parent. Every matrix
    /// coefficient is nonnegative, so channel bounds are still conservative.
    pub(super) fn grayscale(self, amount: f64) -> Self {
        if amount == 0. {
            return self;
        }
        let transform = |channels: Rgb| {
            let gray = channels.into_iter().zip(WEIGHTS).map(|(c, w)| c * w).sum();
            mix(channels, [gray; 3], amount)
        };
        Self::surface(
            transform(self.from),
            transform(self.to),
            self.alpha,
            self.shape,
        )
    }

    pub fn over_with_opacity(self, color: Rgba8, opacity: f64) -> Self {
        let alpha = alpha(color);
        let foreground = rgb(color).map(|channel| channel * alpha);
        Self::surface(foreground, foreground, [alpha; 2], Shape::Solid)
            .composite_over(self, opacity)
    }

    pub(super) fn composite_over(self, backing: Self, opacity: f64) -> Self {
        if opacity == 0. || self.alpha_bounds()[1] == 0. {
            return backing;
        }
        let project = |fg: Rgb, alpha, bg: Rgb| {
            std::array::from_fn(|channel| {
                opacity * fg[channel] + (1. - opacity * alpha) * bg[channel]
            })
        };
        let exact_shape = match (self.shape, backing.shape) {
            (Shape::Solid, shape) => Some(shape),
            (Shape::Line, Shape::Solid) => Some(Shape::Line),
            _ => None,
        };
        if let Some(shape) = exact_shape {
            return Self::surface(
                project(self.from, self.alpha[0], backing.from),
                project(self.to, self.alpha[1], backing.to),
                [
                    over_alpha(opacity * self.alpha[0], backing.alpha[0]),
                    over_alpha(opacity * self.alpha[1], backing.alpha[1]),
                ],
                shape,
            );
        }
        let (min, max) = self.channel_bounds();
        let (behind_min, behind_max) = backing.channel_bounds();
        let [alpha_min, alpha_max] = self.alpha_bounds();
        let [behind_alpha_min, behind_alpha_max] = backing.alpha_bounds();
        Self::surface(
            project(min, alpha_max, behind_min),
            project(max, alpha_min, behind_max),
            [
                over_alpha(opacity * alpha_min, behind_alpha_min),
                over_alpha(opacity * alpha_max, behind_alpha_max),
            ],
            Shape::Box,
        )
    }

    #[cfg(test)]
    pub fn gradient(self, from: Rgba8, to: Rgba8) -> Self {
        self.gradient_with_opacity(from, to, 1.0)
    }

    pub fn gradient_with_opacity(self, from: Rgba8, to: Rgba8, opacity: f64) -> Self {
        if from == to {
            return self.over_with_opacity(from, opacity);
        }
        let a = alpha(from) * opacity;
        let b = alpha(to) * opacity;
        if a >= 1.0 && b >= 1.0 {
            return Self {
                from: rgb(from),
                to: rgb(to),
                alpha: [1.; 2],
                shape: Shape::Line,
            };
        }
        if matches!(self.shape, Shape::Solid) && (a - b).abs() < f64::EPSILON {
            let placed_alpha = over_alpha(a, self.alpha[0]);
            return Self::surface(
                mix(self.from, rgb(from), a),
                mix(self.from, rgb(to), a),
                [placed_alpha; 2],
                Shape::Line,
            );
        }
        // The GPU interpolates straight sRGB + alpha before compositing. The
        // resulting channel is multi-affine in foreground, alpha and backing
        // color; extrema over their independent ranges occur at box corners.
        let (parent_low, parent_high) = self.channel_bounds();
        let from = rgb(from);
        let to = rgb(to);
        let mut low = [1.0; 3];
        let mut high = [0.0; 3];
        for channel in 0..3 {
            for foreground in [from[channel], to[channel]] {
                for opacity in [a, b] {
                    for parent in [parent_low[channel], parent_high[channel]] {
                        let value = parent * (1.0 - opacity) + foreground * opacity;
                        low[channel] = f64::min(low[channel], value);
                        high[channel] = f64::max(high[channel], value);
                    }
                }
            }
        }
        let [alpha_min, alpha_max] = self.alpha_bounds();
        Self::surface(
            low,
            high,
            [
                over_alpha(a.min(b), alpha_min),
                over_alpha(a.max(b), alpha_max),
            ],
            Shape::Box,
        )
    }

    #[cfg(test)]
    pub fn contrast(self, text: Rgba8) -> Evidence {
        self.contrast_with_opacity(text, 1.0)
    }

    pub fn solid_srgb(self) -> Option<[f64; 3]> {
        (matches!(self.shape, Shape::Solid) && self.alpha[0].to_bits() == 1.0_f64.to_bits())
            .then_some(self.from)
    }

    pub(super) fn contrast_against(self, ink: Self) -> Evidence {
        Evidence {
            ratio: range_ratio(self.luminance_bounds(), ink.luminance_bounds()),
            conservative: !matches!((self.shape, ink.shape), (Shape::Solid, Shape::Solid)),
        }
    }

    pub fn contrast_with_opacity(self, text: Rgba8, element_opacity: f64) -> Evidence {
        let foreground = rgb(text);
        let opacity = alpha(text) * element_opacity;
        if matches!(self.shape, Shape::Solid) {
            return Evidence {
                ratio: ratio(
                    luminance(mix(self.from, foreground, opacity)),
                    luminance(self.from),
                ),
                conservative: false,
            };
        }
        let background = self.luminance_bounds();
        if opacity >= 1.0 {
            return Evidence {
                ratio: range_ratio(background, (luminance(foreground), luminance(foreground))),
                conservative: matches!(self.shape, Shape::Box),
            };
        }
        let (low, high) = self.channel_bounds();
        let painted = (
            luminance(mix(low, foreground, opacity)),
            luminance(mix(high, foreground, opacity)),
        );
        let independent = range_ratio(background, painted);
        Evidence {
            ratio: paired_corner_ratio(low, high, foreground, opacity)
                .map_or(independent, |paired| independent.max(paired)),
            conservative: true,
        }
    }

    fn channel_bounds(self) -> (Rgb, Rgb) {
        (
            std::array::from_fn(|i| self.from[i].min(self.to[i])),
            std::array::from_fn(|i| self.from[i].max(self.to[i])),
        )
    }

    fn alpha_bounds(self) -> [f64; 2] {
        [
            self.alpha[0].min(self.alpha[1]),
            self.alpha[0].max(self.alpha[1]),
        ]
    }

    fn surface(mut from: Rgb, mut to: Rgb, alpha: [f64; 2], shape: Shape) -> Self {
        if matches!(shape, Shape::Box) {
            let maximum = alpha[0].max(alpha[1]);
            // Independent upper RGB and alpha bounds can otherwise invent
            // a premultiplied channel brighter than the greatest coverage.
            from = from.map(|channel| channel.clamp(0., maximum));
            to = to.map(|channel| channel.clamp(0., maximum));
        }
        let shape = if from.map(f64::to_bits) == to.map(f64::to_bits)
            && alpha[0].to_bits() == alpha[1].to_bits()
        {
            Shape::Solid
        } else {
            shape
        };
        Self {
            from,
            to,
            alpha,
            shape,
        }
    }

    fn luminance_bounds(self) -> (f64, f64) {
        match self.shape {
            Shape::Solid => (luminance(self.from), luminance(self.from)),
            Shape::Box => (luminance(self.from), luminance(self.to)),
            Shape::Line => {
                let mut low = 0.0;
                let mut high = 1.0;
                // sRGB decode is convex, so the derivative is monotone along
                // an affine RGB segment. Bisection finds its unique minimum
                // (or the appropriate endpoint), without sampling past it.
                for _ in 0..48 {
                    let t = f64::midpoint(low, high);
                    let value = mix(self.from, self.to, t);
                    let derivative: f64 = (0..3)
                        .map(|i| {
                            WEIGHTS[i] * decode_derivative(value[i]) * (self.to[i] - self.from[i])
                        })
                        .sum();
                    if derivative < 0.0 {
                        low = t;
                    } else {
                        high = t;
                    }
                }
                (
                    luminance(mix(self.from, self.to, f64::midpoint(low, high))),
                    luminance(self.from).max(luminance(self.to)),
                )
            }
        }
    }
}

/// Bound the ratio over the RGB enclosure while keeping the ink and its
/// actual destination paired. This can be tighter than dividing unrelated
/// extrema (e.g. dark paper behind the glyph, but a light stripe behind
/// that same paper). The enclosure can still include unreachable colors.
fn paired_corner_ratio(low: Rgb, high: Rgb, foreground: Rgb, opacity: f64) -> Option<f64> {
    let corner = if (0..3).all(|i| foreground[i] >= high[i]) {
        high
    } else if (0..3).all(|i| foreground[i] <= low[i]) {
        low
    } else {
        return None;
    };
    let ink_low = mix(low, foreground, opacity);
    let ink_high = mix(high, foreground, opacity);
    // The signed derivative of (ink luminance + .05) / (paper
    // luminance + .05) is nonpositive in each channel if this
    // conservative bound holds. The decode derivative is nondecreasing.
    // A dark foreground uses the reciprocal, hence the other corner.
    let crosses_knee = |lo, hi| lo <= 0.04045 && hi > 0.04045;
    // The standardized cutoff is rounded: don't infer monotonicity
    // across its tiny discontinuity from a derivative bound alone.
    if (0..3).any(|i| crosses_knee(low[i], high[i]) || crosses_knee(ink_low[i], ink_high[i])) {
        return None;
    }
    let decreases = |lo: Rgb, hi: Rgb, back_ink_lo: Rgb, back_ink_hi: Rgb| {
        let max_back = luminance(hi) + 0.05;
        let min_ink = luminance(back_ink_lo) + 0.05;
        (0..3).all(|i| {
            (1.0 - opacity) * decode_derivative(back_ink_hi[i]) * max_back + 1e-12
                <= decode_derivative(lo[i]) * min_ink
        })
    };
    if !decreases(low, high, ink_low, ink_high) {
        // Small tints can be too wide for a single independent derivative
        // bound (especially dark captions on light paper). Certify up to
        // eight smaller boxes; avoid this extra work on broad surfaces.
        if (0..3).any(|i| high[i] - low[i] > 0.08) {
            return None;
        }
        let mid = mix(low, high, 0.5);
        for part in 0..8 {
            let lo = std::array::from_fn(|i| if part & (1 << i) == 0 { low[i] } else { mid[i] });
            let hi = std::array::from_fn(|i| {
                if part & (1 << i) == 0 {
                    mid[i]
                } else {
                    high[i]
                }
            });
            if !decreases(
                lo,
                hi,
                mix(lo, foreground, opacity),
                mix(hi, foreground, opacity),
            ) {
                return None;
            }
        }
    }
    Some(ratio(
        luminance(mix(corner, foreground, opacity)),
        luminance(corner),
    ))
}

fn over_alpha(front: f64, back: f64) -> f64 {
    1. - (1. - front) * (1. - back)
}

fn rgb(color: Rgba8) -> Rgb {
    let [red, green, blue, _] = color.as_rgba_hex().to_be_bytes();
    [red, green, blue].map(|channel| f64::from(channel) / 255.0)
}

fn alpha(color: Rgba8) -> f64 {
    f64::from(color.as_rgba_hex().to_be_bytes()[3]) / 255.0
}
fn mix(from: Rgb, to: Rgb, t: f64) -> Rgb {
    std::array::from_fn(|i| from[i] * (1.0 - t) + to[i] * t)
}
fn decode(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
fn decode_derivative(value: f64) -> f64 {
    if value <= 0.04045 {
        1.0 / 12.92
    } else {
        (2.4 / 1.055) * ((value + 0.055) / 1.055).powf(1.4)
    }
}
fn luminance(color: Rgb) -> f64 {
    (0..3).map(|i| WEIGHTS[i] * decode(color[i])).sum()
}
fn ratio(left: f64, right: f64) -> f64 {
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}
fn range_ratio(left: (f64, f64), right: (f64, f64)) -> f64 {
    if left.0 > right.1 {
        ratio(left.0, right.1)
    } else if right.0 > left.1 {
        ratio(right.0, left.1)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(value: u32) -> Rgba8 {
        Rgba8::from_rgba_hex(value)
    }

    #[test]
    fn translucent_ink_stays_paired_with_its_light_or_dark_hatch() {
        for (plate, tint, label) in [
            (0x2323_26ff, 0xffff_ff0a, 0x8a8a_8aff),
            (0xffff_ffff, 0x0000_0009, 0x0000_0073),
        ] {
            let striped = ContrastSurface::solid(color(plate)).layered(
                [([gpui::rgba(0x0000_0000), gpui::rgba(tint)], false)].into_iter(),
                1.0,
            );
            let (low, high) = striped.channel_bounds();
            let ink = rgb(color(label));
            let opacity = alpha(color(label)) * 0.7;
            let render_ratio = |paper| ratio(luminance(mix(paper, ink, opacity)), luminance(paper));
            let paired = render_ratio(low).min(render_ratio(high));
            let result = striped.contrast_with_opacity(color(label), 0.7);
            assert!(
                result.conservative,
                "the RGB enclosure is not a solid witness"
            );
            assert!(
                (result.ratio - paired).abs() < 1e-12,
                "{label:x}: {}",
                result.ratio
            );
            assert!(result.ratio < 4.5, "the real risk must remain");
        }
    }

    #[test]
    fn paired_ratio_never_exceeds_any_sampled_color_in_the_enclosure() {
        for (lo, hi) in [
            ([0.03, 0.034, 0.045], [0.052, 0.091, 0.13]),
            ([0.11, 0.12, 0.13], [0.25, 0.24, 0.22]),
            ([0.79, 0.88, 0.80], [0.90, 0.96, 0.94]),
            ([0.11, 0.32, 0.31], [0.72, 0.76, 0.59]),
        ] {
            let surface = ContrastSurface::surface(lo, hi, [1.; 2], Shape::Box);
            for ink in [0x4823_78ff, 0x0000_00ff, 0xffff_ffff, 0x6666_66ff] {
                let ink = color(ink);
                for opacity in [0.1, 0.45, 0.7, 0.95] {
                    let estimate = surface.contrast_with_opacity(ink, opacity).ratio;
                    for r in 0..=4 {
                        for g in 0..=4 {
                            for b in 0..=4 {
                                let indices = [r, g, b];
                                let paper = std::array::from_fn(|i| {
                                    lo[i] + (hi[i] - lo[i]) * f64::from(indices[i]) / 4.
                                });
                                let nominal = mix(paper, rgb(ink), opacity);
                                assert!(
                                    estimate <= ratio(luminance(paper), luminance(nominal)) + 1e-12,
                                    "{paper:?} {ink:?} opacity {opacity}: {estimate}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn layered_envelope_correlates_alpha_and_applies_stack_opacity_once() {
        // A translucent layer over light opaque stops cannot invent dark
        // pixels, even when its rounded padding clip has incomplete coverage.
        let backing = ContrastSurface::solid(color(0x1010_10ff));
        let layers = [
            (
                vec![gpui::rgba(0xc0c0_c0ff), gpui::rgba(0xffff_ffff)],
                false,
            ),
            (vec![gpui::rgba(0xb0b0_b000), gpui::rgba(0xeeee_eeff)], true),
        ];
        let result = backing.layered(layers.into_iter(), 1.);
        let (low, high) = result.channel_bounds();
        assert!(low.into_iter().all(|v| v >= 0.75));
        assert_eq!(high, [1.; 3]);

        // Full stack opaque red, atop opaque blue, atop black. At 50% the
        // backing contributes black, not blue from inside the same stack.
        let half = ContrastSurface::solid(color(0x0000_00ff)).layered(
            [
                (vec![gpui::rgba(0x0000_ffff)], false),
                (vec![gpui::rgba(0xff00_00ff)], false),
            ]
            .into_iter(),
            0.5,
        );
        assert_eq!(half.solid_srgb(), Some([0.5, 0., 0.]));
    }

    #[test]
    fn isolated_translucent_gradient_bounds_include_composited_samples() {
        let foreground = color(0xa8ef_e5d0);
        let inner = [(0x2255_aa40, 0xaa22_77cc), (0x6611_4420, 0x77aa_bb88)];
        let outer = [color(0x348a_46a0), color(0x6aa2_5458)];
        let canvas = color(0x1616_1aff);
        let base = ContrastSurface::solid(canvas);
        let outside = ContrastSurface::transparent().gradient(outer[0], outer[1]);
        let inside = ContrastSurface::transparent().layered(
            inner
                .into_iter()
                .map(|(a, b)| (vec![gpui::rgba(a), gpui::rgba(b)], false)),
            1.,
        );
        let paper = inside
            .composite_over(outside, 0.60)
            .composite_over(base, 0.74);
        let ink = inside
            .over_with_opacity(foreground, 0.83)
            .composite_over(outside, 0.60)
            .composite_over(base, 0.74);
        let evidence = paper.contrast_against(ink);
        assert!(evidence.conservative);

        // Resolve translucent layers independently at sampled points.
        let clear = ([0.; 3], 0.);
        let compose = |(p, a): (Rgb, f64), (behind, b_a): (Rgb, f64), weight| {
            (
                std::array::from_fn(|i| p[i] * weight + behind[i] * (1. - weight * a)),
                a * weight + b_a * (1. - weight * a),
            )
        };
        let premul = |rgba| (rgb(rgba).map(|c| c * alpha(rgba)), alpha(rgba));
        let lerp = |(p, a): (Rgb, f64), (q, b): (Rgb, f64), t| (mix(p, q, t), a + (b - a) * t);
        for a in 0..=16 {
            for b in 0..=16 {
                for c in 0..=16 {
                    let (a, b, c) = (f64::from(a) / 16., f64::from(b) / 16., f64::from(c) / 16.);
                    let lower = lerp(premul(color(inner[0].0)), premul(color(inner[0].1)), a);
                    let upper = lerp(premul(color(inner[1].0)), premul(color(inner[1].1)), b);
                    let outside_ink = mix(rgb(outer[0]), rgb(outer[1]), c);
                    let outside_alpha = alpha(outer[0]) * (1. - c) + alpha(outer[1]) * c;
                    let behind = (outside_ink.map(|v| v * outside_alpha), outside_alpha);
                    let under = compose(lower, clear, 1.);
                    let background = compose(upper, under, 1.);
                    let with_ink = compose(premul(foreground), background, 0.83);
                    let place =
                        |local| compose(compose(local, behind, 0.60), (rgb(canvas), 1.), 0.74).0;
                    let (sampled_paper, sampled_ink) = (place(background), place(with_ink));
                    assert!(
                        evidence.ratio
                            <= ratio(luminance(sampled_paper), luminance(sampled_ink)) + 1e-9
                    );
                    let (lo, hi) = paper.channel_bounds();
                    for axis in 0..3 {
                        assert!((lo[axis] - 1e-9..=hi[axis] + 1e-9).contains(&sampled_paper[axis]));
                    }
                }
            }
        }
    }

    #[test]
    fn dark_gradients_are_not_their_light_parent_and_light_gradients_still_fail() {
        let white = color(0xffff_ffff);
        let backing = ContrastSurface::solid(white);
        assert!(
            backing
                .gradient(color(0x1f45_b8ff), color(0x173a_acff))
                .contrast(white)
                .ratio
                > 4.5
        );
        assert!(
            backing
                .gradient(color(0xeeee_eeff), white)
                .contrast(white)
                .ratio
                < 1.2
        );
        assert!(
            backing
                .gradient(color(0x0000_00ff), white)
                .contrast(white)
                .ratio
                < 1.01
        );
    }

    #[test]
    fn the_gradient_interior_can_be_darker_than_both_stops() {
        let from = color(0xff00_00ff);
        let to = color(0x00ff_00ff);
        let surface = ContrastSurface::solid(color(0xffff_ffff)).gradient(from, to);
        let result = surface.contrast(color(0x0000_00ff));
        assert!(ratio(luminance(rgb(from)), 0.0) > 4.5);
        assert!(ratio(luminance(rgb(to)), 0.0) > 4.5);
        assert!(
            result.ratio < 4.5,
            "endpoint-only checking would wrongly pass"
        );
        let sampled = (0..=10_000)
            .map(|i| {
                ratio(
                    luminance(mix(rgb(from), rgb(to), f64::from(i) / 10_000.0)),
                    0.0,
                )
            })
            .reduce(f64::min)
            .unwrap();
        assert!((sampled - result.ratio).abs() < 1e-6);
    }

    #[test]
    fn transparent_gradient_stacks_are_bounded_instead_of_claimed_exact() {
        let base = color(0x1717_17ff);
        let first = [color(0x1122_4488), color(0x8866_22dd)];
        let second = [color(0x3344_1122), color(0x2288_77aa)];
        let text = color(0xffff_ffbb);
        let result = ContrastSurface::solid(base)
            .gradient(first[0], first[1])
            .gradient(second[0], second[1])
            .contrast(text);
        assert!(result.conservative);
        for t in 0..=20 {
            for u in 0..=20 {
                let t = f64::from(t) / 20.0;
                let u = f64::from(u) / 20.0;
                let background = mix(
                    rgb(base),
                    mix(rgb(first[0]), rgb(first[1]), t),
                    alpha(first[0]) * (1.0 - t) + alpha(first[1]) * t,
                );
                let background = mix(
                    background,
                    mix(rgb(second[0]), rgb(second[1]), u),
                    alpha(second[0]) * (1.0 - u) + alpha(second[1]) * u,
                );
                let actual = ratio(
                    luminance(background),
                    luminance(mix(background, rgb(text), alpha(text))),
                );
                assert!(actual + 1e-9 >= result.ratio);
            }
        }
    }
}
