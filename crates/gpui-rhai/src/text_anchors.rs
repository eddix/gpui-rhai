//! Opt-in, versioned queries of one rendered text node's grapheme geometry.

#[cfg(not(feature = "vendor-gpui"))]
use crate::presentation_backend::PublicWindowExt as _;
use std::{collections::BTreeMap, fmt, ops::Range, rc::Rc};

use gpui::{Bounds, Pixels, Point, TextLayout, Window, px, size};
use rhai::{Array, Dynamic, Engine, EvalAltResult, FuncRegistration, INT, ImmutableString, Map};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation as _;

use crate::{
    GeometryBounds, UiValue,
    text_geometry::{CaretAffinity, TextGeometry},
};

pub(crate) const MAX_ANCHORED_NODES: usize = 128;
const MAX_REQUESTS: usize = 32;
const MAX_BYTES: usize = 16 * 1024;
const MAX_RECTS: usize = 256;

/// One anchor in the original document text (extended grapheme indices).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextAnchorRequest {
    Caret { at: usize, affinity: CaretAffinity },
    Range(Range<usize>),
}

#[derive(Clone, Eq, PartialEq)]
struct Plan {
    source: Rc<str>,
    revision: u64,
    requests: BTreeMap<String, TextAnchorRequest>,
}

/// A small immutable request. Source bytes, revision and requested ranges all
/// participate in matching the subsequently published geometry.
#[derive(Clone, Eq, PartialEq)]
pub struct TextAnchorPlan(Rc<Plan>);

impl fmt::Debug for TextAnchorPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never dump document contents via a diagnostic about its UI node.
        f.debug_struct("TextAnchorPlan")
            .field("revision", &self.0.revision)
            .field("source_bytes", &self.0.source.len())
            .field("request_count", &self.0.requests.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum TextAnchorError {
    #[error("text anchor plans accept at most 16 KiB of text and 32 requests")]
    Capacity,
    #[error("text anchor requests need distinct non-empty IDs (at most 96 bytes)")]
    InvalidId,
    #[error("text anchor endpoints must be ordered extended grapheme boundaries in that text")]
    InvalidRange,
    #[error("text anchor capture requires a referenced plain or rich text node")]
    InvalidNode,
    #[error("the text anchor plan belongs to different text")]
    SourceMismatch,
    #[error(
        "expected nonnegative revision and anchor maps with id, start, optional end or affinity"
    )]
    InvalidArguments,
}

impl TextAnchorPlan {
    /// Validate and retain a versioned request.
    ///
    /// # Errors
    ///
    /// Rejects excessive text/requests, invalid IDs and out-of-range endpoints.
    pub fn new(
        source: impl Into<Rc<str>>,
        revision: u64,
        requests: impl IntoIterator<Item = (String, TextAnchorRequest)>,
    ) -> Result<Self, TextAnchorError> {
        let source = source.into();
        if source.len() > MAX_BYTES {
            return Err(TextAnchorError::Capacity);
        }
        let count = source.graphemes(true).count();
        let mut result = BTreeMap::new();
        for (id, request) in requests {
            if result.len() >= MAX_REQUESTS {
                return Err(TextAnchorError::Capacity);
            }
            if id.is_empty()
                || id.len() > 96
                || id.chars().any(char::is_control)
                || result.contains_key(&id)
            {
                return Err(TextAnchorError::InvalidId);
            }
            let valid = match &request {
                TextAnchorRequest::Caret { at, .. } => *at <= count,
                TextAnchorRequest::Range(range) => range.start < range.end && range.end <= count,
            };
            if !valid {
                return Err(TextAnchorError::InvalidRange);
            }
            result.insert(id, request);
        }
        Ok(Self(Rc::new(Plan {
            source,
            revision,
            requests: result,
        })))
    }

    pub(crate) fn source(&self) -> &str {
        &self.0.source
    }

    pub(crate) fn project(
        &self,
        layout: &TextLayout,
        window: &Window,
    ) -> Option<TextAnchorSnapshot> {
        let actual = layout.text();
        if actual != self.source() {
            // Truncation/clamping must not turn old or absent character indices
            // into an apparently valid anchor for a new document version.
            return None;
        }
        let map = TextGeometry::from_painted(layout, window.text_style().text_align, &actual)?;
        let coords = window
            .visual_coordinates()
            .clipped_to(window.content_mask().bounds);
        let mut result = BTreeMap::new();
        let mut remaining = MAX_RECTS;
        for (id, request) in &self.0.requests {
            let value = match request {
                TextAnchorRequest::Caret { at, affinity } => {
                    let Some(caret) = map.caret(*at, *affinity) else {
                        continue;
                    };
                    remaining = remaining.checked_sub(1)?;
                    let bounds = Bounds::new(caret.position, size(px(0.0), caret.line_height));
                    let point_value = coords
                        .visual_point(caret.position)
                        .map_or(UiValue::Null, into_point);
                    UiValue::Map(BTreeMap::from([
                        ("caret".to_owned(), point_value),
                        ("byte".to_owned(), int_value(caret.byte_offset)),
                        (
                            "rects".to_owned(),
                            UiValue::Array(vec![rect_value(bounds, &coords)]),
                        ),
                    ]))
                }
                TextAnchorRequest::Range(range) => {
                    let rects = map.range(range.clone());
                    // All or nothing: consumers may rely on a complete selection.
                    remaining = remaining.checked_sub(rects.len())?;
                    UiValue::Map(BTreeMap::from([(
                        "rects".to_owned(),
                        UiValue::Array(
                            rects
                                .into_iter()
                                .map(|bounds| rect_value(bounds, &coords))
                                .collect(),
                        ),
                    )]))
                }
            };
            result.insert(id.clone(), value);
        }
        Some(TextAnchorSnapshot {
            plan: self.clone(),
            value: UiValue::Map(result),
        })
    }
}

