//! The component key and the child keys of reconcile (docs/contract-rt.md sections 3.2 and 6).

use crate::contract::{IdSource, MemoKey};
use craft_ui_rt::hash_key;
use egui::Id;

pub use crate::contract::ComponentKey;

/// The key of `component`, or `None` for an unkeyed type.
pub(crate) fn component_key<C: ComponentKey>(component: &C) -> Option<u128> {
    if C::UNKEYED {
        return None;
    }
    let mut key = MemoKey::new();
    component.key(&mut key);
    Some(key.finish())
}

/// A type's key among its siblings, lifetimes erased (`IconButton<'a>` and `IconButton<'b>` are
/// one type).
pub(crate) fn type_key<C: ?Sized>() -> u64 {
    hash_key(&typeid::of::<C>())
}

/// A 128-bit key as a ui-rt props key.
pub(crate) fn fold(key: u128) -> u64 {
    (key as u64) ^ ((key >> 64) as u64)
}

/// Counts the siblings of each component type in one build, for [`IdSource::Auto`].
#[derive(Default)]
pub(crate) struct Ordinals(smallvec::SmallVec<[(u64, u32); 8]>);

impl Ordinals {
    /// The craft `Id` of a child shown under `scope` and its ui-rt child key: `Auto` counts the
    /// siblings of its type, as step 3 does today; `Salt` joins the scope; `Id` is the caller's.
    pub(crate) fn child(&mut self, scope: Id, type_key: u64, source: IdSource) -> (Id, u64) {
        let id = match source {
            IdSource::Auto => {
                let n = match self.0.iter_mut().find(|(t, _)| *t == type_key) {
                    Some((_, n)) => {
                        *n += 1;
                        *n - 1
                    }
                    None => {
                        self.0.push((type_key, 1));
                        0
                    }
                };
                scope.with((type_key, n))
            }
            IdSource::Salt(salt) => scope.with_salt(salt),
            IdSource::Id(id) => id,
        };
        (id, id.value())
    }
}
