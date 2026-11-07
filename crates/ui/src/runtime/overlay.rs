//! The layer stack (15.2): the record of the egui layers shown, their order and the open-modal fact.
//!
//! The stack owns no open state. The overlay host records an entry for each layer it shows; entries recorded in a
//! pass are read in the next one, and an entry not recorded for a frame is gone.

use crate::runtime::commands;
use crate::runtime::egui::areas::{self, Layer, Tier};
use crate::runtime::egui::memory::Service;
use crate::runtime::egui::pass;
use crate::runtime::frame::{PassStep, PerViewport, Stamped};
use craft_behavior::OwnerClass;
use craft_behavior::overlay::Side;
use egui::{Context, Id, Rect, ViewportId};
use smallvec::SmallVec;
use std::collections::HashMap;

/// A modal's backdrop.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Backdrop {
    /// No scrim; the layer still blocks input below it.
    None,
    /// One scrim over the viewport.
    Scrim,
    /// A scrim with a hole around a rect (the tour).
    ScrimWithHole(Rect),
}

/// An overlay's place in the order. `Rank` is the only place a layer's egui order is chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Rank {
    /// A floating window or panel.
    Window,
    /// A toast.
    Toast,
    /// A popup, menu or dropdown.
    Popup,
    /// A modal layer; it blocks input below it.
    #[non_exhaustive]
    Modal {
        /// The scrim behind it.
        backdrop: Backdrop,
    },
    /// The chrome of a modal that stays above its popups.
    ModalChrome,
    /// A tooltip.
    Tooltip,
}

impl Rank {
    /// A modal rank with `backdrop`.
    pub const fn modal(backdrop: Backdrop) -> Rank {
        Rank::Modal { backdrop }
    }

    /// Families of one egui order sort by this class: toast below popup below modal below modal chrome.
    const fn class(self) -> u8 {
        match self {
            Rank::Window => 0,
            Rank::Toast => 1,
            Rank::Popup => 2,
            Rank::Modal { .. } => 3,
            Rank::ModalChrome => 4,
            Rank::Tooltip => 5,
        }
    }

    pub(crate) const fn tier(self) -> Tier {
        match self {
            Rank::Window => Tier::Window,
            Rank::Tooltip => Tier::Tooltip,
            _ => Tier::Foreground,
        }
    }

    /// Whether the layer takes keys: the layers `top` can name.
    const fn takes_keys(self) -> bool {
        matches!(self, Rank::Popup | Rank::Modal { .. } | Rank::ModalChrome)
    }

    pub(crate) const fn is_modal(self) -> bool {
        matches!(self, Rank::Modal { .. })
    }
}

/// Why a layer closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CloseReason {
    /// Code asked for it.
    Requested,
    /// Escape reached the top layer.
    Escape,
    /// A press landed outside the layer and its anchor.
    Outside,
    /// Focus left the layer.
    FocusLeft,
    /// A component inside was activated.
    Activated,
    /// Enter was pressed inside.
    Enter,
    /// The timeout ran out.
    Timeout,
    /// The anchor is no longer shown.
    AnchorGone,
}

/// What `any_open` counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenFilter {
    /// Every layer.
    Any,
    /// Modal layers.
    Modal,
    /// Layers that take keys: popups and modals.
    Keys,
}

/// One shown egui layer.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct OpenEntry {
    /// The layer's id.
    pub id: Id,
    /// The layer it was opened from inside.
    pub parent: Option<Id>,
    /// Its rank.
    pub rank: Rank,
    /// Its key class.
    pub keys: OwnerClass,
    /// The anchor in screen space.
    pub anchor: Option<Rect>,
    /// The layer's rect in screen space.
    pub rect: Rect,
    /// The side of the anchor it took.
    pub side: Side,
    /// It is fading out: not `top`, not counted by `any_open`, taking no input.
    pub exiting: bool,
    /// The layer's egui id and the cover under it, listed in the order.
    pub(crate) layer: Layer,
    pub(crate) cover: Option<Layer>,
}

impl OpenEntry {
    /// An entry for the layer `id` of `rank`.
    pub(crate) fn new(id: Id, rank: Rank, keys: OwnerClass, rect: Rect, layer: Layer) -> Self {
        OpenEntry { id, parent: None, rank, keys, anchor: None, rect, side: Side::Below, exiting: false, layer, cover: None }
    }
}

#[derive(Default)]
struct Pass {
    current: Vec<OpenEntry>,
    last: Vec<OpenEntry>,
    /// The layers whose bodies are running, innermost last.
    drawing: Vec<Id>,
}

#[derive(Default)]
struct Persist {
    raised: HashMap<Id, u64>,
    counter: u64,
    modal: bool,
    /// A press the modal drop removed, for the window host to raise the modal's window.
    dropped_press: bool,
}

