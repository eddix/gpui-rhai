//! Discrete presentation of one native `TextBuffer`. The separators never exist
//! in text, clipboard payloads or the UTF-16 domain.

use std::ops::Range;

use gpui::{
    App, AvailableSpace, Bounds, Hsla, LayoutId, Pixels, Point, ShapedLine, TextAlign, TextRun,
    UnderlineStyle, Window, fill, point, px, quad, rgba, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    LayoutLength, Length, PrimitiveProps, PrimitiveTheme, StyleProperties, text_edit::TextBuffer,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CodeSpec {
    pub(crate) length: usize,
    pub(crate) alphanumeric: bool,
}

impl CodeSpec {
    pub(crate) fn normalize(self, text: &str) -> String {
        text.chars()
            .take(4096)
            .filter_map(|ch| {
                let mapped = match ch {
                    '０'..='９' => b'0' + u8::try_from(u32::from(ch) - 0xff10).ok()?,
                    '٠'..='٩' => b'0' + u8::try_from(u32::from(ch) - 0x0660).ok()?,
                    '۰'..='۹' => b'0' + u8::try_from(u32::from(ch) - 0x06f0).ok()?,
                    'Ａ'..='Ｚ' => b'A' + u8::try_from(u32::from(ch) - 0xff21).ok()?,
                    'ａ'..='ｚ' => b'A' + u8::try_from(u32::from(ch) - 0xff41).ok()?,
                    ascii if ascii.is_ascii_alphanumeric() => {
                        u8::try_from(u32::from(ascii)).ok()?.to_ascii_uppercase()
                    }
                    _ => return None,
                };
                (mapped.is_ascii_digit() || self.alphanumeric).then_some(char::from(mapped))
            })
            .take(self.length)
            .collect()
    }

    pub(crate) fn select(self, buffer: &mut TextBuffer, byte: usize) {
        let cell = byte.min(buffer.content.len()).min(self.length - 1);
        buffer.move_to(cell);
        if cell < buffer.content.len() {
            buffer.select_to(buffer.next_boundary(cell));
        }
    }

    pub(crate) fn replace(
        self,
        buffer: &mut TextBuffer,
        range_utf16: Option<Range<usize>>,
        text: &str,
        pasted: bool,
    ) {
        let clean = self.normalize(text);
        if !text.is_empty() && clean.is_empty() {
            buffer.cancel_composition();
            return;
        }
        let range = if pasted && clean.len() == self.length {
            Some(0..buffer.offset_to_utf16(buffer.content.len()))
        } else if pasted && buffer.marked().is_none() && buffer.selected.len() <= 1 {
            let start = buffer.selected.start;
            let end = (start + clean.len()).min(buffer.content.len());
            Some(buffer.range_to_utf16(&(start..end)))
        } else {
            range_utf16
        };
        if range.is_none()
            && buffer
                .marked()
                .is_some_and(|mark| buffer.content.get(mark.clone()) == Some(clean.as_str()))
        {
            buffer.unmark();
        } else {
            buffer.replace(range.as_ref(), &clean);
        }
        self.select(buffer, buffer.cursor_offset());
    }
}

#[derive(Clone)]
struct SlotSkin {
    fill: Hsla,
    edge: Hsla,
    edge_width: Pixels,
    radius: Pixels,
    shadows: gpui::ShadowStack,
}

impl SlotSkin {
    fn resolve(
        style: &StyleProperties,
        theme: &PrimitiveTheme,
        rem: Pixels,
        fallback_fill: u32,
    ) -> Self {
        let fill = style
            .background
            .as_deref()
            .and_then(crate::BackgroundFill::as_solid_value)
            .and_then(|value| theme.resolve_color(value))
            .map_or(fallback_fill, crate::Rgba8::as_rgba_hex);
        let edge = style
            .border_color
            .as_ref()
            .and_then(|ink| ink.native_ink(&mut |value| theme.resolve_color(value)))
            .map_or_else(
                || rgba(0x8a8a_8a40).into(),
                |ink| ink.resolve_hsla(rgba(0x1515_15ff).into()),
            );
        Self {
            fill: rgba(fill).into(),
            edge,
            edge_width: length(theme, style.border_widths.top.as_ref(), rem).unwrap_or(px(1.)),
            radius: length(theme, style.radii.top_left.as_ref(), rem).unwrap_or(px(9.)),
            shadows: crate::renderer::resolve_shadows(
                style.shadows.as_deref().unwrap_or(&[]),
                theme,
            ),
        }
    }

    fn paint(&self, bounds: Bounds<Pixels>, scale: f32, window: &mut Window) {
        self.paint_radius(bounds, self.radius * scale, window);
    }

    fn paint_radius(&self, bounds: Bounds<Pixels>, radius: Pixels, window: &mut Window) {
        let corners = gpui::Corners::all(radius);
        let shadows = self.shadows.resolve(|| window.text_style().color);
        window.paint_drop_shadows_with_clip(
            bounds,
            corners,
            &shadows,
            gpui::OuterShadowClip::BorderBox,
        );
        window.paint_quad(quad(
            bounds,
            corners,
            self.fill,
            gpui::Edges::all(self.edge_width),
            self.edge,
            gpui::BorderStyle::default(),
        ));
        window.paint_inset_shadows(bounds, corners, &shadows);
    }
}

fn length(theme: &PrimitiveTheme, value: Option<&Length>, rem: Pixels) -> Option<Pixels> {
    match theme.resolve_length(*value?)? {
        Length::Pixels(value) => Some(px(crate::renderer::f64_to_f32(value))),
        Length::Rems(value) => Some(rem * crate::renderer::f64_to_f32(value)),
        Length::Relative(_) | Length::Token(_) => None,
    }
}

fn dimension(theme: &PrimitiveTheme, value: Option<&LayoutLength>, rem: Pixels) -> Option<Pixels> {
    let LayoutLength::Definite(value) = value? else {
        return None;
    };
    length(theme, Some(value), rem)
}

#[derive(Clone)]
pub(crate) struct CodePresentation {
    pub(crate) spec: CodeSpec,
    separator_at: Option<usize>,
    centered: bool,
    side: Pixels,
    height: Pixels,
    gap: Pixels,
    separator_width: Pixels,
    separator_height: Pixels,
    separator_color: Hsla,
    digit_size: Pixels,
    empty: SlotSkin,
    filled: SlotSkin,
    active: SlotSkin,
    caret: Hsla,
    selection: Hsla,
    shimmer_phase: Option<f32>,
    shimmer_color: gpui::Rgba,
    success: Option<SuccessSkin>,
    discard_phase: Option<f32>,
}

#[derive(Clone)]
struct SuccessSkin {
    label: String,
    verified: bool,
    skin: SlotSkin,
    height: Pixels,
    size: Pixels,
    leading: Pixels,
    trailing: Pixels,
    gap: Pixels,
    icon_size: Pixels,
    icon_color: Hsla,
    ink: Option<Hsla>,
    phase: f32,
}

impl SuccessSkin {
    fn resolve(props: &PrimitiveProps, theme: &PrimitiveTheme, window: &Window) -> Option<Self> {
        let label = props.string("code_success_label")?;
        let rem = window.rem_size();
        let default = crate::Style::new();
        let style = &props.style("code_success_style").unwrap_or(&default).base;
        let icon = &props
            .style("code_success_icon_style")
            .unwrap_or(&default)
            .base;
        let solid = |name| {
            theme
                .part_color(props, name, false)
                .map(|color| Hsla::from(rgba(color.as_rgba_hex())))
        };
        Some(Self {
            label: label.to_owned(),
            verified: props.boolean("code_verified").unwrap_or(false),
            skin: SlotSkin::resolve(style, theme, rem, 0xffff_ffff),
            height: dimension(theme, style.height.as_ref(), rem).unwrap_or(px(36.)),
            size: length(theme, style.font_size.as_ref(), rem).unwrap_or(px(14.)),
            leading: length(theme, style.padding.left.as_ref(), rem).unwrap_or(px(12.)),
            trailing: length(theme, style.padding.right.as_ref(), rem).unwrap_or(px(15.)),
            gap: length(theme, style.gap.as_ref(), rem).unwrap_or(px(6.)),
            icon_size: dimension(theme, icon.width.as_ref(), rem).unwrap_or(px(17.)),
            icon_color: solid("code_success_icon_style")
                .unwrap_or_else(|| rgba(0x1e8a_4fff).into()),
            ink: solid("code_success_style"),
            phase: theme
                .motion_progress()
                .map_or(1., crate::renderer::f64_to_f32),
        })
    }

    fn shape(&self, available: Pixels, window: &mut Window) -> Option<ShapedLine> {
        let style = window.text_style();
        let run = |text: &str| TextRun {
            len: text.len(),
            font: style.font(),
            letter_spacing: style.letter_spacing,
            color: self.ink.unwrap_or(style.color),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        // Bound presentation work for a localized/custom label.
        let label = self
            .label
            .graphemes(true)
            .take(128)
            .collect::<String>()
            .replace(['\n', '\r'], " ");
        let shape = |text: &str, window: &mut Window| {
            window
                .text_system()
                .shape_line(text.to_owned().into(), self.size, &[run(text)], None)
        };
        let line = shape(&label, window);
        if line.width <= available {
            return Some(line);
        }
        let ellipsis = shape("…", window);
        if ellipsis.width > available {
            return None;
        }
        let boundaries = label
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        // Find the longest fitting ellipsis label through the active shaper.
        let (mut low, mut high) = (0, boundaries.len());
        let mut best = ellipsis;
        while low < high {
            let mid = usize::midpoint(low, high);
            let line = shape(&format!("{}…", &label[..boundaries[mid]]), window);
            if line.width <= available {
                best = line;
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        Some(best)
    }

    fn frame(
        &self,
        cells: &[CodeCell],
        bounds: Bounds<Pixels>,
        window: &mut Window,
    ) -> Option<SuccessFrame> {
        let first = cells.first()?.bounds;
        let last = cells.last()?.bounds;
        let envelope = Bounds::from_corners(first.origin, last.bottom_right());
        let reserve = self.leading + self.icon_size + self.gap + self.trailing;
        let label = self.shape((bounds.size.width - reserve).max(px(0.)), window);
        Some(SuccessFrame {
            envelope,
            label,
            available: bounds,
        })
    }
}

impl CodePresentation {
    pub(crate) fn is_success(&self) -> bool {
        self.success.as_ref().is_some_and(|skin| skin.verified)
    }

    pub(crate) fn resolve(
        props: &PrimitiveProps,
        theme: &PrimitiveTheme,
        window: &Window,
    ) -> Option<Self> {
        let count = props
            .usize("code_length")
            .filter(|n| (1..=24).contains(n))?;
        let rem = window.rem_size();
        let blank = crate::Style::new();
        let style = |name| props.style(name).unwrap_or(&blank);
        let cell = style("code_cell_style");
        let filled = cell.clone().merged(style("code_filled_style"));
        let active = style("code_active_style");
        let separator = &style("code_separator_style").base;
        let side = dimension(theme, cell.base.width.as_ref(), rem)
            .unwrap_or(px(48.))
            .clamp(px(12.), px(160.));
        let color = |name: &str, fallback| {
            rgba(
                theme
                    .part_color(props, name, false)
                    .map_or(fallback, crate::Rgba8::as_rgba_hex),
            )
            .into()
        };
        Some(Self {
            spec: CodeSpec {
                length: count,
                alphanumeric: props.boolean("code_alphanumeric").unwrap_or(false),
            },
            separator_at: props
                .usize("code_separator")
                .filter(|i| (1..count).contains(i)),
            centered: props.boolean("code_centered").unwrap_or(false),
            side,
            height: dimension(theme, cell.base.height.as_ref(), rem).unwrap_or(side * 1.18),
            digit_size: length(theme, cell.base.font_size.as_ref(), rem).unwrap_or(side * 0.46),
            gap: length(theme, cell.base.gap.as_ref(), rem).unwrap_or(px(8.)),
            separator_width: dimension(theme, separator.width.as_ref(), rem).unwrap_or(px(10.)),
            separator_height: dimension(theme, separator.height.as_ref(), rem).unwrap_or(px(2.)),
            separator_color: color("code_separator_style", 0xc4c4_c8ff),
            empty: SlotSkin::resolve(&cell.base, theme, rem, 0xfafa_faff),
            filled: SlotSkin::resolve(&filled.base, theme, rem, 0xffff_ffff),
            active: SlotSkin::resolve(&active.base, theme, rem, 0x0000_0000),
            caret: color("code_caret_style", 0x1111_11ff),
            selection: rgba(
                theme
                    .color("selection")
                    .map_or(0x4466_8828, crate::Rgba8::as_rgba_hex),
            )
            .into(),
            shimmer_phase: (theme.motion_preference() == crate::MotionPreference::Normal
                && props.boolean("code_verified") != Some(true)
                && props.boolean("code_discarding") != Some(true))
            .then(|| theme.motion_progress().map(crate::renderer::f64_to_f32))
            .flatten(),
            shimmer_color: rgba(
                theme
                    .color("otp.shine")
                    .map_or(0xffff_ffd1, crate::Rgba8::as_rgba_hex),
            ),
            success: SuccessSkin::resolve(props, theme, window),
            discard_phase: (theme.motion_preference() == crate::MotionPreference::Normal
                && props.boolean("code_discarding") == Some(true))
            .then(|| theme.motion_progress().map(crate::renderer::f64_to_f32))
            .flatten(),
        })
    }

    fn metrics(&self, width: Pixels) -> (Pixels, Pixels, Pixels) {
        let count = f32::from(u16::try_from(self.spec.length).unwrap_or(24));
        let has_separator = self.separator_at.is_some();
        let gap_count = if has_separator { count } else { count - 1. };
        let full = self.side * count
            + self.gap * gap_count
            + if has_separator {
                self.separator_width
            } else {
                px(0.)
            };
        let natural_gap = if width >= full {
            self.gap
        } else if width < px(250.) {
            self.gap.min(px(4.))
        } else if width < px(360.) {
            self.gap.min(px(6.))
        } else {
            self.gap
        };
        // Treat spacing as the flexible part before squeezing a long code.
        let gap = natural_gap.min(width / (count * 4.));
        let sep = if has_separator {
            self.separator_width.min(width / (count * 2.))
        } else {
            px(0.)
        };
        let side = self
            .side
            .min(((width - sep - gap * gap_count) / count).max(px(0.)));
        (side, gap, sep)
    }

    pub(crate) fn request_layout(&self, window: &mut Window) -> LayoutId {
        let config = self.clone();
        let count = f32::from(u16::try_from(config.spec.length).unwrap_or(24));
        let extra = config
            .separator_at
            .map_or(px(0.), |_| config.separator_width + config.gap);
        let slots = config.side * count + config.gap * (count - 1.) + extra;
        let natural = config.success.as_ref().map_or(slots, |success| {
            let reserve = success.leading + success.icon_size + success.gap + success.trailing;
            let label = success
                .shape(px(4096.), window)
                .map_or(px(0.), |line| line.width);
            slots.max(reserve + label)
        });
        window.request_measured_layout(gpui::Style::default(), move |known, available, _, _| {
            let width = known.width.unwrap_or_else(|| match available.width {
                AvailableSpace::Definite(width) => natural.min(width),
                AvailableSpace::MinContent => px(0.),
                AvailableSpace::MaxContent => natural,
            });
            let (side, _, _) = config.metrics(width);
            let slots_height = config.height * (side / config.side);
            let minimum = config
                .success
                .as_ref()
                .map_or(px(0.), |pill| pill.height.min(width));
            size(width, known.height.unwrap_or(slots_height.max(minimum)))
        })
    }

    pub(crate) fn frame(
        &self,
        bounds: Bounds<Pixels>,
        buffer: &TextBuffer,
        focused: bool,
        interactive: bool,
        window: &mut Window,
    ) -> CodeFrame {
        let (side, gap, separator_width) = self.metrics(bounds.size.width);
        if side <= px(0.) || bounds.size.height <= px(0.) {
            return CodeFrame {
                cells: Vec::new(),
                divider: None,
                scale: 0.,
                focused,
                ring_position: None,
                pill: None,
                discard_phase: None,
                caret_alpha: 0.,
            };
        }
        let scale = side / self.side;
        let height = (self.height * scale).min(bounds.size.height);
        let top = bounds.top() + (bounds.size.height - height) / 2.;
        let count = f32::from(u16::try_from(self.spec.length).unwrap_or(24));
        let used = side * count
            + gap * (count - 1.)
            + if self.separator_at.is_some() {
                separator_width + gap
            } else {
                px(0.)
            };
        let mut left = bounds.left()
            + if self.centered {
                (bounds.size.width - used).max(px(0.)) / 2.
            } else {
                px(0.)
            };
        let mut divider = None;
        let mut glyphs = buffer.content.grapheme_indices(true);
        let mut cells = Vec::with_capacity(self.spec.length);
        let style = window.text_style();
        let selection = buffer.selection();
        let active_byte = if selection.len() <= 1 {
            selection.start
        } else {
            buffer.cursor_offset()
        };
        let active_index = buffer.content[..active_byte]
            .graphemes(true)
            .count()
            .min(self.spec.length - 1);
        let partial = !buffer.content.is_empty() && buffer.grapheme_count() < self.spec.length;
        for index in 0..self.spec.length {
            if self.separator_at == Some(index) {
                let sep_height = self.separator_height.min(height);
                divider = Some(Bounds::new(
                    point(left, top + (height - sep_height) / 2.),
                    size(separator_width, sep_height),
                ));
                left += separator_width + gap;
            }
            let slot = Bounds::new(point(left, top), size(side, height));
            left += side + gap;
            let glyph = glyphs.next().map(|(start, value)| {
                shape_digit(
                    start,
                    value,
                    &style,
                    self.digit_size * scale,
                    buffer.marked(),
                    window,
                )
            });
            let selected = focused
                && glyph.as_ref().is_some_and(|(_, range)| {
                    selection.len() > 1
                        && selection.start < range.end
                        && selection.end > range.start
                });
            let active = interactive && index == active_index && (focused || partial);
            cells.push(CodeCell {
                bounds: slot,
                glyph,
                selected,
                active,
                effect: GlyphEffect::default(),
            });
        }
        let pill = self
            .success
            .as_ref()
            .filter(|skin| skin.verified)
            .and_then(|success| success.frame(&cells, bounds, window));
        CodeFrame {
            cells,
            divider,
            scale,
            focused,
            ring_position: None,
            pill,
            discard_phase: self.discard_phase,
            caret_alpha: 0.,
        }
    }

    pub(crate) fn paint(&self, frame: &CodeFrame, window: &mut Window, cx: &mut App) {
        if let (Some(skin), Some(pill)) = (&self.success, &frame.pill) {
            self.paint_success(frame, skin, pill, window, cx);
            return;
        }
        for (index, cell) in frame.cells.iter().enumerate() {
            let skin = if cell.glyph.is_some() {
                &self.filled
            } else {
                &self.empty
            };
            skin.paint(cell.bounds, frame.scale, window);
            if let Some(phase) = self.shimmer_phase {
                let offset = f32::from(u16::try_from(index).unwrap_or(24)) * 0.07 / 1.3;
                shimmer(
                    cell.bounds,
                    skin.radius * frame.scale,
                    (phase - offset).rem_euclid(1.),
                    self.shimmer_color,
                    window,
                );
            }
            if cell.selected {
                let shade = quad(
                    cell.bounds,
                    gpui::Corners::all(skin.radius * frame.scale),
                    self.selection,
                    gpui::Edges::all(px(0.)),
                    self.selection,
                    gpui::BorderStyle::default(),
                );
                window.paint_quad(shade);
            }
            cell.paint_glyph(window, cx);
        }
        if let Some(divider) = frame.divider {
            window.paint_quad(fill(divider, self.separator_color));
        }
        if let Some(ring) = frame.ring_position {
            self.active.paint(ring, frame.scale, window);
            if frame.caret_alpha > 0. {
                let mut ink = self.caret;
                ink.a *= frame.caret_alpha;
                let height = ring.size.height * 0.4;
                window.paint_quad(fill(
                    Bounds::new(
                        point(ring.center().x - px(0.5), ring.center().y - height / 2.),
                        size(px(1.), height),
                    ),
                    ink,
                ));
            }
        }
    }
}

struct SuccessFrame {
    envelope: Bounds<Pixels>,
    label: Option<ShapedLine>,
    available: Bounds<Pixels>,
}

fn shape_digit(
    start: usize,
    value: &str,
    style: &gpui::TextStyle,
    font_size: Pixels,
    marked: Option<&Range<usize>>,
    window: &mut Window,
) -> (ShapedLine, Range<usize>) {
    let range = start..start + value.len();
    let marked = marked.is_some_and(|mark| mark.start < range.end && mark.end > range.start);
    let line = window.text_system().shape_line(
        value.to_owned().into(),
        font_size,
        &[TextRun {
            len: value.len(),
            font: style.font(),
            letter_spacing: style.letter_spacing,
            color: style.color,
            background_color: None,
            strikethrough: None,
            underline: marked.then_some(UnderlineStyle {
                color: None,
                thickness: px(1.),
                paint_tag: None,
                baseline_offset: None,
                wavy: false,
            }),
        }],
        None,
    );
    (line, range)
}

fn success_phase(elapsed: f32, start: f32, duration: f32) -> f32 {
    let linear = ((elapsed - start) / duration).clamp(0., 1.);
    crate::renderer::f64_to_f32(
        crate::MotionEasing::CubicBezier {
            x1: 0.22,
            y1: 1.,
            x2: 0.36,
            y2: 1.,
        }
        .sample(f64::from(linear)),
    )
}

impl CodePresentation {
    fn paint_folding_cells(
        &self,
        frame: &CodeFrame,
        center: Point<Pixels>,
        elapsed: f32,
        window: &mut Window,
        cx: &mut App,
    ) {
        let converge = success_phase(elapsed, 0., 340.);
        let opacity = 1. - ((elapsed - 120.) / 220.).clamp(0., 1.);
        if opacity <= 0. {
            return;
        }
        for cell in &frame.cells {
            let design = if cell.glyph.is_some() {
                &self.filled
            } else {
                &self.empty
            };
            if converge <= 0. {
                design.paint(cell.bounds, frame.scale, window);
                cell.paint_glyph(window, cx);
                continue;
            }
            let scale = f64::from((-0.3_f32).mul_add(converge, 1.));
            let transform = crate::ElementTransform {
                scale_x: scale,
                scale_y: scale,
                ..Default::default()
            };
            if let Some(mut matrix) = transform.matrix(cell.bounds) {
                matrix.translation[0] += f32::from(center.x - cell.bounds.center().x) * converge;
                window.paint_effect_group(opacity, px(0.), |window| {
                    let _ = window.with_composited_transform(matrix, |window| {
                        design.paint(cell.bounds, frame.scale, window);
                        cell.paint_glyph(window, cx);
                    });
                });
            }
        }
        if let Some(divider) = frame.divider {
            let mut ink = self.separator_color;
            ink.a *= 1. - (elapsed / 200.).clamp(0., 1.);
            window.paint_quad(fill(divider, ink));
        }
    }

    fn paint_success(
        &self,
        frame: &CodeFrame,
        success: &SuccessSkin,
        pill: &SuccessFrame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let elapsed = success.phase * 940.;
        let center = pill.envelope.center();
        self.paint_folding_cells(frame, center, elapsed, window, cx);
        let opacity = ((elapsed - 120.) / 160.).clamp(0., 1.);
        if opacity <= 0. {
            return;
        }
        let morph = success_phase(elapsed, 220., 460.);
        let target_width = (success.leading
            + success.icon_size
            + success.gap
            + success.trailing
            + pill.label.as_ref().map_or(px(0.), |line| line.width))
        .min(pill.available.size.width);
        let width = pill.envelope.size.width + (target_width - pill.envelope.size.width) * morph;
        let end_height = success.height.min(pill.available.size.height);
        let height = pill.envelope.size.height + (end_height - pill.envelope.size.height) * morph;
        let left = (center.x - width / 2.).clamp(
            pill.available.left(),
            (pill.available.right() - width).max(pill.available.left()),
        );
        let bounds = Bounds::new(point(left, center.y - height / 2.), size(width, height));
        let from_radius = self.empty.radius * frame.scale;
        let target_radius = success.skin.radius.min(height / 2.).min(width / 2.);
        let radius = from_radius + (target_radius - from_radius) * morph;
        let paint = |window: &mut Window, cx: &mut App| {
            success.skin.paint_radius(bounds, radius, window);
            let ink = ((elapsed - 460.) / 240.).clamp(0., 1.);
            if ink > 0. {
                window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                    let label_height = px(20.).min(height);
                    let origin = point(
                        bounds.left() + success.leading,
                        center.y - success.icon_size / 2.,
                    );
                    if let Some(line) = &pill.label {
                        let label_origin = point(
                            origin.x + success.icon_size + success.gap,
                            center.y - label_height / 2.,
                        );
                        let mut draw = |window: &mut Window| {
                            let _ = line.paint(
                                label_origin,
                                label_height,
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            );
                        };
                        if ink >= 1. {
                            draw(window);
                        } else {
                            window.paint_effect_group(ink, px(0.), draw);
                        }
                    }
                    let reveal = success_phase(elapsed, 560., 380.);
                    draw_check(
                        origin,
                        success.icon_size,
                        success.icon_color,
                        ink,
                        reveal,
                        window,
                    );
                });
            }
        };
        if opacity >= 1. {
            paint(window, cx);
        } else {
            window.paint_effect_group(opacity, px(0.), |window| paint(window, cx));
        }
    }
}

fn draw_check(
    origin: Point<Pixels>,
    side: Pixels,
    mut ink: Hsla,
    opacity: f32,
    progress: f32,
    window: &mut Window,
) {
    if progress <= 0. || side <= px(0.) {
        return;
    }
    let at = |x, y| origin + point(side * x, side * y);
    let a = at(5.5 / 24., 12.5 / 24.);
    let b = at(9.5 / 24., 16.5 / 24.);
    let c = at(18.5 / 24., 8. / 24.);
    let upper = 0.314_f32;
    let mut path = gpui::PathBuilder::stroke(side * 0.1);
    path.move_to(a);
    if progress < upper {
        path.line_to(a + (b - a) * (progress / upper));
    } else {
        path.line_to(b);
        path.line_to(b + (c - b) * ((progress - upper) / (1. - upper)));
    }
    ink.a *= opacity;
    if let Ok(path) = path.build() {
        window.paint_path(path, ink);
    }
}

fn shimmer(slot: Bounds<Pixels>, radius: Pixels, phase: f32, ink: gpui::Rgba, window: &mut Window) {
    if phase > 0.55 || slot.size.width <= px(0.) {
        return;
    }
    let band = slot.size.width * 0.38;
    let center = slot.left() + slot.size.width * ((phase / 0.55).mul_add(1.76, -0.38));
    let height = (slot.size.height - radius).max(px(0.));
    let top = slot.top() + (slot.size.height - height) / 2.;
    let transparent = gpui::Rgba { a: 0., ..ink };
    window.with_content_mask(Some(gpui::ContentMask { bounds: slot }), |window| {
        for (left, from, to) in [
            (center - band, transparent, ink),
            (center, ink, transparent),
        ] {
            let bounds = Bounds::new(point(left, top), size(band, height));
            window.paint_quad(fill(
                bounds,
                gpui::linear_gradient(
                    90.,
                    gpui::linear_color_stop(from, 0.),
                    gpui::linear_color_stop(to, 1.),
                ),
            ));
        }
    });
}

pub(crate) struct CodeFrame {
    cells: Vec<CodeCell>,
    divider: Option<Bounds<Pixels>>,
    scale: f32,
    focused: bool,
    ring_position: Option<Bounds<Pixels>>,
    pill: Option<SuccessFrame>,
    discard_phase: Option<f32>,
    caret_alpha: f32,
}

struct CodeCell {
    bounds: Bounds<Pixels>,
    glyph: Option<(ShapedLine, Range<usize>)>,
    selected: bool,
    active: bool,
    effect: GlyphEffect,
}

#[derive(Clone, Copy, PartialEq)]
struct GlyphEffect {
    opacity: f32,
    scale: f32,
    offset_y: f32,
    blur: f32,
}

impl Default for GlyphEffect {
    fn default() -> Self {
        Self {
            opacity: 1.,
            scale: 1.,
            offset_y: 0.,
            blur: 0.,
        }
    }
}

impl GlyphEffect {
    fn enter(p: f32) -> Self {
        Self {
            opacity: p,
            scale: 0.3_f32.mul_add(p, 0.7),
            offset_y: (1. - p) * 4.,
            blur: (1. - p) * 3.,
        }
    }

    fn discard(elapsed: f32, from_right: u8, occupied: u8) -> Self {
        // Long codes still stagger, without postponing their leftmost cell
        // past the transaction that clears the failed attempt at 980 ms.
        let stride = 22_f32.min(180. / f32::from(occupied.max(1)));
        let start = 700. + f32::from(from_right) * stride;
        let duration = 240_f32.min(980. - start);
        let t = ((elapsed - start) / duration).clamp(0., 1.);
        let p = crate::renderer::f64_to_f32(
            crate::MotionEasing::CubicBezier {
                x1: 0.25,
                y1: 0.1,
                x2: 0.25,
                y2: 1.,
            }
            .sample(f64::from(t)),
        );
        Self {
            opacity: 1. - p,
            scale: (-0.2_f32).mul_add(p, 1.),
            offset_y: -4. * p,
            blur: 2. * p,
        }
    }
}

impl CodeCell {
    fn paint_glyph(&self, window: &mut Window, cx: &mut App) {
        let Some((line, _)) = &self.glyph else { return };
        let effect = self.effect;
        if effect.opacity <= 0. {
            return;
        }
        let x = self.bounds.left() + ((self.bounds.size.width - line.width) / 2.).max(px(0.));
        let origin = point(x, self.bounds.top());
        let mut draw = |window: &mut Window| {
            let _ = line.paint(
                origin,
                self.bounds.size.height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
        };
        window.with_content_mask(
            Some(gpui::ContentMask {
                bounds: self.bounds,
            }),
            |window| {
                if effect == GlyphEffect::default() {
                    draw(window);
                } else {
                    let scale = f64::from(effect.scale);
                    let visual = crate::ElementTransform {
                        scale_x: scale,
                        scale_y: scale,
                        ..Default::default()
                    };
                    if let Some(mut matrix) = visual.matrix(self.bounds) {
                        matrix.translation[1] += effect.offset_y;
                        window.paint_effect_group(effect.opacity, px(effect.blur), |window| {
                            let _ = window.with_composited_transform(matrix, draw);
                        });
                    }
                }
            },
        );
    }
}

impl CodeFrame {
    pub(crate) fn sample_edit_motion(
        &mut self,
        state: &mut crate::segmented_motion::SegmentedMotion,
        motion: Option<&crate::renderer::NativeNodeMotion>,
        committed: bool,
        window: &Window,
    ) {
        state.clear_retired(motion, window);
        let target = self.cells.iter().position(|cell| cell.active);
        self.ring_position = state.ring(target, motion, window).and_then(|index| {
            let low = self
                .cells
                .iter()
                .enumerate()
                .rfind(|(i, _)| f32::from(u8::try_from(*i).unwrap_or_default()) <= index);
            let (floor, cell) = low?;
            let fraction = (index - f32::from(u8::try_from(floor).ok()?)).clamp(0., 1.);
            let mut bounds = cell.bounds;
            if let Some(right) = self.cells.get(floor + 1) {
                bounds.origin.x += (right.bounds.left() - bounds.left()) * fraction;
            }
            Some(bounds)
        });
        let mask = window.content_mask().bounds;
        let caret = target.filter(|slot| {
            let cell = &self.cells[*slot];
            cell.glyph.is_none()
                && self.focused
                && window.is_window_active()
                && cell.bounds.intersects(&mask)
        });
        let alpha = state.caret(caret, motion, window);
        self.caret_alpha = if caret.is_some() { alpha } else { 0. };
        if let Some(phase) = self.discard_phase {
            let elapsed = phase * 980.;
            let occupied = self
                .cells
                .iter()
                .take_while(|cell| cell.glyph.is_some())
                .count();
            let count = u8::try_from(occupied).unwrap_or_default();
            for (i, cell) in self.cells.iter_mut().take(occupied).enumerate() {
                cell.effect = GlyphEffect::discard(
                    elapsed,
                    count - u8::try_from(i).unwrap_or_default(),
                    count,
                );
            }
        } else if committed {
            for (i, cell) in self.cells.iter_mut().enumerate() {
                if cell.glyph.is_some() {
                    cell.effect = GlyphEffect::enter(state.glyph(i, motion, window));
                }
            }
        }
    }
    pub(crate) fn index_for_point(&self, point: Point<Pixels>, end: usize) -> usize {
        self.cells
            .iter()
            .find(|cell| point.x < cell.bounds.right())
            .and_then(|cell| cell.glyph.as_ref())
            .map_or(end, |(_, range)| range.start)
    }

    pub(crate) fn bounds_for_range(
        &self,
        range: &Range<usize>,
        end: usize,
    ) -> Option<Bounds<Pixels>> {
        let last = self.cells.last()?.bounds;
        let cell_for = |byte| {
            self.cells
                .iter()
                .find(|cell| {
                    cell.glyph
                        .as_ref()
                        .is_some_and(|(_, range)| range.contains(&byte))
                        || (cell.glyph.is_none() && byte >= end)
                })
                .map_or(last, |cell| cell.bounds)
        };
        let start = cell_for(range.start);
        if range.is_empty() {
            return Some(Bounds::new(
                point(start.center().x, start.top()),
                size(px(0.), start.size.height),
            ));
        }
        let finish = cell_for(range.end.saturating_sub(1));
        Some(Bounds::from_corners(start.origin, finish.bottom_right()))
    }
}
