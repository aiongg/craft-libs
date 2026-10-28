//! [`PartFocus`]: one Tab stop for an overlay made of parts (a transform box's handles, a path's
//! anchors, a warp mesh's points), with a picked part the keys act on.

use egui::accesskit::Role;
use egui::epaint::RectShape;
use egui::{CornerRadius, Event, EventFilter, Id, Key, Modifiers, Painter, Rect, Response, Sense, StrokeKind, Ui, UiBuilder, WidgetInfo};

use super::Claim;
use crate::primitives::FocusRing;
use crate::theme::{ToEgui, current};

/// One part of an overlay for the keyboard and the AccessKit tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    /// The overlay's key for the part, stable across passes.
    pub key: u64,
    /// The accessible name, e.g. "Top left corner" or "Anchor 3 of 12".
    pub name: String,
    /// The position a screen reader reads after the name, in the document's units.
    pub value: Option<String>,
    /// The part's screen rect: its AccessKit bounds and where a press gives the group focus.
    pub rect: Rect,
    /// Whether the part is selected (a path's selected anchors), for a screen reader.
    pub selected: bool,
}

/// What [`PartFocus::show`] reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartFocusResponse {
    /// The group's id: the overlay's own.
    pub id: Id,
    /// Whether the group has keyboard focus.
    pub focused: bool,
    /// A new picked part for the host: `Some(Some(key))` from Page Up, Page Down, Home or End, or
    /// the first part when the group gains focus from the keyboard with none picked; `Some(None)`
    /// when the picked part is no longer among the parts.
    pub pick: Option<Option<u64>>,
    /// Whether the group has focus and the last input came from the keyboard: the overlay draws
    /// [`PartFocus::ring`] round the picked part only then.
    pub keyboard: bool,
}

/// The keyboard model of an overlay made of parts, as the ruler guides have it: the overlay is one
/// Tab stop, and the keys act on one picked part.
///
/// ```
/// use craft_ui::canvas::{Part, PartFocus};
/// # egui::__run_test_ui(|ui| {
/// let (_, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let parts = [Part { key: 0, name: "Top left corner".into(), value: Some("0 px, 0 px".into()), rect: egui::Rect::from_center_size(canvas.rect.min, egui::Vec2::splat(12.0)), selected: false }];
/// let id = ui.make_persistent_id("box");
/// let r = PartFocus::new(id, "Free Transform").parts(&parts).picked(Some(0)).show(ui, &canvas);
/// if r.keyboard {
///     PartFocus::ring(ui.painter(), parts[0].rect);
/// }
/// # });
/// ```
///
/// - A focusable widget over the canvas is the AccessKit `Group`, named by the host; each part is
///   a hover-only child, a `GraphicsObject` with its name, value and `selected`. The picked part
///   is the group's active descendant.
/// - While the group has focus, Page Down and Page Up pick the next and previous part, Home and
///   End the first and last, in the order of [`parts`](Self::parts) (the overlay's order). The
///   arrows and Esc stay with the group (its focus lock filter); Enter, Delete, Backspace and
///   Insert are the overlay's to read.
/// - The release of a press the overlay owns ([`Claim`], taken with the same id) that began on a
///   part gives the group focus, so the keys act on what the pointer took.
/// - The overlay draws its parts and, while [`keyboard`](PartFocusResponse::keyboard), the focus
///   ring round the picked one.
pub struct PartFocus<'a> {
    id: Id,
    group: String,
    value: Option<String>,
    parts: &'a [Part],
    picked: Option<u64>,
    focusable: bool,
}

impl<'a> PartFocus<'a> {
    /// The overlay `id` (the one it claims presses with) as a group named `group`, e.g. "Free
    /// Transform".
    pub fn new(id: Id, group: impl Into<String>) -> Self {
        Self { id, group: group.into(), value: None, parts: &[], picked: None, focusable: true }
    }

    /// The parts, in keyboard order.
    pub fn parts(mut self, parts: &'a [Part]) -> Self {
        self.parts = parts;
        self
    }

    /// The host's picked part.
    pub fn picked(mut self, key: Option<u64>) -> Self {
        self.picked = key;
        self
    }