#[derive(Default)]
struct Closes(Vec<(Id, CloseReason)>);

fn with_pass<R>(ctx: &Context, f: impl FnOnce(&mut Pass) -> R) -> R {
    let viewport = pass::current_viewport(ctx);
    PerViewport::<Stamped<Pass>>::with_pass(
        ctx,
        viewport,
        pass::pass_nr(ctx, viewport),
        |pass, step| {
            pass.last = std::mem::take(&mut pass.current);
            if matches!(step, PassStep::Skipped) {
                pass.last.clear();
            }
            pass.drawing.clear();
        },
        f,
    )
}

fn with_persist<R>(ctx: &Context, viewport: ViewportId, f: impl FnOnce(&mut Persist) -> R) -> R {
    PerViewport::<Persist>::with(ctx, viewport, f)
}

/// Whether the layer `id` was shown last frame.
pub fn is_open(ctx: &Context, id: Id) -> bool {
    with_pass(ctx, |pass| pass.last.iter().any(|entry| entry.id == id))
}

/// Queues a close of the layer `id`. The owner's step 9 or `OverlayUi::show` collects it and step 15 writes it.
pub fn request_close(ctx: &Context, id: Id, why: CloseReason) {
    Service::<Closes>::of(ctx).with(|closes| closes.0.push((id, why)));
    ctx.request_repaint();
}

/// The queued close of `id`, once.
pub(crate) fn take_close(ctx: &Context, id: Id) -> Option<CloseReason> {
    Service::<Closes>::of(ctx).with(|closes| {
        let at = closes.0.iter().position(|(closed, _)| *closed == id)?;
        let reason = closes.0[at].1;
        closes.0.retain(|(closed, _)| *closed != id);
        Some(reason)
    })
}

/// Whether any layer of the running viewport that `filter` counts was shown last frame.
pub fn any_open(ctx: &Context, filter: OpenFilter) -> bool {
    with_pass(ctx, |pass| {
        pass.last.iter().filter(|entry| !entry.exiting).any(|entry| match filter {
            OpenFilter::Any => true,
            OpenFilter::Modal => entry.rank.is_modal(),
            OpenFilter::Keys => entry.rank.takes_keys(),
        })
    })
}

/// The layer that receives Cancel: the newest member of the topmost family that takes keys.
pub fn top(ctx: &Context) -> Option<Id> {
    let viewport = pass::current_viewport(ctx);
    let raised = with_persist(ctx, viewport, |persist| persist.raised.clone());
    with_pass(ctx, |pass| {
        let live: Vec<OpenEntry> = pass.last.iter().filter(|entry| !entry.exiting).cloned().collect();
        let families = families(&live, &raised);
        families.iter().rev().find_map(|members| members.iter().rev().find(|entry| entry.rank.takes_keys()).map(|entry| entry.id))
    })
}

/// `id` and everything opened from inside it, shown last frame.
pub fn family(ctx: &Context, id: Id) -> SmallVec<[Id; 4]> {
    with_pass(ctx, |pass| {
        let mut out: SmallVec<[Id; 4]> = SmallVec::new();
        if pass.last.iter().any(|entry| entry.id == id) {
            out.push(id);
        }
        // Entries are recorded parent first, so one sweep finds every descendant.
        for entry in &pass.last {
            if entry.parent.is_some_and(|parent| out.contains(&parent)) && !out.contains(&entry.id) {
                out.push(entry.id);
            }
        }
        out
    })
}

/// The entry of the layer `id` shown last frame.
pub fn entry(ctx: &Context, id: Id) -> Option<OpenEntry> {
    with_pass(ctx, |pass| pass.last.iter().find(|entry| entry.id == id).cloned())
}

/// The parent a layer opened now has: the layer whose body is running.
pub(crate) fn drawing_parent(ctx: &Context) -> Option<Id> {
    with_pass(ctx, |pass| pass.drawing.last().copied())
}

/// Marks the layer `id`'s body as running, so layers opened from inside it become its children.
pub(crate) fn begin_layer(ctx: &Context, id: Id) {
    with_pass(ctx, |pass| pass.drawing.push(id));
}

/// Ends the body `begin_layer` started.
pub(crate) fn end_layer(ctx: &Context) {
    with_pass(ctx, |pass| {
        pass.drawing.pop();
    });
}

/// Records the layer shown in this pass; a layer shown again in a discarded pass replaces its entry.
pub(crate) fn record(ctx: &Context, entry: OpenEntry) {
    with_pass(ctx, |pass| {
        match pass.current.iter().position(|recorded| recorded.id == entry.id) {
            Some(at) => pass.current[at] = entry,
            None => pass.current.push(entry),
        };
    });
}

