//! `TextureCache`, the one texture path.

use crate::runtime::egui::{memory, textures};
use egui::{ColorImage, Context, TextureHandle, TextureId};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/// The textures of one context by key hash. A handle that is dropped frees its texture, so the cache owns them.
#[derive(Default)]
struct Loaded {
    handles: HashMap<u64, TextureHandle>,
}

/// The texture cache of a context: components keep a cache key and ask for the texture when they paint.
pub struct TextureCache;

impl TextureCache {
    /// The texture for `key`: loads `image()` on the first request and returns the same texture afterwards. A key
    /// names everything the image depends on (icon, pixel size, stroke, mirroring), so a changed input is a
    /// different key.
    pub fn texture(ctx: &Context, key: impl Hash, image: impl FnOnce() -> ColorImage) -> TextureId {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(id) = memory::with(ctx, |loaded: &mut Loaded| loaded.handles.get(&hash).map(TextureHandle::id)) {
            return id;
        }
        // The image is built and uploaded outside the service lock: neither calls back into the cache.
        let handle = textures::load(ctx, format!("craft_ui/{hash:016x}"), image());
        memory::with(ctx, |loaded: &mut Loaded| loaded.handles.entry(hash).or_insert(handle).id())
    }

    /// The number of textures the context holds.
    pub fn len(ctx: &Context) -> usize {
        memory::with(ctx, |loaded: &mut Loaded| loaded.handles.len())
    }

    /// Whether the context holds no texture.
    pub fn is_empty(ctx: &Context) -> bool {
        Self::len(ctx) == 0
    }

    /// Drops every texture of the context; the next request loads again.
    pub fn clear(ctx: &Context) {
        memory::with(ctx, |loaded: &mut Loaded| loaded.handles.clear());
    }
}
