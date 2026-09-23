#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::sync::{Arc, RwLock};

use serde_json::Value;
use thiserror::Error;
use usvg::tiny_skia_path::{PathSegment, Point};
use usvg::{Node, Tree};

use crate::InlineSvg;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartGeoPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartGeoRing {
    pub points: Arc<[ChartGeoPoint]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartGeoFeature {
    pub key: String,
    pub name: String,
    /// One feature may contain multiple polygon rings. The first ring of each
    /// source polygon is an exterior; subsequent rings are holes.
    pub rings: Arc<[ChartGeoRing]>,
    pub properties: Arc<BTreeMap<String, String>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartGeoBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl ChartGeoBounds {
    #[must_use]
    pub fn width(self) -> f64 {
        self.max_x - self.min_x
    }

    #[must_use]
    pub fn height(self) -> f64 {
        self.max_y - self.min_y
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartGeoMap {
    pub key: String,
    pub features: Arc<[ChartGeoFeature]>,
    pub bounds: ChartGeoBounds,
    pub projected: bool,
}

impl ChartGeoMap {
    /// Parse a `GeoJSON` `FeatureCollection` containing `Polygon` or `MultiPolygon`
    /// geometry. Coordinates remain longitude/latitude until a region applies
    /// its registered projection.
    ///
    /// # Errors
    ///
    /// Returns source-size, JSON, geometry, identity, or coordinate diagnostics.
    pub fn from_geojson(key: impl Into<String>, source: &str) -> Result<Self, ChartGeoError> {
        let key = key.into();
        validate_identifier(&key)?;
        if source.len() > 64 * 1024 * 1024 {
            return Err(ChartGeoError::SourceTooLarge(source.len()));
        }
        let root: Value = serde_json::from_str(source)
            .map_err(|error| ChartGeoError::InvalidGeoJson(error.to_string()))?;
        if root.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
            return Err(ChartGeoError::ExpectedFeatureCollection);
        }
        let features = root
            .get("features")
            .and_then(Value::as_array)
            .ok_or(ChartGeoError::ExpectedFeatureCollection)?;
        if features.len() > 50_000 {
            return Err(ChartGeoError::TooManyFeatures(features.len()));
        }
        let mut keys = BTreeSet::new();
        let mut parsed = Vec::with_capacity(features.len());
        for (index, feature) in features.iter().enumerate() {
            let geometry = feature
                .get("geometry")
                .ok_or(ChartGeoError::MissingGeometry(index))?;
            let kind = geometry.get("type").and_then(Value::as_str).unwrap_or("");
            let coordinates = geometry
                .get("coordinates")
                .ok_or(ChartGeoError::MissingGeometry(index))?;
            let rings = match kind {
                "Polygon" => parse_polygon(coordinates, index)?,
                "MultiPolygon" => coordinates
                    .as_array()
                    .ok_or(ChartGeoError::InvalidCoordinates(index))?
                    .iter()
                    .map(|polygon| parse_polygon(polygon, index))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
                other => return Err(ChartGeoError::UnsupportedGeometry(other.to_owned())),
            };
            let properties = feature
                .get("properties")
                .and_then(Value::as_object)
                .map(|properties| {
                    properties
                        .iter()
                        .filter_map(|(name, value)| {
                            scalar_property(value).map(|value| (name.clone(), value))
                        })
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            let feature_key = feature
                .get("id")
                .and_then(scalar_property)
                .or_else(|| properties.get("id").cloned())
                .or_else(|| properties.get("name").cloned())
                .unwrap_or_else(|| format!("feature_{index}"));
            if !keys.insert(feature_key.clone()) {
                return Err(ChartGeoError::DuplicateFeature(feature_key));
            }
            let name = properties
                .get("name")
                .cloned()
                .unwrap_or_else(|| feature_key.clone());
            parsed.push(ChartGeoFeature {
                key: feature_key,
                name,
                rings: rings.into(),
                properties: Arc::new(properties),
            });
        }
        Self::from_features(key, parsed, false)
    }

    /// Parse self-contained SVG paths as already-projected map regions. Every
    /// visible path requires an `id`; curves are flattened for deterministic
    /// hit testing and export.
    ///
    /// # Errors
    ///
    /// Returns validation, parse, identity, or empty-geometry diagnostics.
    pub fn from_svg(key: impl Into<String>, source: &str) -> Result<Self, ChartGeoError> {
        let key = key.into();
        validate_identifier(&key)?;
        InlineSvg::new(source).map_err(|error| ChartGeoError::InvalidSvg(error.to_string()))?;
        let tree = Tree::from_str(source, &usvg::Options::default())
            .map_err(|error| ChartGeoError::InvalidSvg(error.to_string()))?;
        let mut features = Vec::new();
        collect_svg_paths(tree.root().children(), &mut features)?;
        Self::from_features(key, features, true)
    }

    /// Construct a map from Host-prepared feature geometry.
    ///
    /// # Errors
    ///
    /// Returns for invalid identity, duplicate/empty features, or bad bounds.
    pub fn from_features(
        key: impl Into<String>,
        features: Vec<ChartGeoFeature>,
        projected: bool,
    ) -> Result<Self, ChartGeoError> {
        let key = key.into();
        validate_identifier(&key)?;
        if features.is_empty() {
            return Err(ChartGeoError::NoFeatures);
        }
        let mut feature_keys = BTreeSet::new();
        let mut bounds = None;
        for feature in &features {
            if feature.key.is_empty() || !feature_keys.insert(feature.key.clone()) {
                return Err(ChartGeoError::DuplicateFeature(feature.key.clone()));
            }
            for ring in feature.rings.iter() {
                if ring.points.len() < 3 {
                    return Err(ChartGeoError::InvalidRing(feature.key.clone()));
                }
                for point in ring.points.iter().copied() {
                    if !point.x.is_finite() || !point.y.is_finite() {
                        return Err(ChartGeoError::NonFinitePoint(feature.key.clone()));
                    }
                    bounds = Some(expand_bounds(bounds, point));
                }
            }
        }
        let bounds = bounds.ok_or(ChartGeoError::NoFeatures)?;
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Err(ChartGeoError::EmptyBounds);
        }
        Ok(Self {
            key,
            features: features.into(),
            bounds,
            projected,
        })
    }

    #[must_use]
    pub fn hit_test(&self, point: ChartGeoPoint) -> Option<&ChartGeoFeature> {
        self.features.iter().rev().find(|feature| {
            feature
                .rings
                .iter()
                .filter(|ring| point_in_ring(point, &ring.points))
                .count()
                % 2
                == 1
        })
    }
}

pub trait ChartGeoProjection: Send + Sync {
    fn project(&self, longitude: f64, latitude: f64) -> Option<ChartGeoPoint>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EquirectangularProjection;

impl ChartGeoProjection for EquirectangularProjection {
    fn project(&self, longitude: f64, latitude: f64) -> Option<ChartGeoPoint> {
        (longitude.is_finite()
            && latitude.is_finite()
            && (-180.0..=180.0).contains(&longitude)
            && (-90.0..=90.0).contains(&latitude))
        .then_some(ChartGeoPoint {
            x: longitude,
            y: -latitude,
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MercatorProjection;

impl ChartGeoProjection for MercatorProjection {
    fn project(&self, longitude: f64, latitude: f64) -> Option<ChartGeoPoint> {
        if !longitude.is_finite()
            || !latitude.is_finite()
            || !(-180.0..=180.0).contains(&longitude)
            || !(-85.051_128_78..=85.051_128_78).contains(&latitude)
        {
            return None;
        }
        let radians = latitude.to_radians();
        Some(ChartGeoPoint {
            x: longitude,
            y: -(std::f64::consts::FRAC_PI_4 + radians / 2.0)
                .tan()
                .ln()
                .to_degrees(),
        })
    }
}

#[derive(Clone, Default)]
pub struct ChartGeoRegistry {
    maps: Arc<RwLock<BTreeMap<String, Arc<ChartGeoMap>>>>,
    projections: Arc<RwLock<BTreeMap<String, Arc<dyn ChartGeoProjection>>>>,
}

impl std::fmt::Debug for ChartGeoRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let maps = self
            .maps
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let projections = self
            .projections
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        formatter
            .debug_struct("ChartGeoRegistry")
            .field("maps", &maps)
            .field("projections", &projections)
            .finish()
    }
}

impl ChartGeoRegistry {
    /// Construct a registry with equirectangular and Mercator projections.
    ///
    /// # Panics
    ///
    /// Panics only if the compile-time built-in IDs stop satisfying the same
    /// public identifier validation used for Host projections.
    #[must_use]
    pub fn new() -> Self {
        let registry = Self::default();
        registry
            .register_projection("equirectangular", EquirectangularProjection)
            .expect("built-in projection ID is valid");
        registry
            .register_projection("mercator", MercatorProjection)
            .expect("built-in projection ID is valid");
        registry
    }

    /// Register one Host-owned map.
    ///
    /// # Errors
    ///
    /// Returns for a duplicate map or poisoned registry.
    pub fn register_map(&self, map: ChartGeoMap) -> Result<(), ChartGeoError> {
        let mut maps = self.maps.write().map_err(|_| ChartGeoError::Poisoned)?;
        match maps.entry(map.key.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(Arc::new(map));
            }
            Entry::Occupied(_) => return Err(ChartGeoError::DuplicateMap(map.key)),
        }
        Ok(())
    }

    /// Register one trusted projection.
    ///
    /// # Errors
    ///
    /// Returns for an unsafe/duplicate ID or poisoned registry.
    pub fn register_projection(
        &self,
        id: impl Into<String>,
        projection: impl ChartGeoProjection + 'static,
    ) -> Result<(), ChartGeoError> {
        let id = id.into();
        validate_identifier(&id)?;
        let mut projections = self
            .projections
            .write()
            .map_err(|_| ChartGeoError::Poisoned)?;
        match projections.entry(id.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(Arc::new(projection));
            }
            Entry::Occupied(_) => return Err(ChartGeoError::DuplicateProjection(id)),
        }
        Ok(())
    }

    /// Read a registered map.
    ///
    /// # Errors
    ///
    /// Returns for an unknown map or poisoned registry.
    pub fn map(&self, id: &str) -> Result<Arc<ChartGeoMap>, ChartGeoError> {
        self.maps
            .read()
            .map_err(|_| ChartGeoError::Poisoned)?
            .get(id)
            .cloned()
            .ok_or_else(|| ChartGeoError::UnknownMap(id.to_owned()))
    }

    /// Read a registered projection.
    ///
    /// # Errors
    ///
    /// Returns for an unknown projection or poisoned registry.
    pub fn projection(&self, id: &str) -> Result<Arc<dyn ChartGeoProjection>, ChartGeoError> {
        self.projections
            .read()
            .map_err(|_| ChartGeoError::Poisoned)?
            .get(id)
            .cloned()
            .ok_or_else(|| ChartGeoError::UnknownProjection(id.to_owned()))
    }

    /// Project one longitude/latitude map without modifying the registered
    /// source. SVG maps are already projected and are returned unchanged.
    ///
    /// # Errors
    ///
    /// Returns registry lookup, rejected-coordinate, or geometry diagnostics.
    pub fn projected_map(&self, map: &str, projection: &str) -> Result<ChartGeoMap, ChartGeoError> {
        let map = self.map(map)?;
        if map.projected {
            return Ok((*map).clone());
        }
        let projection = self.projection(projection)?;
        let features = map
            .features
            .iter()
            .map(|feature| {
                let rings = feature
                    .rings
                    .iter()
                    .map(|ring| {
                        let points = ring
                            .points
                            .iter()
                            .map(|point| {
                                projection.project(point.x, point.y).ok_or(
                                    ChartGeoError::ProjectionFailed {
                                        longitude: point.x,
                                        latitude: point.y,
                                    },
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok(ChartGeoRing {
                            points: points.into(),
                        })
                    })
                    .collect::<Result<Vec<_>, ChartGeoError>>()?;
                Ok(ChartGeoFeature {
                    key: feature.key.clone(),
                    name: feature.name.clone(),
                    rings: rings.into(),
                    properties: Arc::clone(&feature.properties),
                })
            })
            .collect::<Result<Vec<_>, ChartGeoError>>()?;
        ChartGeoMap::from_features(map.key.clone(), features, true)
    }

    #[cfg(feature = "dev-reload")]
    pub(crate) fn copy_from(&self, source: &Self) {
        let maps = source
            .maps
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let projections = source
            .projections
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        *self
            .maps
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = maps;
        *self
            .projections
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = projections;
    }
}

fn parse_polygon(value: &Value, index: usize) -> Result<Vec<ChartGeoRing>, ChartGeoError> {
    value
        .as_array()
        .ok_or(ChartGeoError::InvalidCoordinates(index))?
        .iter()
        .map(|ring| {
            let points = ring
                .as_array()
                .ok_or(ChartGeoError::InvalidCoordinates(index))?
                .iter()
                .map(|coordinate| {
                    let coordinate = coordinate
                        .as_array()
                        .ok_or(ChartGeoError::InvalidCoordinates(index))?;
                    let x = coordinate
                        .first()
                        .and_then(Value::as_f64)
                        .ok_or(ChartGeoError::InvalidCoordinates(index))?;
                    let y = coordinate
                        .get(1)
                        .and_then(Value::as_f64)
                        .ok_or(ChartGeoError::InvalidCoordinates(index))?;
                    Ok(ChartGeoPoint { x, y })
                })
                .collect::<Result<Vec<_>, ChartGeoError>>()?;
            if points.len() < 3 {
                return Err(ChartGeoError::InvalidCoordinates(index));
            }
            Ok(ChartGeoRing {
                points: points.into(),
            })
        })
        .collect()
}

fn scalar_property(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn collect_svg_paths(
    nodes: &[Node],
    output: &mut Vec<ChartGeoFeature>,
) -> Result<(), ChartGeoError> {
    for node in nodes {
        match node {
            Node::Group(group) => collect_svg_paths(group.children(), output)?,
            Node::Path(path) if path.is_visible() => {
                let id = path.id();
                if id.is_empty() {
                    return Err(ChartGeoError::SvgPathWithoutId);
                }
                let transform = path.abs_transform();
                let rings = flatten_svg_path(path.data().segments(), transform)?;
                output.push(ChartGeoFeature {
                    key: id.to_owned(),
                    name: id.to_owned(),
                    rings: rings.into(),
                    properties: Arc::new(BTreeMap::new()),
                });
            }
            Node::Path(_) | Node::Image(_) | Node::Text(_) => {}
        }
    }
    Ok(())
}

fn flatten_svg_path(
    segments: impl Iterator<Item = PathSegment>,
    transform: usvg::Transform,
) -> Result<Vec<ChartGeoRing>, ChartGeoError> {
    let mut rings = Vec::new();
    let mut current = Vec::new();
    let mut last = Point::from_xy(0.0, 0.0);
    for segment in segments {
        match segment {
            PathSegment::MoveTo(point) => {
                if current.len() >= 3 {
                    rings.push(ChartGeoRing {
                        points: std::mem::take(&mut current).into(),
                    });
                } else {
                    current.clear();
                }
                last = point;
                current.push(transform_point(point, transform));
            }
            PathSegment::LineTo(point) => {
                last = point;
                current.push(transform_point(point, transform));
            }
            PathSegment::QuadTo(control, point) => {
                for step in 1..=8 {
                    let t = f32::from(step as u16) / 8.0;
                    current.push(transform_point(
                        quad_point(last, control, point, t),
                        transform,
                    ));
                }
                last = point;
            }
            PathSegment::CubicTo(first, second, point) => {
                for step in 1..=12 {
                    let t = f32::from(step as u16) / 12.0;
                    current.push(transform_point(
                        cubic_point(last, first, second, point, t),
                        transform,
                    ));
                }
                last = point;
            }
            PathSegment::Close => {
                if current.len() >= 3 {
                    rings.push(ChartGeoRing {
                        points: std::mem::take(&mut current).into(),
                    });
                }
            }
        }
    }
    if current.len() >= 3 {
        rings.push(ChartGeoRing {
            points: current.into(),
        });
    }
    if rings.is_empty() {
        Err(ChartGeoError::SvgPathWithoutGeometry)
    } else {
        Ok(rings)
    }
}

fn transform_point(mut point: Point, transform: usvg::Transform) -> ChartGeoPoint {
    transform.map_point(&mut point);
    ChartGeoPoint {
        x: f64::from(point.x),
        y: f64::from(point.y),
    }
}

fn quad_point(start: Point, control: Point, end: Point, t: f32) -> Point {
    let inverse = 1.0 - t;
    Point::from_xy(
        inverse * inverse * start.x + 2.0 * inverse * t * control.x + t * t * end.x,
        inverse * inverse * start.y + 2.0 * inverse * t * control.y + t * t * end.y,
    )
}

fn cubic_point(start: Point, first: Point, second: Point, end: Point, t: f32) -> Point {
    let inverse = 1.0 - t;
    Point::from_xy(
        inverse.powi(3) * start.x
            + 3.0 * inverse * inverse * t * first.x
            + 3.0 * inverse * t * t * second.x
            + t.powi(3) * end.x,
        inverse.powi(3) * start.y
            + 3.0 * inverse * inverse * t * first.y
            + 3.0 * inverse * t * t * second.y
            + t.powi(3) * end.y,
    )
}

fn expand_bounds(bounds: Option<ChartGeoBounds>, point: ChartGeoPoint) -> ChartGeoBounds {
    bounds.map_or(
        ChartGeoBounds {
            min_x: point.x,
            min_y: point.y,
            max_x: point.x,
            max_y: point.y,
        },
        |bounds| ChartGeoBounds {
            min_x: bounds.min_x.min(point.x),
            min_y: bounds.min_y.min(point.y),
            max_x: bounds.max_x.max(point.x),
            max_y: bounds.max_y.max(point.y),
        },
    )
}

fn point_in_ring(point: ChartGeoPoint, ring: &[ChartGeoPoint]) -> bool {
    let mut inside = false;
    let mut previous = ring.len().saturating_sub(1);
    for current in 0..ring.len() {
        let left = ring[current];
        let right = ring[previous];
        if (left.y > point.y) != (right.y > point.y)
            && point.x < (right.x - left.x) * (point.y - left.y) / (right.y - left.y) + left.x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn validate_identifier(value: &str) -> Result<(), ChartGeoError> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Ok(())
    } else {
        Err(ChartGeoError::InvalidId(value.to_owned()))
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartGeoError {
    #[error("chart geo ID `{0}` must be a safe identifier")]
    InvalidId(String),
    #[error("chart geo source has {0} bytes and exceeds the 64 MiB limit")]
    SourceTooLarge(usize),
    #[error("invalid GeoJSON: {0}")]
    InvalidGeoJson(String),
    #[error("chart GeoJSON must be a FeatureCollection")]
    ExpectedFeatureCollection,
    #[error("chart GeoJSON contains {0} features; limit is 50000")]
    TooManyFeatures(usize),
    #[error("chart GeoJSON feature {0} has no geometry")]
    MissingGeometry(usize),
    #[error("chart GeoJSON feature {0} has invalid polygon coordinates")]
    InvalidCoordinates(usize),
    #[error("chart GeoJSON geometry `{0}` is not Polygon or MultiPolygon")]
    UnsupportedGeometry(String),
    #[error("chart geo feature `{0}` is duplicated")]
    DuplicateFeature(String),
    #[error("chart geo map must contain at least one feature")]
    NoFeatures,
    #[error("chart geo feature `{0}` contains a ring with fewer than three points")]
    InvalidRing(String),
    #[error("chart geo feature `{0}` contains a non-finite point")]
    NonFinitePoint(String),
    #[error("chart geo map bounds are empty")]
    EmptyBounds,
    #[error("invalid chart SVG map: {0}")]
    InvalidSvg(String),
    #[error("every visible chart SVG map path requires an id")]
    SvgPathWithoutId,
    #[error("chart SVG map path has no closed geometry")]
    SvgPathWithoutGeometry,
    #[error("chart geo map `{0}` is already registered")]
    DuplicateMap(String),
    #[error("chart geo projection `{0}` is already registered")]
    DuplicateProjection(String),
    #[error("chart geo map `{0}` is not registered")]
    UnknownMap(String),
    #[error("chart geo projection `{0}` is not registered")]
    UnknownProjection(String),
    #[error("chart geo projection rejected coordinate {longitude},{latitude}")]
    ProjectionFailed { longitude: f64, latitude: f64 },
    #[error("chart geo registry lock was poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geojson_maps_parse_project_and_hit_test() {
        let source = r#"{
            "type":"FeatureCollection",
            "features":[{"type":"Feature","id":"box","properties":{"name":"Box"},
              "geometry":{"type":"Polygon","coordinates":[
                [[0,0],[10,0],[10,10],[0,10],[0,0]],
                [[4,4],[4,6],[6,6],[6,4],[4,4]]
              ]}}
            ]}"#;
        let map = ChartGeoMap::from_geojson("test", source).unwrap();
        assert_eq!(map.features[0].name, "Box");
        assert_eq!(
            map.hit_test(ChartGeoPoint { x: 1.0, y: 1.0 })
                .map(|feature| feature.key.as_str()),
            Some("box")
        );
        assert!(map.hit_test(ChartGeoPoint { x: 5.0, y: 5.0 }).is_none());

        let registry = ChartGeoRegistry::new();
        registry.register_map(map).unwrap();
        let projected = registry.projected_map("test", "mercator").unwrap();
        assert!(projected.projected);
        assert_eq!(projected.features.len(), 1);
    }

    #[test]
    fn svg_map_paths_keep_ids_and_curves_become_hit_testable_polygons() {
        let map = ChartGeoMap::from_svg(
            "svg_test",
            r#"<svg viewBox="0 0 20 20"><path id="region" d="M1 1 C 10 0, 20 10, 10 19 L 1 10 Z"/></svg>"#,
        )
        .unwrap();
        assert_eq!(map.features[0].key, "region");
        assert!(!map.features[0].rings[0].points.is_empty());
    }
}