/// Groups `entries` into families, each parent first, ordered bottom to top.
fn families(entries: &[OpenEntry], raised: &HashMap<Id, u64>) -> Vec<Vec<OpenEntry>> {
    let root_of = |entry: &OpenEntry| {
        let mut at = entry;
        while let Some(parent) = at.parent.and_then(|parent| entries.iter().find(|other| other.id == parent)) {
            at = parent;
        }
        at.id
    };
    let mut groups: Vec<(Id, Vec<OpenEntry>)> = Vec::new();
    for entry in entries {
        let root = root_of(entry);
        match groups.iter_mut().find(|(id, _)| *id == root) {
            Some((_, members)) => members.push(entry.clone()),
            None => groups.push((root, vec![entry.clone()])),
        }
    }
    let class = |members: &Vec<OpenEntry>| members.first().map_or(0, |root| root.rank.class());
    groups.sort_by_key(|(root, members)| (class(members), raised.get(root).copied().unwrap_or(0)));
    groups.into_iter().map(|(_, members)| members).collect()
}

/// The end hook: raises the families that appeared or were pressed, hands egui the whole order, and records the
/// open-modal fact.
pub(crate) fn end_pass(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    let (current, last_ids) = with_pass(ctx, |pass| (pass.current.clone(), pass.last.iter().map(|entry| entry.id).collect::<Vec<_>>()));
    let pressed = ctx.input(|input| input.pointer.primary_pressed().then(|| input.pointer.interact_pos()).flatten());
    let order = with_persist(ctx, viewport, |persist| {
        let stamp = |id: Id, persist: &mut Persist| {
            persist.counter += 1;
            persist.raised.insert(id, persist.counter);
        };
        let live = |entry: &&OpenEntry| !entry.exiting;
        let roots: Vec<Id> = current.iter().filter(|entry| entry.parent.is_none_or(|parent| !current.iter().any(|other| other.id == parent))).map(|entry| entry.id).collect();
        for root in &roots {
            if !last_ids.contains(root) {
                stamp(*root, persist);
            }
        }
        persist.raised.retain(|id, _| roots.contains(id));
        let before = families(&current, &persist.raised);
        if let Some(point) = pressed
            && let Some(hit) = before.iter().rev().find(|members| members.iter().filter(live).any(|entry| entry.rect.contains(point)))
            && let Some(root) = hit.first()
            && hit.iter().all(|entry| entry.rank.class() == root.rank.class() || entry.parent.is_some())
        {
            stamp(root.id, persist);
        }
        let families = families(&current, &persist.raised);
        let mut order: Vec<Layer> = Vec::new();
        for members in &families {
            for member in members {
                order.extend(member.cover);
                order.push(member.layer);
            }
        }
        persist.modal = current.iter().any(|entry| entry.rank.is_modal() && !entry.exiting);
        order
    });
    areas::set_layer_order(ctx, &order);
    let top_modal = current.iter().rev().find(|entry| entry.rank.is_modal() && !entry.exiting);
    if let Some(entry) = top_modal {
        areas::set_modal_layer(ctx, entry.layer);
    }
    let any = {
        let here = top_modal.is_some();
        here || modal_in_other_viewports(ctx, viewport)
    };
    commands::set_modal_open(ctx, any);
}

fn modal_in_other_viewports(ctx: &Context, running: ViewportId) -> bool {
    pass::live_viewports(ctx).into_iter().filter(|viewport| *viewport != running).any(|viewport| with_persist(ctx, viewport, |persist| persist.modal))
}

/// Whether a modal layer is open in a viewport other than `viewport`: the global open-modal fact the input hook
/// reads (H3).
pub(crate) fn modal_elsewhere(ctx: &Context, viewport: ViewportId) -> bool {
    modal_in_other_viewports(ctx, viewport)
}

/// Records that the input hook dropped a press for `viewport` because a modal is open elsewhere.
pub(crate) fn note_dropped_press(ctx: &Context, viewport: ViewportId) {
    with_persist(ctx, viewport, |persist| persist.dropped_press = true);
}

/// Whether the input hook dropped a press for `viewport` since the last call; the window host raises the modal's
/// window for it.
#[expect(dead_code, reason = "the window host (F7) raises the modal's window for a dropped press")]
pub(crate) fn take_dropped_press(ctx: &Context, viewport: ViewportId) -> bool {
    with_persist(ctx, viewport, |persist| std::mem::take(&mut persist.dropped_press))
}

/// Whether a layer of key class `class` that is not fading out was shown last frame.
pub(crate) fn class_open(ctx: &Context, class: OwnerClass) -> bool {
    with_pass(ctx, |pass| pass.last.iter().any(|entry| !entry.exiting && entry.keys == class))
}
