//! `Arrangement` and `arrange`: a custom placement of keyed children measured with `measure_in` and shown once.

use crate::runtime::egui::alloc;
use crate::runtime::env::Env;
use crate::runtime::memory::sub;
use craft_behavior::{Arrangement, Flow, Proposal};
use egui::{Id, Rect, Response, Ui, Vec2};
use std::fmt::Debug;
use std::hash::Hash;

/// What `arrange` needs of a child: its size under an id, and a show at a rect under the same id.
// F2-MERGE: F3 (the contract's `Show` with `measure_in` and `show_at`; `Placed::new` then takes `C: Show`)
pub trait Show {
    /// The size of the child under `id`, measured as it will be shown.
    fn measure_in(&mut self, ui: &mut Ui, id: Id, proposal: Proposal) -> Vec2;

    /// Shows the child at `rect` under `id`.
    fn show_at(&mut self, ui: &mut Ui, id: Id, rect: Rect) -> Response;
}

/// A keyed child for `arrange`; shown once, under the id derived from the key.
pub struct Placed<'c> {
    id: Box<dyn Fn(Id) -> Id + 'c>,
    child: Box<dyn Show + 'c>,
    response: Option<Response>,
}

impl<'c> Placed<'c> {
    /// A child shown under the id `key` derives from the arranging `Ui`.
    pub fn new<C: Show + 'c>(key: impl Hash + Debug + 'c, child: C) -> Self {
        Placed { id: Box::new(move |base| sub(base, &key)), child: Box::new(child), response: None }
    }

    /// The child's response after `arrange` showed it.
    pub fn response(&self) -> Option<&Response> {
        self.response.as_ref()
    }
}

/// Places `children` by `layout` in the rest of `ui` and shows each once. The placement is logical; its output
/// is mirrored through the env's direction when the layout mirrors, so no arrangement reads the direction.
/// Returns the rect the placement took.
pub fn arrange(ui: &mut Ui, layout: &dyn Arrangement, children: &mut [Placed<'_>]) -> Rect {
    let rtl = Env::of(ui).direction.is_rtl();
    let room = ui.available_rect_before_wrap();
    let base = ui.scope_id();
    let ids: Vec<Id> = children.iter().map(|placed| (placed.id)(base)).collect();
    let sizes: Vec<Vec2> = children.iter_mut().zip(&ids).map(|(placed, id)| placed.child.measure_in(ui, *id, Proposal::Ideal)).collect();
    let mut rects = Vec::with_capacity(sizes.len());
    let taken = layout.place(room, &sizes, &mut rects);
    let flow = if layout.mirrors() { Flow::new(room, rtl) } else { Flow::new(room, false) };
    for ((placed, id), rect) in children.iter_mut().zip(&ids).zip(&rects) {
        placed.response = Some(placed.child.show_at(ui, *id, flow.to_physical(*rect)));
    }
    let whole = flow.to_physical(Rect::from_min_size(room.min, taken));
    alloc::advance_past(ui, whole);
    whole
}
