//! Dragging a document tab: reorder in its strip, tear it off into a floating window, or drop
//! it into another document group (F16.07). Built on [`crate::tab_strip::TabStrip`]'s drag and
//! the dock's drop zones.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Context, DragAndDrop, Id, Pos2, Rect, Ui, Vec2};

use super::look::DockLook;
use crate::behavior::{DropPosition, ListDrag};
use crate::direction;
use crate::edit::{CancelReason, EditEvent};
use crate::tab_strip::{Tab, TabEvent, TabStrip};
use crate::theme::StrokeExt;

/// Where a torn-off window's top left sits from the pointer, so its title bar stays under the
/// pointer as the preview showed.
const GRAB: Vec2 = Vec2::new(40.0, 12.0);

/// What a strip knows of itself after a pass, for the drags of other strips.
#[derive(Clone)]
struct StripInfo<D> {
    rect: Rect,
    /// Every document in the strip's order.
    documents: Vec<D>,
    /// The shown tabs: host index and rect.
    shown: Vec<(usize, Rect)>,
    rtl: bool,
    /// The strip's id, which names it as the source of a [`ListDrag`].
    strip: Id,
}

impl<D> StripInfo<D> {
    /// The host index a drop at `pos` opens at (the rule of `StripPlan::slot_at`).
    fn slot_at(&self, pos: Pos2) -> usize {
        let past = |rect: &Rect| if self.rtl { rect.center().x < pos.x } else { rect.center().x > pos.x };
        match self.shown.iter().find(|(_, rect)| past(rect)) {
            Some(&(index, _)) => index,
            None => self.shown.last().map_or(0, |&(index, _)| index + 1),
        }
    }
}

/// The strip rects of the last two passes that drew a strip, newest first: a strip drawn early in
/// a pass sees the strips drawn later by the previous pass.
#[derive(Clone, Default)]
struct Registry {
    pass: u64,
    current: Vec<(u64, Rect)>,
    previous: Vec<(u64, Rect)>,
}

fn registry_id() -> Id {
    Id::NULL.with("craft-ui-document-tab-strips")
}

fn info_id(group: u64) -> Id {
    Id::NULL.with(("craft-ui-document-tab-strip", group))
}

fn dragged_id(group: u64) -> Id {
    Id::NULL.with(("craft-ui-document-tab-dragged", group))
}

/// A document as its tab shows it.
pub struct DocumentTab<D> {
    pub id: D,
    pub title: String,
    pub modified: bool,
    pub closable: bool,
}

/// What a document tab drag did this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DocumentTabEvent<D> {
    Selected(D),
    CloseRequested(D),
    /// Reordered inside its group: now before `before` (`None`: last).
    Reordered {
        document: D,
        before: Option<D>,
    },
    /// Dragged out of its strip and released outside every group: float it at `at`.
    TornOff {
        document: D,
        at: Pos2,
    },
    /// Released over another group's strip, before `before`.
    DroppedInto {
        document: D,
        group: u64,
        before: Option<D>,
    },
}

/// One document group's tab strip with a drag that can leave it.
pub struct DocumentTabs<'a, D> {
    group: u64,
    tabs: &'a [DocumentTab<D>],
    selected: Option<&'a D>,
    strip: TabStrip<'a, D>,
}

