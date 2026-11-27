//! The style cache and variant acceptance.

use super::cx::GroupCache;
use super::cx::StyleCx;
use super::key::{EnvKey, StyleKey};
use super::look::ComponentStyle;
use super::replacements::Frozen;
use craft_theme::{ControlSize, Variant};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

/// The variant a component takes for a requested one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Accepted {
    /// The variant the style is built for.
    pub variant: Variant,
    /// The requested variant was not accepted, so the default stands in.
    pub fell_back: bool,
}

/// A component accepts its `VARIANTS` plus every custom variant built for its style key. Any other variant gives
/// the default, the first of `VARIANTS`; a component with none takes every other request as the default look.
pub fn accepted_variant<K: StyleKey, S: ComponentStyle>(requested: Variant) -> Accepted {
    let custom = matches!(requested, Variant::Custom(custom) if custom.key_type() == TypeId::of::<K>());
    if custom || S::VARIANTS.contains(&requested) {
        return Accepted { variant: requested, fell_back: false };
    }
    match S::VARIANTS.first().copied() {
        Some(default) => Accepted { variant: default, fell_back: true },
        None => Accepted { variant: Variant::SECONDARY, fell_back: false },
    }
}

/// `StyleKey` erased: equality and hashing through `Any`, so one map serves every component.
trait ErasedKey: Send + Sync {
    fn eq_dyn(&self, other: &dyn Any) -> bool;
}

impl<K: StyleKey> ErasedKey for K {
    fn eq_dyn(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<K>().is_some_and(|other| other == self)
    }
}

struct Entry {
    ty: TypeId,
    style_ty: TypeId,
    key: Box<dyn ErasedKey>,
    env: EnvKey,
    variant: Variant,
    size: ControlSize,
    style: Arc<dyn Any + Send + Sync>,
}

fn hash_of<K: StyleKey, S: ComponentStyle>(key: &K, env: EnvKey, variant: Variant, size: ControlSize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    TypeId::of::<K>().hash(&mut hasher);
    TypeId::of::<S>().hash(&mut hasher);
    key.hash(&mut hasher);
    env.hash(&mut hasher);
    variant.hash(&mut hasher);
    size.hash(&mut hasher);
    hasher.finish()
}

/// The cache of one context: buckets by hash, each holding the few entries that share one.
#[derive(Default)]
pub(crate) struct StyleCache {
    buckets: HashMap<u64, Vec<Entry>>,
    fallbacks: HashSet<(TypeId, Variant)>,
}

impl StyleCache {
    fn get<K: StyleKey, S: ComponentStyle>(&self, hash: u64, key: &K, env: EnvKey, variant: Variant, size: ControlSize) -> Option<Arc<S>> {
        let entry = self.buckets.get(&hash)?.iter().find(|e| {
            e.ty == TypeId::of::<K>()
                && e.style_ty == TypeId::of::<S>()
                && e.env == env
                && e.variant == variant
                && e.size == size
                && e.key.eq_dyn(key)
        })?;
        entry.style.clone().downcast::<S>().ok()
    }
}

/// The style of a component: the cached one for `(TypeId::of::<K>(), TypeId::of::<S>(), key, EnvKey, variant, size)`, else the app's
/// replacement or `default` run on a miss. A hit costs one lock, one hash lookup and one `Arc` clone; the style
/// function runs outside the lock and its result is inserted after it returns.
/// One window's style caches (docs/contract-rt.md section 15.7): the resolved styles, the token groups and the app's
/// style replacements. The window root provides it (`BuildCx::provide`, through the window's `Context`); the
/// window's tokens and theme-base revisions it was filled at clear it when either moves.
#[derive(Clone, Default)]
pub struct StyleCaches(Rc<RefCell<Caches>>);

#[derive(Default)]
pub(crate) struct Caches {
    pub(crate) styles: StyleCache,
    pub(crate) groups: GroupCache,
    pub(crate) replacements: Frozen,
    /// The revisions of the window's tokens, theme-base and scale cells the entries were filled at.
    filled_at: Option<(u64, u64, u64)>,
}

impl StyleCaches {
    /// Runs `f` on the caches. `f` must not reach the caches again.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut Caches) -> R) -> R {
        f(&mut self.0.borrow_mut())
    }

    /// Clears every entry when the window's tokens, theme-base or scale revision moved since the caches were filled
    /// at `revisions` (a theme, contrast, density or scale write); the replacements stay.
    pub(crate) fn clear_if_moved(&self, revisions: (u64, u64, u64)) {
        let mut caches = self.0.borrow_mut();
        if caches.filled_at != Some(revisions) {
            if caches.filled_at.is_some() {
                caches.styles = StyleCache::default();
                caches.groups = GroupCache::default();
            }
            caches.filled_at = Some(revisions);
        }
    }

    /// Installs the app's style replacements.
    pub(crate) fn set_replacements(&self, frozen: Frozen) {
        let mut caches = self.0.borrow_mut();
        caches.replacements = frozen;
        caches.styles = StyleCache::default();
    }
}

#[doc(hidden)]
pub fn resolve_style<K: StyleKey, S: ComponentStyle>(
    cx: &StyleCx<'_>,
    key: K,
    caches: &StyleCaches,
    default: for<'a> fn(&StyleCx<'a>, K) -> S,
) -> Arc<S> {
    let accepted = accepted_variant::<K, S>(cx.variant);
    let (env, size) = (cx.key, cx.size);
    let hash = hash_of::<K, S>(&key, env, accepted.variant, size);
    if let Some(hit) = caches.with(|caches| caches.styles.get::<K, S>(hash, &key, env, accepted.variant, size)) {
        return hit;
    }
    let build = caches.with(|caches| caches.replacements.get::<K, S>()).unwrap_or(default);
    // The style function is app code and may resolve other styles: it runs outside the borrow.
    let style = Arc::new(build(&cx.with_variant(accepted.variant), key));
    let entry =
        Entry { ty: TypeId::of::<K>(), style_ty: TypeId::of::<S>(), key: Box::new(key), env, variant: accepted.variant, size, style: style.clone() };
    caches.with(|caches| {
        let cache = &mut caches.styles;
        if accepted.fell_back && cache.fallbacks.insert((TypeId::of::<K>(), cx.variant)) {
            log::warn!("{} does not accept the variant {:?}; the default look stands in", std::any::type_name::<K>(), cx.variant);
        }
        let bucket = cache.buckets.entry(hash).or_default();
        match bucket.iter_mut().find(|e| {
            e.ty == entry.ty && e.style_ty == entry.style_ty && e.env == env && e.variant == entry.variant && e.size == size && e.key.eq_dyn(&key)
        }) {
            Some(slot) => *slot = entry,
            None => bucket.push(entry),
        }
    });
    style
}
