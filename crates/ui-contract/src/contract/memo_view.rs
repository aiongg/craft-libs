//! `MemoView`: a view the framework runs only when one of its inputs changed, and otherwise keeps as its last run left
//! it.

use super::Ui;
use super::memo::{MemoKey, MemoProps};
use super::rt::key::{fold, type_key};
use super::rt::scope::{ScopeShow, show_scope};
use super::{Lazy, Shown};
use crate::runtime::env::Env;
use crate::runtime::shift::Shift;
use std::fmt;
use std::hash::Hash;

/// A memoized view: its content is a `fn`, which captures nothing, and its props ([`MemoProps`]) are its only input
/// besides the `Ui`, so the framework sees every input it reads. It is a scope node of its own. While its props key
/// equals the key of its last run, its place (the room it is given) is unchanged, no read its last run made changed
/// and no node under it is marked, the view keeps its subtree as it is (paint, hit shapes, access nodes, focus stops)
/// and returns what its last run returned, at the cost of a key check. Input reaches a kept node through the node
/// itself and marks the path to it, so a view whose nodes take input runs.
///
/// The key is the props type, the `fn` and every input the props feed, and what the scope hands down beyond its style
/// inputs (`Scoped::scope_key`). A run that writes through its props (a `&mut Tracked<T>` model, a
/// [`Bind`](super::Bind)) moves its key. A `Tracked` read through `Deref` inside a build is a tracked read: a write of
/// that `Tracked` later in the frame (`DerefMut`, `edit`, a `bind_tracked!` field's edit) runs every node that read it
/// before the write again in a later build round of the same frame, this view included when its own run read it (an
/// app that applies its panel's events in the view's body shows them in that frame). A write that nothing in the
/// frame read before it reruns nothing then: a field's own bound write shows in the field, and the view runs once more
/// when a pass next reaches it, and is kept from then on.
///
/// Its content is a layout room of its own: the flow of the body around it does not reach in. To the body around it
/// the view is one unit, sized by what its content took.
///
/// What `show` returns is a [`Shown`]: `fresh` is true when the `fn` ran in this build. State values (a size, a rect,
/// which part took the room) are read from `value` either way; a kept view at another position returns its value
/// moved (`Shift`). Events (picks, closes, a drag that started) are read only through [`Shown::value_if_fresh`], or
/// leave through an [`Outbox`] in the props: a kept view never repeats an event.
///
/// Inside a scroll body the view is also a `Lazy`: out of view it keeps its extent, its nodes go, and `show` returns
/// `None`.
///
/// `show` is a `fn`: a closure that captures does not build.
///
/// ```compile_fail,E0308
/// # extern crate craft_ui_contract as craft_ui;
/// use craft_ui::contract::{MemoView, Ui};
/// fn view(ui: &mut Ui, count: u32) {
///     let _ = MemoView::new("count", (), move |_: &mut (), _: &mut Ui| count).show(ui);
/// }
/// ```
#[must_use]
pub struct MemoView<P, R> {
    lazy: Lazy,
    props: P,
    show: fn(&mut P, &mut Ui) -> R,
}

impl<P: MemoProps, R: Clone + PartialEq + Shift + Send + Sync + 'static> MemoView<P, R> {
    /// The view `id` showing `props` through `show`. The id is unique as a `Lazy` key is: among the memoized views and
    /// `Lazy` children of its parent `Ui`.
    pub fn new(id: impl Hash, props: P, show: fn(&mut P, &mut Ui) -> R) -> MemoView<P, R> {
        MemoView { lazy: Lazy::new(id, 0.0), props, show }
    }

    /// Runs the view in its scope, placed in flow in `ui`, or keeps its last run. `None` only when it is out of view
    /// in a scroll body. In a measuring run a kept view answers with its kept size and value.
    pub fn show(self, ui: &mut Ui) -> Option<Shown<R>> {
        let MemoView { lazy, mut props, show } = self;
        let key = Self::key(&Env::of(ui), &props, show);
        let scope =
            ScopeShow { id: lazy.id_source(ui), type_key: type_key::<MemoView<P, R>>(), memo: Some(key), estimate: 0.0, reveal: false, lazy: true };
        show_scope(ui, scope, Some(R::shifted), &mut |ui| {
            let before = props_key(&props);
            let value = show(&mut props, ui);
            // The run wrote through its props: its key moved, so the view runs again in a later build round of this
            // frame, from the written value. A measuring run is no real run: it edits nothing it keeps.
            if ui.flow.measuring.is_none() && props_key(&props) != before {
                ui.cx.rebuild();
            }
            value
        })
    }

    /// The key of `props` shown through `show` in the scope `env`: the props type, the `fn`'s address, the props' inputs
    /// and what the scope hands down beyond its style inputs (`Scoped::scope_key`). Two `fn`s the compiler merged into
    /// one address differ in their props type or show the same.
    fn key(env: &Env, props: &P, show: fn(&mut P, &mut Ui) -> R) -> u64 {
        let mut key = MemoKey::new();
        key.feed_hash(&P::identity());
        key.write(&(show as usize).to_le_bytes());
        key.write(&env.scope_key().to_le_bytes());
        props.key(&mut key);
        fold(key.finish())
    }
}

/// The part of a view's key its props feed.
fn props_key<P: MemoProps>(props: &P) -> u64 {
    let mut key = MemoKey::new();
    props.key(&mut key);
    fold(key.finish())
}

/// The events a memoized view's run sends to the code around it, in the pass it runs: what its components reported (a
/// pick, a close, a drag that started). A run sends; a kept view sends nothing. That is complete, because input reaches
/// only a view whose nodes it reaches, and input queued on a node marks the path to it, so the view runs. The view cannot read what it sent, so an outbox is no input and feeds nothing into the key.
///
/// ```
/// # extern crate craft_ui_contract as craft_ui;
/// use craft_ui::contract::{MemoProps, MemoView, Outbox};
/// #[derive(MemoProps)]
/// struct Row<'a> {
///     title: &'a str,
///     clicked: Outbox<'a, ()>,
/// }
/// fn row(props: &mut Row<'_>, ui: &mut craft_ui::contract::Ui) {
///     if craft_ui::contract::Lazy::new(props.title, 0.0).show(ui, |_| true).unwrap_or(false) {
///         props.clicked.send(());
///     }
/// }
/// fn view(ui: &mut craft_ui::contract::Ui) -> bool {
///     let mut clicked = Vec::new();
///     let _ = MemoView::new("row", Row { title: "Open", clicked: Outbox::new(&mut clicked) }, row).show(ui);
///     !clicked.is_empty()
/// }
/// ```
pub struct Outbox<'a, E> {
    sent: &'a mut Vec<E>,
}

impl<'a, E> Outbox<'a, E> {
    /// An outbox whose events go into `sent`, after what it holds.
    pub fn new(sent: &'a mut Vec<E>) -> Outbox<'a, E> {
        Outbox { sent }
    }

    /// Sends `event`.
    pub fn send(&mut self, event: E) {
        self.sent.push(event);
    }
}

impl<E> fmt::Debug for Outbox<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Outbox").field("sent", &self.sent.len()).finish()
    }
}

/// An outbox feeds nothing: the view only writes it.
impl<E> MemoProps for Outbox<'_, E> {
    fn key(&self, _: &mut MemoKey) {}
}