    /// The group's value for a screen reader, e.g. "W 120 %, H 100 %, 15°".
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Whether the group is a Tab stop; true by default. A group that isn't (the path being drawn)
    /// still puts its parts in the AccessKit tree.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Senses the keys over the host's `canvas` response, and adds the group and its parts to the
    /// AccessKit tree.
    pub fn show(self, ui: &mut Ui, canvas: &Response) -> PartFocusResponse {
        let ctx = ui.ctx().clone();
        let id = self.id;
        let tab_stop = self.focusable && !self.parts.is_empty() && ui.is_enabled();
        let group = ui.interact(canvas.rect, id, if tab_stop { Sense::FOCUSABLE } else { Sense::hover() });
        group.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::Group, ui.is_enabled(), &self.group);
            info.current_text_value = self.value.clone();
            info
        });
        let by_pointer = tab_stop && self.released_on_part(ui, canvas, id);
        if by_pointer {
            ui.memory_mut(|m| m.request_focus(id));
        }
        let focused = tab_stop && group.has_focus();
        let current = self.picked.and_then(|k| self.parts.iter().position(|p| p.key == k));
        let mut pick = None;
        if focused {
            pick = self.keys(ui, current).map(|i| Some(self.parts[i].key));
            // Focus from the keyboard picks a part to act on; the pointer picks its own.
            if pick.is_none() && current.is_none() && group.gained_focus() && !by_pointer {
                pick = Some(Some(self.parts[0].key));
            }
            let filter = EventFilter { horizontal_arrows: true, vertical_arrows: true, escape: true, ..EventFilter::default() };
            ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
        }
        if pick.is_none() && self.picked.is_some() && current.is_none() {
            pick = Some(None);
        }
        let picked = pick.unwrap_or(self.picked);
        self.nodes(ui, canvas.rect, id, picked);
        PartFocusResponse { id, focused, pick, keyboard: focused && FocusRing::visible(&ctx) }
    }

    /// Draws the focus ring round a part's screen `rect`, over an `overlay.halo` ring `handle.halo`
    /// wider on each side so it reads on any image. The overlay calls it while
    /// [`keyboard`](PartFocusResponse::keyboard).
    pub fn ring(painter: &Painter, rect: Rect) {
        let tokens = current(painter.ctx());
        let style = FocusRing::style(&tokens);
        let halo = tokens.component.handle.halo;
        let under = rect.expand(style.offset - halo);
        painter.add(RectShape::stroke(
            under,
            CornerRadius::ZERO,
            (style.stroke.width + 2.0 * halo, tokens.color.overlay.halo.to_egui()),
            StrokeKind::Outside,
        ));
        painter.add(FocusRing::ring(&tokens, CornerRadius::ZERO, rect));
    }

    /// The id of a part's AccessKit node in the group `id`.
    pub fn part_id(id: Id, key: u64) -> Id {
        id.with(("craft_ui::canvas::part", key))
    }

    /// Whether a press the overlay owns, begun on a part, was released in this pass.
    fn released_on_part(&self, ui: &Ui, canvas: &Response, id: Id) -> bool {
        ui.input(|i| i.pointer.primary_released())
            && Claim::owned_press(ui.ctx(), canvas.id, id).is_some_and(|press| self.parts.iter().any(|p| p.rect.contains(press.origin)))
    }

    /// Page Up, Page Down, Home and End with no modifier: the index of the part they pick.
    fn keys(&self, ui: &Ui, current: Option<usize>) -> Option<usize> {
        let last = self.parts.len().checked_sub(1)?;
        let take = |key: Key| {
            ui.input_mut(|i| {
                let at = i.events.iter().position(
                    |e| matches!(e, Event::Key { key: k, pressed: true, modifiers, .. } if *k == key && modifiers.matches_exact(Modifiers::NONE)),
                );
                at.map(|at| i.events.remove(at)).is_some()
            })
        };
        let (up, down, home, end) = (take(Key::PageUp), take(Key::PageDown), take(Key::Home), take(Key::End));
        match (up, down, home, end, current) {
            (_, _, true, false, _) => Some(0),
            (_, _, false, true, _) => Some(last),
            (true, false, _, _, Some(c)) => Some(c.saturating_sub(1)),
            (true, false, _, _, None) => Some(last),
            (false, true, _, _, Some(c)) => Some((c + 1).min(last)),
            (false, true, _, _, None) => Some(0),
            _ => None,
        }
    }

    /// AccessKit: the parts as graphics objects in the group; the picked one is its active
    /// descendant.
    fn nodes(&self, ui: &mut Ui, rect: Rect, id: Id, picked: Option<u64>) {
        let nodes = ui.new_child(UiBuilder::new().max_rect(rect).accessibility_parent(id));
        for part in self.parts {
            let response = nodes.interact(part.rect, PartFocus::part_id(id, part.key), Sense::hover());
            response.widget_info(|| {
                let mut info = WidgetInfo::labeled(Role::GraphicsObject, nodes.is_enabled(), &part.name);
                info.current_text_value = part.value.clone();
                info
            });
            ui.ctx().accesskit_node_builder(response.id, |node| node.set_selected(part.selected));
        }
        let active = picked.filter(|&k| self.parts.iter().any(|p| p.key == k));
        ui.ctx().accesskit_node_builder(id, |node| match active {
            Some(key) => node.set_active_descendant(PartFocus::part_id(id, key).accesskit_id()),
            None => node.clear_active_descendant(),
        });
    }
}

#[cfg(test)]
#[path = "part_focus_tests.rs"]
mod tests;
