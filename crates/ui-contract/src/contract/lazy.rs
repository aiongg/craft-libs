//! `Lazy`: a child of a scroll body that runs only while it is in view. Out of view its content does not run and its
//! nodes go; it keeps the extent it had when it last ran, or takes the app's estimate before its first run. In view it
//! is a scope whose closure runs whenever its owner's build reaches it; its children are kept nodes that cost a key
//! check. A wheel step inside the band its scroll body built moves kept content and runs nothing; one that leaves the
//! band builds the body again, and only the `Lazy` children entering the band build their content.

use super::Ui;
use super::rt::key::type_key;
use super::rt::scope::{ScopeShow, show_scope};
use crate::contract::IdSource;
use std::hash::{Hash, Hasher};

/// A child of a scroll body that runs only while it is in view (module docs).
#[must_use]
pub struct Lazy {
    key: u64,
    estimate: f32,
    reveal: bool,
}

impl Lazy {
    /// The child keyed `key` among the `Lazy` children and memoized views of its parent `Ui`. `estimate` is how long
    /// it is down the scroll axis before it first ran.
    pub fn new(key: impl Hash, estimate: f32) -> Lazy {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        Lazy { key: hasher.finish(), estimate, reveal: false }
    }

    /// When `reveal` holds, the scroll body glides until the child starts at the view's start, as far as the content
    /// lets it. A wheel or drag ends the glide.
    pub fn reveal(mut self, reveal: bool) -> Lazy {
        self.reveal = reveal;
        self
    }

    /// The child's craft id source under `ui`'s node: the key the scope is reconciled by.
    pub(crate) fn id_source(&self, ui: &Ui) -> IdSource {
        IdSource::Id(ui.flow.id.with(("lazy", self.key)))
    }

    /// Runs `add` in the child's flow, or takes its kept extent out of view; `None` when it did not run. A lazy body
    /// may run in two build rounds of one frame (a measuring run, then the real one).
    pub fn show<R>(self, ui: &mut Ui, mut add: impl FnMut(&mut Ui) -> R) -> Option<R> {
        let scope =
            ScopeShow { id: self.id_source(ui), type_key: type_key::<Lazy>(), memo: None, estimate: self.estimate, reveal: self.reveal, lazy: true };
        // A lazy scope keeps no value: the value leaves through a slot, never through the kept scope.
        let mut out = None;
        show_scope(ui, scope, None, &mut |ui| {
            out = Some(add(ui));
        })?;
        out
    }
}