fn rect_value(
    bounds: Bounds<Pixels>,
    coords: &crate::presentation_backend::VisualCoordinates,
) -> UiValue {
    let quad = [
        gpui::point(bounds.left(), bounds.top()),
        gpui::point(bounds.right(), bounds.top()),
        gpui::point(bounds.right(), bounds.bottom()),
        gpui::point(bounds.left(), bounds.bottom()),
    ]
    .into_iter()
    .map(|point| coords.visual_point(point).map(into_point))
    .collect::<Option<Vec<_>>>()
    .map_or(UiValue::Null, UiValue::Array);
    UiValue::Map(BTreeMap::from([
        ("layout".to_owned(), into_bounds(bounds)),
        ("quad".to_owned(), quad),
        (
            "visual".to_owned(),
            coords
                .visual_bounds(bounds)
                .map_or(UiValue::Null, into_bounds),
        ),
        (
            "clipped".to_owned(),
            coords
                .clipped_visual_bounds(bounds)
                .map_or(UiValue::Null, into_bounds),
        ),
    ]))
}

fn into_point(point: Point<Pixels>) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("x".to_owned(), UiValue::Float(f64::from(point.x))),
        ("y".to_owned(), UiValue::Float(f64::from(point.y))),
    ]))
}

fn into_bounds(bounds: Bounds<Pixels>) -> UiValue {
    GeometryBounds {
        x: f64::from(bounds.left()),
        y: f64::from(bounds.top()),
        width: f64::from(bounds.size.width),
        height: f64::from(bounds.size.height),
    }
    .into_value()
}

fn int_value(value: usize) -> UiValue {
    UiValue::Integer(INT::try_from(value).unwrap_or(INT::MAX))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextAnchorSnapshot {
    pub(crate) plan: TextAnchorPlan,
    pub(crate) value: UiValue,
}

pub(crate) fn script_error(error: TextAnchorError) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.to_string().into(), rhai::Position::NONE)
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_type_with_name::<TextAnchorPlan>("TextAnchorPlan");
    FuncRegistration::new("text_anchor_plan")
        .in_global_namespace()
        .register_into_engine(engine, script_plan);
}

fn script_plan(
    source: &str,
    revision: INT,
    requests: Array,
) -> Result<TextAnchorPlan, Box<EvalAltResult>> {
    if requests.len() > MAX_REQUESTS || source.len() > MAX_BYTES {
        return Err(Box::new(script_error(TextAnchorError::Capacity)));
    }
    let revision = u64::try_from(revision)
        .map_err(|_| Box::new(script_error(TextAnchorError::InvalidArguments)))?;
    let requests = requests
        .into_iter()
        .map(script_request)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| Box::new(script_error(error)))?;
    TextAnchorPlan::new(source, revision, requests).map_err(|error| Box::new(script_error(error)))
}

fn script_request(value: Dynamic) -> Result<(String, TextAnchorRequest), TextAnchorError> {
    let error = TextAnchorError::InvalidArguments;
    let mut form = value.try_cast::<Map>().ok_or(error)?;
    let id = form
        .remove("id")
        .and_then(Dynamic::try_cast::<ImmutableString>)
        .ok_or(error)?;
    let start = form
        .remove("start")
        .and_then(Dynamic::try_cast::<INT>)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(error)?;
    let request = if let Some(end) = form.remove("end") {
        let end = end
            .try_cast::<INT>()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(error)?;
        TextAnchorRequest::Range(start..end)
    } else {
        let affinity = match form.remove("affinity") {
            None => CaretAffinity::Downstream,
            Some(value) => match value.try_cast::<ImmutableString>().as_deref() {
                Some("upstream") => CaretAffinity::Upstream,
                Some("downstream") => CaretAffinity::Downstream,
                _ => return Err(error),
            },
        };
        TextAnchorRequest::Caret {
            at: start,
            affinity,
        }
    };
    if !form.is_empty() {
        return Err(error);
    }
    Ok((id.to_string(), request))
}
