//! Requests to a collection's item step from outside it, taken at the collection's next pass.

use crate::runtime::egui::memory as store;
use egui::{Context, Id};

/// The rename requests not yet taken: the collection and the key to rename.
struct RenameRequests<K>(Vec<(Id, K)>);

impl<K> Default for RenameRequests<K> {
    fn default() -> Self {
        RenameRequests(Vec::new())
    }
}

/// Asks the collection `collection` (the id it is built with, `Props::id`) to start renaming the item `key`. At its
/// next item step the lead moves to the item, the item is revealed and its rename field opens, as for the rename
/// key. A request for a key the source lacks, or for a collection that cannot rename the item, is dropped. A second
/// request for the same collection replaces the first.
pub fn request_rename<K: Clone + Send + Sync + 'static>(ctx: &Context, collection: Id, key: &K) {
    store::Service::<RenameRequests<K>>::run(ctx, |requests| {
        requests.0.retain(|(id, _)| *id != collection);
        requests.0.push((collection, key.clone()));
    });
    crate::runtime::egui::repaint::request_of(ctx, crate::runtime::egui::pass::current_viewport(ctx));
}

/// The rename request addressed to `collection`, once.
pub(super) fn take_rename<K: Send + Sync + 'static>(ctx: &Context, collection: Id) -> Option<K> {
    store::Service::<RenameRequests<K>>::run(ctx, |requests| {
        let at = requests.0.iter().position(|(id, _)| *id == collection)?;
        Some(requests.0.remove(at).1)
    })
}

/// A move of the lead or of the view that a component asks of its collection's item step.
#[derive(Clone, Copy)]
pub(crate) enum Move {
    /// The lead goes to the position, selects it and scrolls it into view, as a plain arrow key does.
    Lead(usize),
    /// The view scrolls until the span of the content, `top` to `bottom` from the rows' top, shows.
    Reveal { top: f32, bottom: f32 },
}

#[derive(Default)]
struct Moves(Vec<(Id, Move)>);

/// Asks the collection `collection` to apply `mv` at its next item step.
pub(crate) fn request_move(ctx: &Context, collection: Id, mv: Move) {
    store::Service::<Moves>::run(ctx, |moves| moves.0.push((collection, mv)));
    crate::runtime::egui::repaint::request_of(ctx, crate::runtime::egui::pass::current_viewport(ctx));
}

/// The moves addressed to `collection`, in the order they were asked.
pub(super) fn take_moves(ctx: &Context, collection: Id) -> Vec<Move> {
    store::Service::<Moves>::run(ctx, |moves| {
        let (mine, rest) = std::mem::take(&mut moves.0).into_iter().partition(|(id, _)| *id == collection);
        moves.0 = rest;
        mine.into_iter().map(|(_, mv)| mv).collect()
    })
}

/// The lead's position as the item step left it at the end of its last pass.
#[derive(Clone, Copy)]
struct LeadFact(Option<usize>);

fn lead_slot(collection: Id) -> Id {
    crate::runtime::memory::sub(collection, "lead_position")
}

pub(super) fn record_lead(ctx: &Context, collection: Id, at: Option<usize>) {
    let slot = lead_slot(collection);
    store::put(ctx, slot, LeadFact(at));
    crate::runtime::memory::adopt::<LeadFact>(ctx, collection, slot);
}

/// The position of the lead of `collection` after its last item step, `None` while it has none.
pub(crate) fn lead_position(ctx: &Context, collection: Id) -> Option<usize> {
    store::peek_clone::<LeadFact>(ctx, lead_slot(collection)).and_then(|fact| fact.0)
}
