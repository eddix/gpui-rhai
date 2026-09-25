use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, LazyLock};

use gpui::{AppContext as _, Image, ImageCacheError, ImageFormat, ImageSource, RenderImage};
use rhai::{CustomType, TypeBuilder};
use thiserror::Error;

use crate::{
    AsyncDelivery, AsyncScope, ComponentInstancePath, OpaqueHandle, Rgba8, ScriptCallback,
    ScriptGeneration, UiValue,
};

const MAX_SVG_RASTER_DIMENSION: u32 = 16_384;
const MAX_SVG_RASTER_DIMENSION_F32: f32 = 16_384.0;
const MAX_SVG_RASTER_PIXELS: u64 = 16_777_216;
const SVG_VARIANT_CACHE_MAX_ENTRIES: usize = 256;
const SVG_VARIANT_CACHE_MAX_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetId(String);

impl AssetId {
    /// Parse a namespaced logical asset ID such as `core/check`.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::InvalidId`] for URLs, absolute/traversal paths, or
    /// unsupported characters.
    pub fn parse(value: impl Into<String>) -> Result<Self, AssetError> {
        let value = value.into();
        let segments = value.split('/').collect::<Vec<_>>();
        let valid = segments.len() >= 2
            && segments.iter().all(|segment| {
                !segment.is_empty()
                    && *segment != "."
                    && *segment != ".."
                    && segment.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                    })
            });
        if valid && !value.contains(':') && !value.contains('\\') {
            Ok(Self(value))
        } else {
            Err(AssetError::InvalidId(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn split(&self) -> (&str, &str) {
        self.0.split_once('/').expect("validated asset ID")
    }
}

impl CustomType for AssetId {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("AssetId")
            .with_fn("to_string", |asset: &mut Self| asset.0.clone());
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetData {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

pub trait AssetProvider {
    /// Load a provider-relative logical asset name.
    ///
    /// # Errors
    ///
    /// Returns a provider error without exposing arbitrary filesystem access to
    /// Rhai.
    fn load(&self, name: &str) -> Result<AssetData, String>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryAssetProvider {
    assets: BTreeMap<String, AssetData>,
}

impl InMemoryAssetProvider {
    #[must_use]
    pub fn new(assets: BTreeMap<String, AssetData>) -> Self {
        Self { assets }
    }
}

impl AssetProvider for InMemoryAssetProvider {
    fn load(&self, name: &str) -> Result<AssetData, String> {
        self.assets
            .get(name)
            .cloned()
            .ok_or_else(|| format!("asset `{name}` does not exist"))
    }
}

#[derive(Clone, Debug)]
pub struct DirectoryAssetProvider {
    root: PathBuf,
}

impl DirectoryAssetProvider {
    /// Restrict a provider to an existing canonical directory.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::Io`] when the root cannot be canonicalized.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, AssetError> {
        let path = root.as_ref();
        Ok(Self {
            root: path.canonicalize().map_err(|source| AssetError::Io {
                path: path.to_path_buf(),
                source,
            })?,
        })
    }
}

impl AssetProvider for DirectoryAssetProvider {
    fn load(&self, name: &str) -> Result<AssetData, String> {
        let direct = self.root.join(name);
        let path = if direct.exists() {
            direct
        } else {
            [
                "svg", "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff",
            ]
            .into_iter()
            .map(|extension| self.root.join(name).with_extension(extension))
            .find(|candidate| candidate.exists())
            .ok_or_else(|| format!("asset `{name}` does not exist"))?
        };
        let canonical = path.canonicalize().map_err(|error| error.to_string())?;
        if !canonical.starts_with(&self.root) {
            return Err("asset escaped provider root".to_owned());
        }
        let mime_type = mime_for_path(&canonical)
            .ok_or_else(|| "asset has an unsupported image extension".to_owned())?;
        let bytes = fs::read(&canonical).map_err(|error| error.to_string())?;
        Ok(AssetData {
            mime_type: mime_type.to_owned(),
            bytes,
        })
    }
}

fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "svg" => Some("image/svg+xml"),
        "bmp" => Some("image/bmp"),
        "tif" | "tiff" => Some("image/tiff"),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageHandle {
    opaque: OpaqueHandle,
    asset: AssetId,
}

impl ImageHandle {
    #[must_use]
    pub fn opaque(&self) -> &OpaqueHandle {
        &self.opaque
    }

    #[must_use]
    pub fn asset(&self) -> &AssetId {
        &self.asset
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ImageDecodeHandle(u64);

impl CustomType for ImageDecodeHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("ImageDecodeHandle")
            .with_fn("to_string", |handle: &mut Self| {
                format!("image-decode#{}", handle.0)
            });
    }
}

#[derive(Clone, Debug)]
struct PendingImageDecode {
    asset: AssetId,
    scope: AsyncScope,
    generation: ScriptGeneration,
    success: ScriptCallback,
    error: ScriptCallback,
    canceled: Arc<AtomicBool>,
}

struct ImageDecodeMessage {
    id: u64,
    result: Result<PreparedImageData, String>,
}

struct PreparedImageData {
    data: StoredAssetData,
    image: Arc<Image>,
}

#[derive(Clone, Debug)]
struct StoredAssetData {
    mime_type: String,
    content: StoredAssetContent,
}

#[derive(Clone, Debug)]
enum StoredAssetContent {
    Svg(Arc<str>),
    Raster,
}

impl StoredAssetData {
    fn from_validated(data: AssetData, format: ImageFormat) -> Result<Self, AssetError> {
        let content = if format == ImageFormat::Svg {
            StoredAssetContent::Svg(Arc::from(
                String::from_utf8(data.bytes).map_err(|_| AssetError::InvalidSvg)?,
            ))
        } else {
            StoredAssetContent::Raster
        };
        Ok(Self {
            mime_type: data.mime_type,
            content,
        })
    }

    fn svg_source(&self) -> Option<Arc<str>> {
        match &self.content {
            StoredAssetContent::Svg(source) => Some(Arc::clone(source)),
            StoredAssetContent::Raster => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SvgRasterSource {
    source: Arc<str>,
    color: Option<u32>,
}

impl SvgRasterSource {
    fn new(source: Arc<str>, color: Option<Rgba8>) -> Self {
        Self {
            source,
            color: color.map(Rgba8::as_rgba_hex),
        }
    }

    fn source_bytes(&self) -> usize {
        self.source.len()
    }

    fn color(&self) -> Option<Rgba8> {
        self.color.map(Rgba8::from_rgba_hex)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvgCacheStats {
    pub entries: usize,
    pub bytes: usize,
    pub max_entries: usize,
    pub max_bytes: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

#[derive(Clone, Debug)]
struct SvgVariantCacheEntry {
    bytes: usize,
    last_access: u64,
    state: SvgVariantCacheState,
}

#[derive(Clone, Debug)]
enum SvgVariantCacheState {
    Pending {
        token: u64,
        canceled: Arc<AtomicBool>,
    },
    Ready(Result<Arc<RenderImage>, ImageCacheError>),
}

struct SvgVariantMessage {
    source: SvgRasterSource,
    token: u64,
    result: Result<Arc<RenderImage>, ImageCacheError>,
}

struct SvgVariantCache {
    entries: BTreeMap<SvgRasterSource, SvgVariantCacheEntry>,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
    access_clock: u64,
    next_token: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
    sender: Sender<SvgVariantMessage>,
    receiver: Receiver<SvgVariantMessage>,
}

impl Default for SvgVariantCache {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self {
            entries: BTreeMap::new(),
            bytes: 0,
            max_entries: SVG_VARIANT_CACHE_MAX_ENTRIES,
            max_bytes: SVG_VARIANT_CACHE_MAX_BYTES,
            access_clock: 0,
            next_token: 1,
            hits: 0,
            misses: 0,
            evictions: 0,
            sender,
            receiver,
        }
    }
}

impl SvgVariantCache {
    #[cfg(test)]
    fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        let mut cache = Self::default();
        cache.max_entries = max_entries;
        cache.max_bytes = max_bytes;
        cache
    }

    fn drain(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            let Some(entry) = self.entries.get_mut(&message.source) else {
                continue;
            };
            let SvgVariantCacheState::Pending { token, canceled } = &entry.state else {
                continue;
            };
            if *token != message.token || canceled.load(Ordering::Acquire) {
                continue;
            }
            let rendered_bytes = message
                .result
                .as_ref()
                .ok()
                .and_then(|image| image.as_bytes(0))
                .map_or(0, <[u8]>::len);
            let new_bytes = message.source.source_bytes().saturating_add(rendered_bytes);
            self.bytes = self.bytes.saturating_sub(entry.bytes);
            entry.bytes = new_bytes;
            entry.state = SvgVariantCacheState::Ready(message.result);
            self.bytes = self.bytes.saturating_add(new_bytes);
        }
    }

    fn clear(&mut self) {
        for entry in self.entries.values() {
            if let SvgVariantCacheState::Pending { canceled, .. } = &entry.state {
                canceled.store(true, Ordering::Release);
            }
        }
        self.evictions = self
            .evictions
            .saturating_add(u64::try_from(self.entries.len()).unwrap_or(u64::MAX));
        self.entries.clear();
        self.bytes = 0;
        while self.receiver.try_recv().is_ok() {}
    }

    fn access(&mut self, source: &SvgRasterSource) -> SvgVariantAccess {
        self.drain();
        self.access_clock = self.access_clock.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(source) {
            entry.last_access = self.access_clock;
            self.hits = self.hits.saturating_add(1);
            let access = match &entry.state {
                SvgVariantCacheState::Pending { .. } => SvgVariantAccess::Pending,
                SvgVariantCacheState::Ready(result) => SvgVariantAccess::Ready(result.clone()),
            };
            self.enforce_limits(Some(source));
            return access;
        }
        let bytes = source.source_bytes();
        let token = self.next_token;
        self.next_token = self.next_token.saturating_add(1);
        let canceled = Arc::new(AtomicBool::new(false));
        self.entries.insert(
            source.clone(),
            SvgVariantCacheEntry {
                bytes,
                last_access: self.access_clock,
                state: SvgVariantCacheState::Pending {
                    token,
                    canceled: Arc::clone(&canceled),
                },
            },
        );
        self.bytes = self.bytes.saturating_add(bytes);
        self.misses = self.misses.saturating_add(1);
        self.enforce_limits(Some(source));
        SvgVariantAccess::Start {
            token,
            canceled,
            sender: self.sender.clone(),
        }
    }

    fn enforce_limits(&mut self, protected: Option<&SvgRasterSource>) {
        while self.entries.len() > self.max_entries || self.bytes > self.max_bytes {
            let candidate = self
                .entries
                .iter()
                .filter(|(source, _)| protected != Some(*source))
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(source, _)| source.clone())
                .or_else(|| protected.cloned());
            let Some(candidate) = candidate else {
                break;
            };
            let Some(entry) = self.entries.remove(&candidate) else {
                break;
            };
            if let SvgVariantCacheState::Pending { canceled, .. } = entry.state {
                canceled.store(true, Ordering::Release);
            }
            self.bytes = self.bytes.saturating_sub(entry.bytes);
            self.evictions = self.evictions.saturating_add(1);
        }
    }

    fn stats(&mut self) -> SvgCacheStats {
        self.drain();
        self.enforce_limits(None);
        SvgCacheStats {
            entries: self.entries.len(),
            bytes: self.bytes,
            max_entries: self.max_entries,
            max_bytes: self.max_bytes,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        }
    }
}

impl Drop for SvgVariantCache {
    fn drop(&mut self) {
        self.clear();
    }
}

enum SvgVariantAccess {
    Start {
        token: u64,
        canceled: Arc<AtomicBool>,
        sender: Sender<SvgVariantMessage>,
    },
    Pending,
    Ready(Result<Arc<RenderImage>, ImageCacheError>),
}

#[derive(Clone, Debug)]
pub(crate) struct ImageDecodeSnapshot {
    pending: BTreeMap<u64, PendingImageDecode>,
    deferred: BTreeMap<u64, PendingImageDecode>,
    transaction_depth: usize,
}

fn defer_or_cancel_decode(inner: &mut AssetRegistryInner, id: u64, pending: PendingImageDecode) {
    if inner.transaction_depth > 0 {
        inner.deferred_decode_cancellations.insert(id, pending);
    } else {
        pending.canceled.store(true, Ordering::Release);
    }
}

fn cancel_decode_ids(inner: &mut AssetRegistryInner, ids: impl IntoIterator<Item = u64>) {
    for id in ids {
        if let Some(pending) = inner.pending_decodes.remove(&id) {
            defer_or_cancel_decode(inner, id, pending);
        }
    }
}

struct AssetRegistryInner {
    providers: BTreeMap<String, Box<dyn AssetProvider>>,
    by_asset: BTreeMap<AssetId, ImageHandle>,
    images: BTreeMap<u64, Arc<Image>>,
    image_data: BTreeMap<u64, StoredAssetData>,
    svg_variants: SvgVariantCache,
    next_id: u64,
    next_decode_id: u64,
    pending_decodes: BTreeMap<u64, PendingImageDecode>,
    deferred_decode_cancellations: BTreeMap<u64, PendingImageDecode>,
    transaction_depth: usize,
    decode_sender: Sender<ImageDecodeMessage>,
    decode_receiver: Receiver<ImageDecodeMessage>,
}

impl Default for AssetRegistryInner {
    fn default() -> Self {
        let (decode_sender, decode_receiver) = channel();
        Self {
            providers: BTreeMap::new(),
            by_asset: BTreeMap::new(),
            images: BTreeMap::new(),
            image_data: BTreeMap::new(),
            svg_variants: SvgVariantCache::default(),
            next_id: 0,
            next_decode_id: 1,
            pending_decodes: BTreeMap::new(),
            deferred_decode_cancellations: BTreeMap::new(),
            transaction_depth: 0,
            decode_sender,
            decode_receiver,
        }
    }
}

#[derive(Clone, Default)]
pub struct AssetRegistry {
    inner: Rc<RefCell<AssetRegistryInner>>,
}

impl std::fmt::Debug for AssetRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.try_borrow() {
            Ok(inner) => formatter
                .debug_struct("AssetRegistry")
                .field("providers", &inner.providers.keys().collect::<Vec<_>>())
                .field("cached", &inner.by_asset.keys().collect::<Vec<_>>())
                .finish_non_exhaustive(),
            Err(_) => formatter.write_str("AssetRegistry(<borrowed>)"),
        }
    }
}

impl AssetRegistry {
    pub(crate) fn begin_transaction(&self) -> Result<ImageDecodeSnapshot, AssetError> {
        let snapshot = self.decode_snapshot()?;
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        inner.transaction_depth = inner.transaction_depth.saturating_add(1);
        Ok(snapshot)
    }

    pub(crate) fn commit_transaction(&self) -> Result<(), AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        inner.transaction_depth = inner.transaction_depth.saturating_sub(1);
        if inner.transaction_depth == 0 {
            for pending in std::mem::take(&mut inner.deferred_decode_cancellations).into_values() {
                pending.canceled.store(true, Ordering::Release);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one logical asset namespace.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError`] for invalid or duplicate namespaces.
    pub fn register(
        &self,
        namespace: impl Into<String>,
        provider: impl AssetProvider + 'static,
    ) -> Result<(), AssetError> {
        let namespace = namespace.into();
        if !valid_segment(&namespace) {
            return Err(AssetError::InvalidNamespace(namespace));
        }
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        if inner.providers.contains_key(&namespace) {
            return Err(AssetError::DuplicateNamespace(namespace));
        }
        inner.providers.insert(namespace, Box::new(provider));
        Ok(())
    }

    /// Reload every cached image in a provider namespace while preserving
    /// existing opaque handle identities.
    ///
    /// # Errors
    ///
    /// Returns provider, MIME, empty-data, or borrow errors. Validation is
    /// transactional: existing cached bytes remain active on failure.
    pub fn refresh_namespace(&self, namespace: &str) -> Result<usize, AssetError> {
        let refreshed = {
            let inner = self.inner.try_borrow().map_err(|_| AssetError::Borrowed)?;
            let provider = inner
                .providers
                .get(namespace)
                .ok_or_else(|| AssetError::UnknownNamespace(namespace.to_owned()))?;
            inner
                .by_asset
                .iter()
                .filter(|(asset, _)| asset.split().0 == namespace)
                .map(|(asset, handle)| {
                    let (_, name) = asset.split();
                    let data = provider
                        .load(name)
                        .map_err(|message| AssetError::Provider {
                            asset: asset.clone(),
                            message,
                        })?;
                    let format = ImageFormat::from_mime_type(&data.mime_type)
                        .ok_or_else(|| AssetError::UnsupportedMime(data.mime_type.clone()))?;
                    if data.bytes.is_empty() {
                        return Err(AssetError::Empty(asset.clone()));
                    }
                    let image = prepare_image(format, &data.bytes, None)?;
                    let stored = StoredAssetData::from_validated(data, format)?;
                    Ok((handle.opaque.id(), stored, image))
                })
                .collect::<Result<Vec<_>, AssetError>>()?
        };
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        for (id, data, image) in &refreshed {
            inner.images.insert(*id, Arc::clone(image));
            inner.image_data.insert(*id, data.clone());
        }
        if !refreshed.is_empty() {
            inner.svg_variants.clear();
        }
        Ok(refreshed.len())
    }

    /// Load, validate, decode, and cache an image by logical ID.
    ///
    /// # Errors
    ///
    /// Returns provider, MIME, decode-boundary, or borrow errors.
    pub fn load_image(&self, id: &AssetId) -> Result<ImageHandle, AssetError> {
        if let Some(handle) = self
            .inner
            .try_borrow()
            .map_err(|_| AssetError::Borrowed)?
            .by_asset
            .get(id)
            .cloned()
        {
            return Ok(handle);
        }
        let (namespace, name) = id.split();
        let data = {
            let inner = self.inner.try_borrow().map_err(|_| AssetError::Borrowed)?;
            inner
                .providers
                .get(namespace)
                .ok_or_else(|| AssetError::UnknownNamespace(namespace.to_owned()))?
                .load(name)
                .map_err(|message| AssetError::Provider {
                    asset: id.clone(),
                    message,
                })?
        };
        let format = ImageFormat::from_mime_type(&data.mime_type)
            .ok_or_else(|| AssetError::UnsupportedMime(data.mime_type.clone()))?;
        if data.bytes.is_empty() {
            return Err(AssetError::Empty(id.clone()));
        }
        let image = prepare_image(format, &data.bytes, None)?;
        let data = StoredAssetData::from_validated(data, format)?;
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        inner.next_id = inner.next_id.saturating_add(1);
        let image_id = inner.next_id;
        let handle = ImageHandle {
            opaque: OpaqueHandle::new("image", image_id),
            asset: id.clone(),
        };
        inner.images.insert(image_id, image);
        inner.image_data.insert(image_id, data);
        inner.by_asset.insert(id.clone(), handle.clone());
        Ok(handle)
    }

    /// Preload a deterministic set of declared image assets.
    ///
    /// # Errors
    ///
    /// Returns the first provider/MIME/data error without hiding which asset
    /// failed. Successfully loaded earlier entries remain cached and reusable.
    pub fn preload_images(
        &self,
        ids: impl IntoIterator<Item = AssetId>,
    ) -> Result<usize, AssetError> {
        let mut loaded = 0;
        for id in ids {
            self.load_image(&id)?;
            loaded += 1;
        }
        Ok(loaded)
    }

    /// Resolve a declarative asset only if preparation already cached it.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::NotPreloaded`] instead of performing provider I/O.
    pub fn cached_image(&self, id: &AssetId) -> Result<ImageHandle, AssetError> {
        self.inner
            .try_borrow()
            .map_err(|_| AssetError::Borrowed)?
            .by_asset
            .get(id)
            .cloned()
            .ok_or_else(|| AssetError::NotPreloaded(id.clone()))
    }

    /// Start background validation/decoding for a logical image while keeping
    /// callbacks and runtime ownership on the foreground thread.
    ///
    /// # Errors
    ///
    /// Returns provider, borrow, or worker-spawn errors.
    pub fn start_image_decode(
        &self,
        id: &AssetId,
        scope: AsyncScope,
        generation: ScriptGeneration,
        success: ScriptCallback,
        error: ScriptCallback,
    ) -> Result<ImageDecodeHandle, AssetError> {
        let (namespace, name) = id.split();
        let data = {
            let inner = self.inner.try_borrow().map_err(|_| AssetError::Borrowed)?;
            inner
                .providers
                .get(namespace)
                .ok_or_else(|| AssetError::UnknownNamespace(namespace.to_owned()))?
                .load(name)
                .map_err(|message| AssetError::Provider {
                    asset: id.clone(),
                    message,
                })?
        };
        let (decode_id, sender, canceled) = {
            let mut inner = self
                .inner
                .try_borrow_mut()
                .map_err(|_| AssetError::Borrowed)?;
            let decode_id = inner.next_decode_id;
            inner.next_decode_id = inner.next_decode_id.saturating_add(1);
            let canceled = Arc::new(AtomicBool::new(false));
            inner.pending_decodes.insert(
                decode_id,
                PendingImageDecode {
                    asset: id.clone(),
                    scope,
                    generation,
                    success,
                    error,
                    canceled: canceled.clone(),
                },
            );
            (decode_id, inner.decode_sender.clone(), canceled)
        };
        let worker_cancel = canceled.clone();
        if let Err(source) = std::thread::Builder::new()
            .name("gpui-rhai-image-decode".to_owned())
            .spawn(move || {
                if worker_cancel.load(Ordering::Acquire) {
                    return;
                }
                let result = decode_asset_data(data);
                if !worker_cancel.load(Ordering::Acquire) {
                    let _ = sender.send(ImageDecodeMessage {
                        id: decode_id,
                        result,
                    });
                }
            })
        {
            canceled.store(true, Ordering::Release);
            self.inner
                .try_borrow_mut()
                .map_err(|_| AssetError::Borrowed)?
                .pending_decodes
                .remove(&decode_id);
            return Err(AssetError::DecodeSpawn(source));
        }
        Ok(ImageDecodeHandle(decode_id))
    }

    /// Cancel one pending decode. Completed or unknown handles return `false`.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::Borrowed`] during conflicting registry access.
    pub fn cancel_image_decode(&self, handle: ImageDecodeHandle) -> Result<bool, AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        let Some(pending) = inner.pending_decodes.remove(&handle.0) else {
            return Ok(false);
        };
        defer_or_cancel_decode(&mut inner, handle.0, pending);
        Ok(true)
    }

    /// Cancel pending image work owned by a closing window and its components.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::Borrowed`] during conflicting registry access.
    pub fn cancel_window_scope(
        &self,
        window: &str,
        component: &ComponentInstancePath,
    ) -> Result<(), AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        let remove = inner
            .pending_decodes
            .iter()
            .filter_map(|(id, pending)| {
                let remove = match &pending.scope {
                    AsyncScope::Window(id) => id == window,
                    AsyncScope::Component(_) | AsyncScope::Effect { .. } => {
                        pending.scope.is_within_component(component)
                    }
                    AsyncScope::App => false,
                };
                remove.then_some(*id)
            })
            .collect::<Vec<_>>();
        cancel_decode_ids(&mut inner, remove);
        Ok(())
    }

    /// Cancel pending image work owned by a removed component subtree.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::Borrowed`] during conflicting registry access.
    pub fn cancel_component_scope(
        &self,
        component: &ComponentInstancePath,
    ) -> Result<(), AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        let remove = inner
            .pending_decodes
            .iter()
            .filter_map(|(id, pending)| pending.scope.is_within_component(component).then_some(*id))
            .collect::<Vec<_>>();
        cancel_decode_ids(&mut inner, remove);
        Ok(())
    }

    /// Cancel pending image work owned by one exact asynchronous scope.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::Borrowed`] during conflicting registry access.
    pub fn cancel_scope(&self, scope: &AsyncScope) -> Result<(), AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        let remove = inner
            .pending_decodes
            .iter()
            .filter_map(|(id, pending)| (&pending.scope == scope).then_some(*id))
            .collect::<Vec<_>>();
        cancel_decode_ids(&mut inner, remove);
        Ok(())
    }

    /// Cancel pending decode work from obsolete script generations.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::Borrowed`] during conflicting registry access.
    pub fn retain_decode_generation(&self, generation: ScriptGeneration) -> Result<(), AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        inner.pending_decodes.retain(|_, pending| {
            let retain = pending.generation == generation;
            if !retain {
                pending.canceled.store(true, Ordering::Release);
            }
            retain
        });
        Ok(())
    }

    /// Install completed images and return generation-safe callback deliveries.
    ///
    /// # Errors
    ///
    /// Returns a borrow error if called during another asset registry access.
    pub fn drain_image_decodes(
        &self,
        generation: ScriptGeneration,
    ) -> Result<Vec<AsyncDelivery>, AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        let mut messages = Vec::new();
        while let Ok(message) = inner.decode_receiver.try_recv() {
            messages.push(message);
        }
        let mut deliveries = Vec::new();
        for message in messages {
            let Some(pending) = inner.pending_decodes.remove(&message.id) else {
                continue;
            };
            if pending.canceled.load(Ordering::Acquire) || pending.generation != generation {
                continue;
            }
            let (callback, payload) = match message.result {
                Ok(prepared) => {
                    let handle = install_decoded_image(&mut inner, pending.asset, prepared);
                    (pending.success, UiValue::Handle(handle.opaque().clone()))
                }
                Err(message) => (pending.error, UiValue::String(message)),
            };
            deliveries.push(AsyncDelivery {
                callback,
                payload,
                scope: pending.scope,
            });
        }
        Ok(deliveries)
    }

    #[must_use]
    pub fn pending_decode_count(&self) -> usize {
        self.inner
            .try_borrow()
            .map_or(0, |inner| inner.pending_decodes.len())
    }

    pub(crate) fn decode_snapshot(&self) -> Result<ImageDecodeSnapshot, AssetError> {
        let inner = self.inner.try_borrow().map_err(|_| AssetError::Borrowed)?;
        Ok(ImageDecodeSnapshot {
            pending: inner.pending_decodes.clone(),
            deferred: inner.deferred_decode_cancellations.clone(),
            transaction_depth: inner.transaction_depth,
        })
    }

    pub(crate) fn restore_decode_snapshot(
        &self,
        snapshot: ImageDecodeSnapshot,
    ) -> Result<(), AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        for (id, pending) in inner
            .pending_decodes
            .iter()
            .chain(inner.deferred_decode_cancellations.iter())
        {
            if !snapshot.pending.contains_key(id) && !snapshot.deferred.contains_key(id) {
                pending.canceled.store(true, Ordering::Release);
            }
        }
        inner.pending_decodes = snapshot.pending;
        inner.deferred_decode_cancellations = snapshot.deferred;
        inner.transaction_depth = snapshot.transaction_depth;
        Ok(())
    }

    /// Resolve an opaque image handle into a GPUI image source.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError`] for wrong-kind, unknown, or borrow errors.
    pub fn image_source(&self, handle: &OpaqueHandle) -> Result<ImageSource, AssetError> {
        self.image_source_tinted(handle, None)
    }

    /// Resolve an image and apply semantic `currentColor` inheritance to SVG
    /// bytes. Raster images reuse their original cache entry.
    ///
    /// # Errors
    ///
    /// Returns wrong-kind, unknown, malformed-SVG, or borrow errors.
    pub fn image_source_tinted(
        &self,
        handle: &OpaqueHandle,
        color: Option<Rgba8>,
    ) -> Result<ImageSource, AssetError> {
        if handle.kind() != "image" {
            return Err(AssetError::WrongHandleKind(handle.kind().to_owned()));
        }
        let inner = self.inner.try_borrow().map_err(|_| AssetError::Borrowed)?;
        let Some(data) = inner.image_data.get(&handle.id()) else {
            return Err(AssetError::UnknownHandle(handle.id()));
        };
        if data.mime_type == "image/svg+xml"
            && let Some(color) = color
            && let Some(source) = data.svg_source()
            && svg_uses_external_current_color(&source)
        {
            drop(inner);
            return Ok(self.svg_variant_image_source(SvgRasterSource::new(source, Some(color))));
        }
        let image = inner
            .images
            .get(&handle.id())
            .cloned()
            .ok_or(AssetError::UnknownHandle(handle.id()))?;
        Ok(ImageSource::Image(image))
    }

    pub(crate) fn inline_svg_source(&self, source: Arc<str>, color: Option<Rgba8>) -> ImageSource {
        self.svg_variant_image_source(SvgRasterSource::new(source, color))
    }

    fn svg_variant_image_source(&self, source: SvgRasterSource) -> ImageSource {
        let registry = self.clone();
        ImageSource::from(move |window: &mut gpui::Window, cx: &mut gpui::App| {
            let access = {
                let Ok(mut inner) = registry.inner.try_borrow_mut() else {
                    return Some(Err(ImageCacheError::Asset(
                        AssetError::Borrowed.to_string().into(),
                    )));
                };
                inner.svg_variants.access(&source)
            };
            match access {
                SvgVariantAccess::Ready(result) => Some(result),
                SvgVariantAccess::Pending => {
                    window.request_animation_frame();
                    None
                }
                SvgVariantAccess::Start {
                    token,
                    canceled,
                    sender,
                } => {
                    let source = source.clone();
                    cx.background_spawn(async move {
                        if canceled.load(Ordering::Acquire) {
                            return;
                        }
                        let result = svg_render_image(source.source.as_bytes(), source.color())
                            .map_err(|error| ImageCacheError::Asset(error.to_string().into()));
                        if !canceled.load(Ordering::Acquire) {
                            let _ = sender.send(SvgVariantMessage {
                                source,
                                token,
                                result,
                            });
                        }
                    })
                    .detach();
                    window.request_animation_frame();
                    None
                }
            }
        })
    }

    /// Return bounded SVG variant-cache usage and lifetime counters.
    ///
    /// The byte count includes retained SVG source text and ready BGRA pixels.
    /// Pending or canceled background work is never counted as a ready image.
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::Borrowed`] during conflicting registry access.
    pub fn svg_cache_stats(&self) -> Result<SvgCacheStats, AssetError> {
        self.inner
            .try_borrow_mut()
            .map(|mut inner| inner.svg_variants.stats())
            .map_err(|_| AssetError::Borrowed)
    }
}

fn install_decoded_image(
    inner: &mut AssetRegistryInner,
    asset: AssetId,
    prepared: PreparedImageData,
) -> ImageHandle {
    if let Some(handle) = inner.by_asset.get(&asset) {
        return handle.clone();
    }
    inner.next_id = inner.next_id.saturating_add(1);
    let image_id = inner.next_id;
    let handle = ImageHandle {
        opaque: OpaqueHandle::new("image", image_id),
        asset: asset.clone(),
    };
    inner.images.insert(image_id, prepared.image);
    inner.image_data.insert(image_id, prepared.data);
    inner.by_asset.insert(asset, handle.clone());
    handle
}

fn prepare_image(
    format: ImageFormat,
    bytes: &[u8],
    color: Option<Rgba8>,
) -> Result<Arc<Image>, AssetError> {
    if format == ImageFormat::Svg {
        svg_image(bytes, color)
    } else {
        Ok(Arc::new(Image::from_bytes(format, bytes.to_vec())))
    }
}

fn decode_asset_data(data: AssetData) -> Result<PreparedImageData, String> {
    if data.bytes.is_empty() {
        return Err("image data is empty".to_owned());
    }
    let format = ImageFormat::from_mime_type(&data.mime_type)
        .ok_or_else(|| format!("unsupported image MIME type `{}`", data.mime_type))?;
    if format == ImageFormat::Svg {
        let source = std::str::from_utf8(&data.bytes)
            .map_err(|_| "SVG image is not valid UTF-8".to_owned())?;
        if !source.contains("<svg") {
            return Err("SVG image has no root element".to_owned());
        }
    } else {
        let raster_format = raster_image_format(format)
            .ok_or_else(|| "unsupported raster image format".to_owned())?;
        image::load_from_memory_with_format(&data.bytes, raster_format)
            .map_err(|error| format!("raster image decode failed: {error}"))?;
    }
    let image = prepare_image(format, &data.bytes, None).map_err(|error| error.to_string())?;
    let data = StoredAssetData::from_validated(data, format).map_err(|error| error.to_string())?;
    Ok(PreparedImageData { data, image })
}

fn raster_image_format(format: ImageFormat) -> Option<image::ImageFormat> {
    match format {
        ImageFormat::Png => Some(image::ImageFormat::Png),
        ImageFormat::Jpeg => Some(image::ImageFormat::Jpeg),
        ImageFormat::Webp => Some(image::ImageFormat::WebP),
        ImageFormat::Gif => Some(image::ImageFormat::Gif),
        ImageFormat::Bmp => Some(image::ImageFormat::Bmp),
        ImageFormat::Tiff => Some(image::ImageFormat::Tiff),
        ImageFormat::Svg | ImageFormat::Ico | ImageFormat::Pnm => None,
    }
}

pub(crate) fn svg_image(bytes: &[u8], color: Option<Rgba8>) -> Result<Arc<Image>, AssetError> {
    // GPUI 0.2.2's ImageSource::Image SVG decoder publishes premultiplied RGBA
    // bytes as a BGRA RenderImage. Rasterizing the complete document to PNG
    // here preserves every SVG color/alpha operation and then uses GPUI's
    // correct PNG RGBA-to-BGRA path. Remove this adapter as one unit when the
    // pinned GPUI SVG decoder is fixed.
    let pixmap = svg_pixmap(bytes, color)?;
    let png = pixmap.encode_png().map_err(|_| AssetError::InvalidSvg)?;
    Ok(Arc::new(Image::from_bytes(ImageFormat::Png, png)))
}

fn svg_render_image(bytes: &[u8], color: Option<Rgba8>) -> Result<Arc<RenderImage>, AssetError> {
    let pixmap = svg_pixmap(bytes, color)?;
    let png = pixmap.encode_png().map_err(|_| AssetError::InvalidSvg)?;
    let mut bgra = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map_err(|_| AssetError::InvalidSvg)?
        .into_rgba8();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(bgra)])))
}

fn svg_pixmap(bytes: &[u8], color: Option<Rgba8>) -> Result<resvg::tiny_skia::Pixmap, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|_| AssetError::InvalidSvg)?;
    let source = inherited_svg_color(source, color)?;
    let tree = usvg::Tree::from_str(&source, &svg_options()).map_err(|_| AssetError::InvalidSvg)?;
    let width = svg_raster_dimension(tree.size().width()).ok_or(AssetError::InvalidSvg)?;
    let height = svg_raster_dimension(tree.size().height()).ok_or(AssetError::InvalidSvg)?;
    if u64::from(width).saturating_mul(u64::from(height)) > MAX_SVG_RASTER_PIXELS {
        return Err(AssetError::InvalidSvg);
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or(AssetError::InvalidSvg)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

pub(crate) fn svg_options() -> usvg::Options<'static> {
    static FONT_DB: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
        let mut database = usvg::fontdb::Database::new();
        database.load_system_fonts();
        let first_family = database
            .faces()
            .find_map(|face| face.families.first().map(|family| family.0.clone()));
        if let Some(family) = available_font_family(
            &database,
            &[
                "Arial",
                "Helvetica",
                "DejaVu Sans",
                "Liberation Sans",
                "Noto Sans",
            ],
        )
        .or_else(|| first_family.clone())
        {
            database.set_sans_serif_family(family.clone());
            database.set_cursive_family(family.clone());
            database.set_fantasy_family(family);
        }
        if let Some(family) = available_font_family(
            &database,
            &[
                "Times New Roman",
                "Times",
                "DejaVu Serif",
                "Liberation Serif",
                "Noto Serif",
            ],
        )
        .or_else(|| first_family.clone())
        {
            database.set_serif_family(family);
        }
        if let Some(family) = available_font_family(
            &database,
            &[
                "Courier New",
                "Menlo",
                "DejaVu Sans Mono",
                "Liberation Mono",
                "Noto Sans Mono",
            ],
        )
        .or(first_family)
        {
            database.set_monospace_family(family);
        }
        Arc::new(database)
    });
    let default_font_resolver = usvg::FontResolver::default_font_selector();
    let font_resolver = Box::new(
        move |font: &usvg::Font, database: &mut Arc<usvg::fontdb::Database>| {
            if database.is_empty() {
                *database = Arc::clone(&FONT_DB);
            }
            default_font_resolver(font, database)
        },
    );
    usvg::Options {
        font_resolver: usvg::FontResolver {
            select_font: font_resolver,
            select_fallback: usvg::FontResolver::default_fallback_selector(),
        },
        ..Default::default()
    }
}

fn available_font_family(database: &usvg::fontdb::Database, candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        database.faces().find_map(|face| {
            face.families
                .iter()
                .find(|family| family.0.eq_ignore_ascii_case(candidate))
                .map(|family| family.0.clone())
        })
    })
}

fn svg_raster_dimension(value: f32) -> Option<u32> {
    let value = value.ceil();
    if !value.is_finite() || !(1.0..=MAX_SVG_RASTER_DIMENSION_F32).contains(&value) {
        return None;
    }
    let parsed = value.to_string().parse::<u32>().ok()?;
    (parsed <= MAX_SVG_RASTER_DIMENSION).then_some(parsed)
}

fn inherited_svg_color(source: &str, color: Option<Rgba8>) -> Result<String, AssetError> {
    let document = roxmltree::Document::parse(source).map_err(|_| AssetError::InvalidSvg)?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" {
        return Err(AssetError::InvalidSvg);
    }
    let Some(color) = color else {
        return Ok(source.to_owned());
    };
    if let Some(attribute) = root
        .attributes()
        .find(|attribute| attribute.name() == "color")
    {
        if is_explicit_svg_color_value(attribute.value()) {
            return Ok(source.to_owned());
        }
        let range = attribute.range();
        let mut inherited = String::with_capacity(source.len().saturating_add(8));
        inherited.push_str(&source[..range.start]);
        let _ = write!(inherited, "color=\"#{:08x}\"", color.as_rgba_hex());
        inherited.push_str(&source[range.end..]);
        return Ok(inherited);
    }
    let root_start = root.range().start;
    let bytes = source.as_bytes();
    let mut name_end = root_start.saturating_add(1);
    while bytes
        .get(name_end)
        .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'>'))
    {
        name_end = name_end.saturating_add(1);
    }
    if name_end <= root_start.saturating_add(1) || name_end > source.len() {
        return Err(AssetError::InvalidSvg);
    }
    let mut inherited = String::with_capacity(source.len().saturating_add(20));
    inherited.push_str(&source[..name_end]);
    let _ = write!(inherited, " color=\"#{:08x}\"", color.as_rgba_hex());
    inherited.push_str(&source[name_end..]);
    Ok(inherited)
}

fn svg_uses_external_current_color(source: &str) -> bool {
    if !source
        .as_bytes()
        .windows(b"currentColor".len())
        .any(|candidate| candidate.eq_ignore_ascii_case(b"currentColor"))
    {
        return false;
    }
    let Ok(document) = roxmltree::Document::parse(source) else {
        return true;
    };
    let mut found_attribute_use = false;
    for node in document.descendants().filter(roxmltree::Node::is_element) {
        let uses_current_color = node.attributes().any(|attribute| {
            attribute
                .value()
                .as_bytes()
                .windows(b"currentColor".len())
                .any(|candidate| candidate.eq_ignore_ascii_case(b"currentColor"))
        });
        if !uses_current_color {
            continue;
        }
        found_attribute_use = true;
        let mut ancestor = Some(node);
        let mut locally_resolved = false;
        while let Some(element) = ancestor {
            if element_has_explicit_color(element) {
                locally_resolved = true;
                break;
            }
            ancestor = element.parent_element();
        }
        if !locally_resolved {
            return true;
        }
    }
    !found_attribute_use
}

fn element_has_explicit_color(node: roxmltree::Node<'_, '_>) -> bool {
    if node
        .attribute("color")
        .is_some_and(is_explicit_svg_color_value)
    {
        return true;
    }
    node.attribute("style").is_some_and(|style| {
        style.split(';').any(|declaration| {
            let Some((name, value)) = declaration.split_once(':') else {
                return false;
            };
            name.trim().eq_ignore_ascii_case("color") && is_explicit_svg_color_value(value)
        })
    })
}

fn is_explicit_svg_color_value(value: &str) -> bool {
    let value = value.trim();
    !value.eq_ignore_ascii_case("inherit")
        && !value.eq_ignore_ascii_case("currentColor")
        && !value.eq_ignore_ascii_case("unset")
        && !value.eq_ignore_ascii_case("revert")
}

fn valid_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

pub(crate) fn asset_id_from_script(value: &str) -> Result<AssetId, Box<rhai::EvalAltResult>> {
    AssetId::parse(value).map_err(|error| error.to_string().into())
}

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("asset ID `{0}` must be a namespaced logical path")]
    InvalidId(String),
    #[error("asset namespace `{0}` is invalid")]
    InvalidNamespace(String),
    #[error("asset namespace `{0}` is already registered")]
    DuplicateNamespace(String),
    #[error("asset namespace `{0}` is not registered")]
    UnknownNamespace(String),
    #[error("asset registry is already borrowed")]
    Borrowed,
    #[error("asset provider failed for `{asset:?}`: {message}")]
    Provider { asset: AssetId, message: String },
    #[error("image MIME type `{0}` is unsupported")]
    UnsupportedMime(String),
    #[error("image asset `{0:?}` is empty")]
    Empty(AssetId),
    #[error("SVG asset is not valid UTF-8 SVG data")]
    InvalidSvg,
    #[error("failed to spawn image decode worker: {0}")]
    DecodeSpawn(std::io::Error),
    #[error("opaque handle kind `{0}` is not an image")]
    WrongHandleKind(String),
    #[error("image handle `{0}` is unknown")]
    UnknownHandle(u64),
    #[error("declarative image asset `{0:?}` was not preloaded during preparation")]
    NotPreloaded(AssetId),
    #[error("asset I/O failed for `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    use rhai::FnPtr;

    #[derive(Clone)]
    struct MutableProvider(Rc<RefCell<AssetData>>);

    impl AssetProvider for MutableProvider {
        fn load(&self, _: &str) -> Result<AssetData, String> {
            Ok(self.0.borrow().clone())
        }
    }

    fn one_pixel_png() -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1)
            .write_to(&mut output, image::ImageFormat::Png)
            .unwrap();
        output.into_inner()
    }

    #[test]
    fn logical_ids_reject_urls_and_traversal() {
        for invalid in ["https://example.com/a.png", "/tmp/a.png", "core/../secret"] {
            assert!(AssetId::parse(invalid).is_err());
        }
        assert_eq!(AssetId::parse("core/check").unwrap().as_str(), "core/check");
    }

    #[test]
    fn provider_images_are_cached_behind_opaque_handles() {
        let registry = AssetRegistry::new();
        registry
            .register(
                "core",
                InMemoryAssetProvider::new(BTreeMap::from([(
                    "check".to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#
                            .to_vec(),
                    },
                )])),
            )
            .unwrap();
        let id = AssetId::parse("core/check").unwrap();
        let first = registry.load_image(&id).unwrap();
        let second = registry.load_image(&id).unwrap();
        assert_eq!(first, second);
        registry.image_source(first.opaque()).unwrap();
        assert!(matches!(
            registry.image_source(&OpaqueHandle::new("task", 1)),
            Err(AssetError::WrongHandleKind(_))
        ));
    }

    #[test]
    fn rejected_duplicate_provider_keeps_uncached_assets_from_the_original() {
        let registry = AssetRegistry::new();
        let data = AssetData {
            mime_type: "image/svg+xml".to_owned(),
            bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#.to_vec(),
        };
        registry
            .register(
                "app",
                InMemoryAssetProvider::new(BTreeMap::from([
                    ("first".to_owned(), data.clone()),
                    ("second".to_owned(), data),
                ])),
            )
            .unwrap();
        registry
            .load_image(&AssetId::parse("app/first").unwrap())
            .unwrap();
        assert!(matches!(
            registry.register("app", InMemoryAssetProvider::default()),
            Err(AssetError::DuplicateNamespace(_))
        ));
        registry
            .load_image(&AssetId::parse("app/second").unwrap())
            .unwrap();
    }

    #[test]
    fn namespace_refresh_preserves_handle_and_replaces_cached_bytes() {
        let data = Rc::new(RefCell::new(AssetData {
            mime_type: "image/svg+xml".to_owned(),
            bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" data-version="one"/>"#.to_vec(),
        }));
        let registry = AssetRegistry::new();
        registry
            .register("app", MutableProvider(Rc::clone(&data)))
            .unwrap();
        let id = AssetId::parse("app/icon").unwrap();
        let handle = registry.load_image(&id).unwrap();
        data.borrow_mut().bytes =
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" data-version="two"/>"#
                .to_vec();
        assert_eq!(registry.refresh_namespace("app").unwrap(), 1);
        assert_eq!(registry.load_image(&id).unwrap(), handle);
        assert_eq!(
            registry.inner.borrow().image_data[&handle.opaque.id()]
                .svg_source()
                .unwrap()
                .as_bytes(),
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" data-version="two"/>"#
        );
    }

    #[test]
    fn svg_current_color_uses_external_default_without_overriding_local_color() {
        let inherited = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="currentColor"/></svg>"#;
        assert!(svg_uses_external_current_color(
            std::str::from_utf8(inherited).unwrap()
        ));
        let image = svg_image(inherited, Some(Rgba8::from_rgba_hex(0x1234_ab80))).unwrap();
        assert_eq!(
            image::load_from_memory(&image.bytes)
                .unwrap()
                .into_rgba8()
                .into_raw(),
            [0x12, 0x34, 0xab, 0x80]
        );

        let local = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" color="#ff0000" fill="currentColor"/></svg>"##;
        assert!(!svg_uses_external_current_color(
            std::str::from_utf8(local).unwrap()
        ));
        let image = svg_image(local, Some(Rgba8::from_rgba_hex(0x00ff_00ff))).unwrap();
        assert_eq!(
            image::load_from_memory(&image.bytes)
                .unwrap()
                .into_rgba8()
                .into_raw(),
            [0xff, 0x00, 0x00, 0xff]
        );

        let root_inherit = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" color="inherit"><rect width="1" height="1" fill="currentColor"/></svg>"#;
        let image = svg_image(root_inherit, Some(Rgba8::from_rgba_hex(0x1234_abff))).unwrap();
        assert_eq!(
            image::load_from_memory(&image.bytes)
                .unwrap()
                .into_rgba8()
                .into_raw(),
            [0x12, 0x34, 0xab, 0xff]
        );
    }

    #[test]
    fn svg_raster_adapter_preserves_fixed_color_gradient_and_semantic_alpha() {
        let mixed = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="1" height="1" fill="currentColor"/><rect x="1" width="1" height="1" fill="#ff0000"/></svg>"##;
        let image = svg_image(mixed, Some(Rgba8::from_rgba_hex(0x1234_ab80))).unwrap();
        let pixels = image::load_from_memory(&image.bytes)
            .unwrap()
            .into_rgba8()
            .into_raw();
        assert_eq!(&pixels[0..4], &[0x12, 0x34, 0xab, 0x80]);
        assert_eq!(&pixels[4..8], &[0xff, 0x00, 0x00, 0xff]);

        let gradient = br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="1"><defs><linearGradient id="g"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs><rect width="4" height="1" fill="url(#g)"/></svg>"##;
        let image = svg_image(gradient, None).unwrap();
        let pixels = image::load_from_memory(&image.bytes)
            .unwrap()
            .into_rgba8()
            .into_raw();
        assert!(pixels[0] > pixels[2], "gradient must begin red: {pixels:?}");
        let last = &pixels[pixels.len() - 4..];
        assert!(last[2] > last[0], "gradient must end blue: {pixels:?}");
    }

    #[test]
    fn svg_variant_cache_is_lru_bounded_and_observable() {
        let mut cache = SvgVariantCache::with_limits(2, 120);
        let mut first_canceled = None;
        for index in 0..3 {
            let source = SvgRasterSource::new(
                Arc::from(format!(
                    "<svg width='1' height='1' data-index='{index}'>{}</svg>",
                    "x".repeat(20)
                )),
                None,
            );
            let SvgVariantAccess::Start { canceled, .. } = cache.access(&source) else {
                panic!("new SVG source must start one worker");
            };
            if index == 0 {
                first_canceled = Some(canceled);
            }
        }
        let stats = cache.stats();
        assert!(stats.entries <= 2);
        assert_eq!(stats.misses, 3);
        assert!(stats.evictions >= 1);
        assert!(first_canceled.unwrap().load(Ordering::Acquire));
        assert!(stats.bytes <= stats.max_bytes);
    }

    #[test]
    fn raster_decode_delivers_handle_on_foreground_drain() {
        let registry = AssetRegistry::new();
        registry
            .register(
                "app",
                InMemoryAssetProvider::new(BTreeMap::from([(
                    "pixel".to_owned(),
                    AssetData {
                        mime_type: "image/png".to_owned(),
                        bytes: one_pixel_png(),
                    },
                )])),
            )
            .unwrap();
        let generation = ScriptGeneration::initial();
        registry
            .start_image_decode(
                &AssetId::parse("app/pixel").unwrap(),
                AsyncScope::Component(crate::ComponentInstancePath::root("App", "root")),
                generation,
                ScriptCallback::try_from_fn_ptr(FnPtr::new("loaded").unwrap(), generation).unwrap(),
                ScriptCallback::try_from_fn_ptr(FnPtr::new("failed").unwrap(), generation).unwrap(),
            )
            .unwrap();
        let mut deliveries = Vec::new();
        for _ in 0..1_000 {
            deliveries = registry.drain_image_decodes(generation).unwrap();
            if !deliveries.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].callback.name(), "loaded");
        let UiValue::Handle(handle) = &deliveries[0].payload else {
            panic!("successful decode must deliver an image handle");
        };
        registry.image_source(handle).unwrap();
        assert_eq!(registry.pending_decode_count(), 0);
    }

    #[test]
    fn canceled_and_stale_image_decodes_do_not_deliver() {
        let registry = AssetRegistry::new();
        registry
            .register(
                "app",
                InMemoryAssetProvider::new(BTreeMap::from([(
                    "pixel".to_owned(),
                    AssetData {
                        mime_type: "image/png".to_owned(),
                        bytes: one_pixel_png(),
                    },
                )])),
            )
            .unwrap();
        let generation = ScriptGeneration::initial();
        let handle = registry
            .start_image_decode(
                &AssetId::parse("app/pixel").unwrap(),
                AsyncScope::App,
                generation,
                ScriptCallback::try_from_fn_ptr(FnPtr::new("loaded").unwrap(), generation).unwrap(),
                ScriptCallback::try_from_fn_ptr(FnPtr::new("failed").unwrap(), generation).unwrap(),
            )
            .unwrap();
        assert!(registry.cancel_image_decode(handle).unwrap());
        let effect_scope = AsyncScope::Effect {
            component: crate::ComponentInstancePath::root("App", "root"),
            key: "image".to_owned(),
            activation: 1,
        };
        registry
            .start_image_decode(
                &AssetId::parse("app/pixel").unwrap(),
                effect_scope.clone(),
                generation,
                ScriptCallback::try_from_fn_ptr(FnPtr::new("loaded").unwrap(), generation).unwrap(),
                ScriptCallback::try_from_fn_ptr(FnPtr::new("failed").unwrap(), generation).unwrap(),
            )
            .unwrap();
        registry.cancel_scope(&effect_scope).unwrap();
        registry
            .retain_decode_generation(generation.next())
            .unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert!(
            registry
                .drain_image_decodes(generation.next())
                .unwrap()
                .is_empty()
        );
        assert_eq!(registry.pending_decode_count(), 0);
    }

    #[test]
    fn transaction_rollback_restores_a_cancelled_decode_registration() {
        let registry = AssetRegistry::new();
        registry
            .register(
                "app",
                InMemoryAssetProvider::new(BTreeMap::from([(
                    "pixel".to_owned(),
                    AssetData {
                        mime_type: "image/png".to_owned(),
                        bytes: one_pixel_png(),
                    },
                )])),
            )
            .unwrap();
        let generation = ScriptGeneration::initial();
        let handle = registry
            .start_image_decode(
                &AssetId::parse("app/pixel").unwrap(),
                AsyncScope::App,
                generation,
                ScriptCallback::try_from_fn_ptr(FnPtr::new("loaded").unwrap(), generation).unwrap(),
                ScriptCallback::try_from_fn_ptr(FnPtr::new("failed").unwrap(), generation).unwrap(),
            )
            .unwrap();
        let snapshot = registry.begin_transaction().unwrap();
        assert!(registry.cancel_image_decode(handle).unwrap());
        assert_eq!(registry.pending_decode_count(), 0);
        registry.restore_decode_snapshot(snapshot).unwrap();
        assert_eq!(registry.pending_decode_count(), 1);
    }
}
