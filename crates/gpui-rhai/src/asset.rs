use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use gpui::{Image, ImageFormat, ImageSource};
use rhai::{CustomType, TypeBuilder};
use thiserror::Error;

use crate::{
    AsyncDelivery, AsyncScope, ComponentInstancePath, OpaqueHandle, Rgba8, ScriptCallback,
    ScriptGeneration, UiValue,
};

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
    result: Result<(ImageFormat, AssetData), String>,
}

struct AssetRegistryInner {
    providers: BTreeMap<String, Box<dyn AssetProvider>>,
    by_asset: BTreeMap<AssetId, ImageHandle>,
    images: BTreeMap<u64, Arc<Image>>,
    image_data: BTreeMap<u64, AssetData>,
    tinted_images: BTreeMap<(u64, u32), Arc<Image>>,
    next_id: u64,
    next_decode_id: u64,
    pending_decodes: BTreeMap<u64, PendingImageDecode>,
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
            tinted_images: BTreeMap::new(),
            next_id: 0,
            next_decode_id: 1,
            pending_decodes: BTreeMap::new(),
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
        if inner
            .providers
            .insert(namespace.clone(), Box::new(provider))
            .is_some()
        {
            return Err(AssetError::DuplicateNamespace(namespace));
        }
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
                    Ok((
                        handle.opaque.id(),
                        data.clone(),
                        Arc::new(Image::from_bytes(format, data.bytes)),
                    ))
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
            inner
                .tinted_images
                .retain(|(image_id, _), _| image_id != id);
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
        let image = Arc::new(Image::from_bytes(format, data.bytes.clone()));
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
        pending.canceled.store(true, Ordering::Release);
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
        inner.pending_decodes.retain(|_, pending| {
            let remove = match &pending.scope {
                AsyncScope::Window(id) => id == window,
                AsyncScope::Component(_) | AsyncScope::Effect { .. } => {
                    pending.scope.is_within_component(component)
                }
                AsyncScope::App => false,
            };
            if remove {
                pending.canceled.store(true, Ordering::Release);
            }
            !remove
        });
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
        inner.pending_decodes.retain(|_, pending| {
            let remove = pending.scope.is_within_component(component);
            if remove {
                pending.canceled.store(true, Ordering::Release);
            }
            !remove
        });
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
        inner.pending_decodes.retain(|_, pending| {
            let remove = &pending.scope == scope;
            if remove {
                pending.canceled.store(true, Ordering::Release);
            }
            !remove
        });
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
                Ok((format, data)) => {
                    let handle = install_decoded_image(&mut inner, pending.asset, format, data);
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

    pub(crate) fn pending_decode_ids(&self) -> Result<std::collections::BTreeSet<u64>, AssetError> {
        Ok(self
            .inner
            .try_borrow()
            .map_err(|_| AssetError::Borrowed)?
            .pending_decodes
            .keys()
            .copied()
            .collect())
    }

    pub(crate) fn retain_decode_ids(
        &self,
        retained: &std::collections::BTreeSet<u64>,
    ) -> Result<(), AssetError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        inner.pending_decodes.retain(|id, pending| {
            let keep = retained.contains(id);
            if !keep {
                pending.canceled.store(true, Ordering::Release);
            }
            keep
        });
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
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| AssetError::Borrowed)?;
        let Some(data) = inner.image_data.get(&handle.id()).cloned() else {
            return Err(AssetError::UnknownHandle(handle.id()));
        };
        let image = if data.mime_type == "image/svg+xml"
            && let Some(color) = color
        {
            let key = (handle.id(), color.as_rgba_hex());
            if let Some(image) = inner.tinted_images.get(&key) {
                image.clone()
            } else {
                let bytes = tint_svg(&data.bytes, color)?;
                let image = Arc::new(Image::from_bytes(ImageFormat::Svg, bytes));
                inner.tinted_images.insert(key, image.clone());
                image
            }
        } else {
            inner
                .images
                .get(&handle.id())
                .cloned()
                .ok_or(AssetError::UnknownHandle(handle.id()))?
        };
        Ok(ImageSource::Image(image))
    }
}

fn install_decoded_image(
    inner: &mut AssetRegistryInner,
    asset: AssetId,
    format: ImageFormat,
    data: AssetData,
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
    inner.images.insert(
        image_id,
        Arc::new(Image::from_bytes(format, data.bytes.clone())),
    );
    inner.image_data.insert(image_id, data);
    inner.by_asset.insert(asset, handle.clone());
    handle
}

fn decode_asset_data(data: AssetData) -> Result<(ImageFormat, AssetData), String> {
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
    Ok((format, data))
}

fn raster_image_format(format: ImageFormat) -> Option<image::ImageFormat> {
    match format {
        ImageFormat::Png => Some(image::ImageFormat::Png),
        ImageFormat::Jpeg => Some(image::ImageFormat::Jpeg),
        ImageFormat::Webp => Some(image::ImageFormat::WebP),
        ImageFormat::Gif => Some(image::ImageFormat::Gif),
        ImageFormat::Bmp => Some(image::ImageFormat::Bmp),
        ImageFormat::Tiff => Some(image::ImageFormat::Tiff),
        ImageFormat::Svg => None,
    }
}

fn tint_svg(bytes: &[u8], color: Rgba8) -> Result<Vec<u8>, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|_| AssetError::InvalidSvg)?;
    if !source.contains("<svg") {
        return Err(AssetError::InvalidSvg);
    }
    let rgb = color.as_rgba_hex() >> 8;
    Ok(source
        .replace("currentColor", &format!("#{rgb:06x}"))
        .replace("currentcolor", &format!("#{rgb:06x}"))
        .into_bytes())
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
                        bytes: b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec(),
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
    fn namespace_refresh_preserves_handle_and_replaces_cached_bytes() {
        let data = Rc::new(RefCell::new(AssetData {
            mime_type: "image/svg+xml".to_owned(),
            bytes: b"<svg data-version=\"one\"/>".to_vec(),
        }));
        let registry = AssetRegistry::new();
        registry
            .register("app", MutableProvider(Rc::clone(&data)))
            .unwrap();
        let id = AssetId::parse("app/icon").unwrap();
        let handle = registry.load_image(&id).unwrap();
        data.borrow_mut().bytes = b"<svg data-version=\"two\"/>".to_vec();
        assert_eq!(registry.refresh_namespace("app").unwrap(), 1);
        assert_eq!(registry.load_image(&id).unwrap(), handle);
        assert_eq!(
            registry.inner.borrow().image_data[&handle.opaque.id()].bytes,
            b"<svg data-version=\"two\"/>"
        );
    }

    #[test]
    fn svg_current_color_is_semantically_tinted_and_cached() {
        let registry = AssetRegistry::new();
        registry
            .register(
                "core",
                InMemoryAssetProvider::new(BTreeMap::from([(
                    "check".to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: b"<svg stroke=\"currentColor\"/>".to_vec(),
                    },
                )])),
            )
            .unwrap();
        let handle = registry
            .load_image(&AssetId::parse("core/check").unwrap())
            .unwrap();
        let color = Rgba8::from_rgb_hex(0x0012_34ab);
        let first = registry
            .image_source_tinted(handle.opaque(), Some(color))
            .unwrap();
        let second = registry
            .image_source_tinted(handle.opaque(), Some(color))
            .unwrap();
        let (ImageSource::Image(first), ImageSource::Image(second)) = (first, second) else {
            panic!("asset registry must return in-memory images");
        };
        assert!(Arc::ptr_eq(&first, &second));
        assert!(String::from_utf8_lossy(&first.bytes).contains("#1234ab"));
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
}