impl<'a, D: Clone + Eq + Hash + Debug + Send + Sync + 'static> DocumentTabs<'a, D> {
    /// `group` identifies the strip for drops from other strips.
    pub fn new(group: u64, tabs: &'a [DocumentTab<D>], selected: Option<&'a D>, strip: TabStrip<'a, D>) -> Self {
        Self { group, tabs, selected, strip }
    }

    /// Draws the strip; a drag past the strip's edge shows the drop line in another strip or
    /// the torn-off preview. Escape cancels the drag and keeps the order.
    // pc: crates/ui-egui/src/canvas/tabs.rs:331 `tab_drag`, :362 `float_torn_off`, :388 `tab_drop_line`.
    pub fn show(self, ui: &mut Ui) -> Vec<DocumentTabEvent<D>> {
        let Self { group, tabs, selected, mut strip } = self;
        let ctx = ui.ctx().clone();
        let own = ctx.data(|d| d.get_temp::<StripInfo<D>>(info_id(group)));
        let pointer = ui.input(|i| i.pointer.latest_pos());

        // A tab of another strip over this one: the drop line shows where it would land.
        let foreign = DragAndDrop::payload::<ListDrag<D>>(&ctx).filter(|payload| own.as_ref().is_none_or(|own| own.strip != payload.list));
        let marker = foreign
            .as_ref()
            .and_then(|_| Some((own.as_ref()?, pointer?)))
            .filter(|(own, pointer)| own.rect.contains(*pointer))
            .map(|(own, pointer)| own.slot_at(pointer));

        let list: Vec<Tab<D>> = tabs.iter().map(|t| Tab::new(t.id.clone(), t.title.as_str()).modified(t.modified).closable(t.closable)).collect();
        strip = strip.tear_off(true).insertion_marker(marker);
        if let Some(selected) = selected {
            strip = strip.selected(selected);
        }
        let out = strip.show(ui, &list);

        let rtl = direction::is_rtl(ui);
        let shown = out.tabs.iter().filter_map(|(key, rect)| Some((tabs.iter().position(|t| &t.id == key)?, *rect))).collect();
        let info = StripInfo { rect: out.response.rect, documents: tabs.iter().map(|t| t.id.clone()).collect(), shown, rtl, strip: out.response.id };
        register(&ctx, group, info.rect);
        ctx.data_mut(|d| d.insert_temp(info_id(group), info));

        // The tab being dragged, kept for the release (the payload is gone by then).
        let dragged_key = dragged_id(group);
        if let Some(dragging) = &out.dragging_out {
            ctx.data_mut(|d| d.insert_temp(dragged_key, dragging.tab.clone()));
        } else if let Some(payload) = DragAndDrop::payload::<ListDrag<D>>(&ctx).filter(|p| p.list == out.response.id)
            && let Some(item) = payload.items.first()
        {
            ctx.data_mut(|d| d.insert_temp(dragged_key, item.clone()));
        }

        let released = ui.input(|i| i.pointer.any_released());
        let over = pointer.filter(|_| released).and_then(|p| strip_at::<D>(&ctx, group, p).map(|(target, info)| (target, info.slot_at(p), info)));
        if let Some(dragging) = &out.dragging_out
            && strip_at::<D>(&ctx, group, dragging.pointer).is_none()
        {
            paint_torn_off(ui, dragging.pointer);
        }

        let dragged = ctx.data(|d| d.get_temp::<D>(dragged_key));
        let mut events = Vec::new();
        let mut dropped = false;
        for event in out.events {
            match event {
                TabEvent::Activate(document) => events.push(DocumentTabEvent::Selected(document)),
                TabEvent::Close(document) => events.push(DocumentTabEvent::CloseRequested(document)),
                TabEvent::Reorder(EditEvent::Set { value, .. }) if !value.copy => {
                    if let Some(document) = value.items.first() {
                        let before = before_of(tabs, document, &value.target, value.position);
                        events.push(DocumentTabEvent::Reordered { document: document.clone(), before });
                    }
                }
                TabEvent::Reorder(EditEvent::Commit { value, changed, .. }) if over.is_none() => {
                    if let (true, false, Some(document)) = (changed, value.copy, value.items.first()) {
                        let before = before_of(tabs, document, &value.target, value.position);
                        events.push(DocumentTabEvent::Reordered { document: document.clone(), before });
                    }
                }
                TabEvent::Reorder(EditEvent::Commit { .. } | EditEvent::Cancel { reason: CancelReason::Dismissed, .. })
                | TabEvent::DroppedOut { .. }
                    if over.is_some() && !dropped =>
                {
                    let tab = match &event {
                        TabEvent::DroppedOut { tab, .. } => Some(tab.clone()),
                        TabEvent::Reorder(EditEvent::Commit { value, .. }) => value.items.first().cloned(),
                        _ => dragged.clone(),
                    };
                    if let (Some(document), Some((target, slot, info))) = (tab, &over) {
                        dropped = true;
                        events.push(DocumentTabEvent::DroppedInto { document, group: *target, before: info.documents.get(*slot).cloned() });
                    }
                }
                TabEvent::DroppedOut { .. } if over.is_some() => {}
                TabEvent::DroppedOut { tab, pointer } => events.push(DocumentTabEvent::TornOff { document: tab, at: pointer - GRAB }),
                _ => {}
            }
        }
        events
    }

    /// The strip rects every document group registered this pass, for cross-group drops. A strip
    /// drawn later in the pass than the caller shows with the rect of the previous pass.
    pub fn strips(ctx: &Context) -> Vec<(u64, Rect)> {
        registered(ctx)
    }
}

fn registered(ctx: &Context) -> Vec<(u64, Rect)> {
    ctx.data(|d| d.get_temp::<Registry>(registry_id())).map_or_else(Vec::new, |registry| {
        let older = registry.previous.iter().filter(|(g, _)| !registry.current.iter().any(|(c, _)| c == g)).copied();
        registry.current.iter().copied().chain(older).collect()
    })
}

/// Records the strip of `group` for this pass.
fn register(ctx: &Context, group: u64, rect: Rect) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let registry = d.get_temp_mut_or_default::<Registry>(registry_id());
        if registry.pass != pass {
            registry.previous = std::mem::take(&mut registry.current);
            registry.pass = pass;
        }
        registry.current.retain(|(g, _)| *g != group);
        registry.current.push((group, rect));
    });
}

/// The strip of another group under `pos`.
fn strip_at<D: Clone + Send + Sync + 'static>(ctx: &Context, own: u64, pos: Pos2) -> Option<(u64, StripInfo<D>)> {
    let (group, _) = registered(ctx).into_iter().find(|&(g, rect)| g != own && rect.contains(pos))?;
    ctx.data(|d| d.get_temp::<StripInfo<D>>(info_id(group))).map(|info| (group, info))
}

/// The document a move to `position` of `target` lands before; `None` for the end.
fn before_of<D: Clone + PartialEq>(tabs: &[DocumentTab<D>], document: &D, target: &D, position: DropPosition) -> Option<D> {
    if position != DropPosition::After {
        return Some(target.clone());
    }
    let at = tabs.iter().position(|t| &t.id == target)?;
    tabs[at + 1..].iter().map(|t| &t.id).find(|id| *id != document).cloned()
}

/// The outline of the window a torn-off tab becomes, under the pointer.
fn paint_torn_off(ui: &Ui, pointer: Pos2) {
    let look = DockLook::of(ui);
    let rect = Rect::from_min_size(pointer - GRAB, egui::vec2(look.float_width, look.float_height));
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, ui.scope_id().with("tab-tear-off")));
    painter.rect_filled(rect, 0.0, look.drop.gamma_multiply(look.ghost_fill));
    let stroke = craft_theme::scale::Stroke::W1.egui(ui.ctx(), look.drop.gamma_multiply(look.ghost_stroke));
    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
}
