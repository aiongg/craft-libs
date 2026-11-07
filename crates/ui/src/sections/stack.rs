//! `SectionStack`: sections in a column that the user reorders by dragging a header or with Alt+Up and Alt+Down. It
//! reuses the drag session, the drop mark and the list move event; the app applies and undoes the moves as it does a
//! list's `Reorder`.

use super::section::SectionOutput;
use crate::contract::reorder::{self, Gap};
use crate::contract::{
    Common, Component, Container, ContainerProps, Cx, DragPreview, DropMark, DropOperation, DropTarget, ListDrag, NoValue, Outcome, Pointer as _,
    ReorderMove, Role, ShowContainer, Sizing,
};
use crate::runtime::access::SetPosition;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Axis, EditEvent, Metrics, Mixed, Sense};
use egui::{Pos2, Rect, Response, Ui};
use std::sync::{Arc, Mutex, PoisonError};

#[cfg(feature = "testing")]
mod stories;

/// What a section of a stack is told about its place: `Section::in_stack` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct StackSlot {
    position: SetPosition,
    salt: u64,
}

impl StackSlot {
    /// The section's place in the stack, for the "n of m" it reports.
    pub fn position(self) -> SetPosition {
        self.position
    }

    /// The id salt that makes the section's id follow its key, so its state stays with it when it moves.
    pub fn salt(self) -> u64 {
        self.salt
    }
}

/// The style of a section stack: no surface or padding; the sections carry the look.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SectionStackStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a stack reports: its response and the moves of this pass, one per drop or Alt+arrow.
#[derive(Debug)]
#[non_exhaustive]
pub struct SectionStackOutput<K> {
    /// The stack's response.
    pub response: Response,
    /// The moves the user made, to apply to the app's order as a list's `CollectionEvent::Reorder` is.
    pub moves: Vec<EditEvent<ReorderMove<K>>>,
}

impl<K> Outcome for SectionStackOutput<K> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// One shown section: where it is and what its header did.
struct Seen<K> {
    key: K,
    rect: Rect,
    grabbed: bool,
    nudged: Option<bool>,
}

/// The closure that shows the section of a key.
type Each<'a, K> = Box<dyn FnMut(&mut Ui, &K, StackSlot) -> SectionOutput + 'a>;

/// Sections in the order of `order`, each shown by the caller's closure as a `Section::in_stack`. Dragging a header
/// or pressing Alt+Up or Alt+Down in a section moves it among the others; the stack reports the move and the app
/// applies it. A section is moved, never copied.
#[derive(crate::contract::Component)]
#[component(container, probe(SectionStack<'static, u8>))]
pub struct SectionStack<'a, K: Clone + PartialEq + std::hash::Hash + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    order: &'a [K],
    each: Option<Each<'a, K>>,
    seen: Arc<Mutex<Vec<Seen<K>>>>,
    moves: Vec<EditEvent<ReorderMove<K>>>,
}

impl<'a, K: Clone + PartialEq + std::hash::Hash + Send + Sync + 'static> SectionStack<'a, K> {
    /// A stack of the sections for `order`; `each` shows the section of a key.
    pub fn new(order: &'a [K], each: impl FnMut(&mut Ui, &K, StackSlot) -> SectionOutput + 'a) -> Self {
        SectionStack {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::FillWidth),
            order,
            each: Some(Box::new(each)),
            seen: Arc::new(Mutex::new(Vec::new())),
            moves: Vec::new(),
        }
    }

    /// Shows every section in order and returns the moves the user made.
    pub fn show(mut self, ui: &mut Ui) -> SectionStackOutput<K> {
        let (order, seen) = (self.order, Arc::clone(&self.seen));
        let mut each = self.each.take();
        let shown = ShowContainer::show(self, ui, move |ui, _| {
            // `each` is taken out of the stack only here, once, so it is present for the one pass the body runs.
            let Some(each) = each.as_mut() else { return };
            let mut rows = Vec::with_capacity(order.len());
            for (index, key) in order.iter().enumerate() {
                let slot = StackSlot { position: SetPosition::new(index).of(order.len()), salt: slot_hash(key) };
                let out = each(ui, key, slot);
                rows.push(Seen { key: key.clone(), rect: out.response.rect, grabbed: out.grabbed, nudged: out.nudged });
            }
            *seen.lock().unwrap_or_else(PoisonError::into_inner) = rows;
        });
        shown.output
    }

    /// Starts a drag from a grabbed header, takes a drop over the sections and turns a nudge into a move.
    fn drive(&mut self, seen: &[Seen<K>], cx: &mut Cx<'_, Self>) {
        let id = cx.core.id;
        if let Some(grabbed) = seen.iter().find(|section| section.grabbed) {
            let key = grabbed.key.clone();
            cx.start_drag(ListDrag::new(id, vec![key.clone()], key), DragPreview::None);
        }
        if let Some((payload, at)) = cx.dragging::<ListDrag<K>>()
            && payload.list == id
        {
            let rects: Vec<Rect> = seen.iter().map(|section| section.rect).collect();
            let target = GapDrop(reorder::gap_at(&rects, Axis::Y, at));
            if let Some(gap) = cx.drop_target::<ListDrag<K>, _>(&target)
                && let Some(from) = seen.iter().position(|section| section.key == payload.grabbed)
            {
                self.push_move(cx, &payload.grabbed, from, reorder::move_to(from, gap.insert));
            }
        }
        if let Some((from, forward)) = seen.iter().enumerate().find_map(|(at, section)| section.nudged.map(|forward| (at, forward))) {
            self.push_move(cx, &seen[from].key, from, reorder::nudge_to(from, seen.len(), forward));
        }
    }

    fn push_move(&mut self, cx: &Cx<'_, Self>, key: &K, from: usize, to: usize) {
        if to == from {
            return;
        }
        let gesture = crate::runtime::edit::next_gesture_id(&cx.core.ctx);
        let before = ReorderMove { key: key.clone(), keys: vec![key.clone()], to: from, copy: false };
        let after = ReorderMove { key: key.clone(), keys: vec![key.clone()], to, copy: false };
        self.moves.push(EditEvent::set(gesture, Mixed::Same(before), after, cx.core.label_arc(), None));
    }
}

/// Where a drop on the stack lands: the gap nearest the pointer.
struct GapDrop(Option<Gap>);

impl<K> DropTarget<ListDrag<K>> for GapDrop {
    type Drop = Gap;

    fn accept(&self, _: &ListDrag<K>, _: Pos2, op: DropOperation) -> Option<Gap> {
        if matches!(op, DropOperation::Copy) { None } else { self.0 }
    }

    fn mark(&self, drop: &Gap) -> DropMark {
        drop.mark
    }
}

impl<K: Clone + PartialEq + std::hash::Hash + Send + Sync + 'static> Component for SectionStack<'_, K> {
    const ROLE: Role = Role::None;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = SectionStackStyle;
    type Output = SectionStackOutput<K>;

    fn style(_: &StyleCx<'_>, _: SectionStackStyleKey<K>) -> SectionStackStyle {
        SectionStackStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(mut self, mut cx: Cx<'_, Self>) -> SectionStackOutput<K> {
        let seen = std::mem::take(&mut *self.seen.lock().unwrap_or_else(PoisonError::into_inner));
        self.drive(&seen, &mut cx);
        SectionStackOutput { response: cx.response.clone(), moves: self.moves }
    }
}

impl<K: Clone + PartialEq + std::hash::Hash + Send + Sync + 'static> Container for SectionStack<'_, K> {}

fn slot_hash<K: std::hash::Hash>(key: &K) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
