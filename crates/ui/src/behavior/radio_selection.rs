//! [`RadioSelection`]: the keyboard, focus and selection behaviour of a WAI-ARIA radio group.

use egui::{EventFilter, Id, Key, Modifiers, Sense, Ui};

use crate::direction;
use crate::primitives::Sensed;

/// The keyboard, focus and selection behaviour of a WAI-ARIA radio group: one of N items, each an
/// [`Interactive`](crate::primitives::Interactive) element.
///
/// - The group is one Tab stop: the selected item, or the first when none is. The other items
///   take clicks but not the focus.
/// - While an item has focus, focus follows the Tab stop, whether a key, a click or a change from
///   outside moved it. The arrow keys stay with the group from the first focused frame on, so
///   egui's own arrow-key focus movement never applies.
/// - Right and Down select the next item, Left and Up the previous one, wrapping; Left and Right
///   swap in a right-to-left interface. Home and End select the first and last.
/// - A click selects the clicked item.
///
/// A frame of a group: [`begin`](RadioSelection::begin) before the items, which handles focus and
/// keys; sense each item with [`sense`](RadioSelection::sense) and
/// [`Interactive::interact`](crate::primitives::Interactive::interact); pass the results to
/// [`sensed`](RadioSelection::sensed), which applies a click; then finish each item checked when
/// it is [`selected`](RadioSelection::selected). Every item thus reports and paints the
/// selection of this frame.
#[derive(Debug)]
#[must_use = "a radio group senses its items and then calls `sensed`"]
pub struct RadioSelection {
    ids: Vec<Id>,
    selected: Option<usize>,
    focused: bool,
    changed: bool,
}

impl RadioSelection {
    /// The arrow keys, kept with the group while it has focus.
    const ARROWS: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

    /// Starts a frame of a group whose items have `ids` (each from
    /// [`Interactive::id_in`](crate::primitives::Interactive::id_in)), with item `selected`
    /// selected. A disabled group takes no keys and lets the focus go.
    pub fn begin(ui: &Ui, ids: Vec<Id>, selected: Option<usize>, enabled: bool) -> Self {
        let focused = enabled && ids.iter().any(|id| ui.memory(|m| m.has_focus(*id)));
        let mut this = Self { ids, selected, focused, changed: false };
        if this.ids.is_empty() {
            return this;
        }
        this.follow(ui);
        if this.focused {
            ui.memory_mut(|m| m.set_focus_lock_filter(this.ids[this.stop()], Self::ARROWS));
            if let Some(target) = this.key_target(ui) {
                this.select(target);
            }
        }
        this
    }

    /// How item `item` senses: [`Sense::click`] for the Tab stop, [`Sense::CLICK`] (no focus) for
    /// the others.
    pub fn sense(&self, item: usize) -> Sense {
        if item == self.stop() { Sense::click() } else { Sense::CLICK }
    }

    /// Applies a click on one of the `sensed` items, in item order, and keeps the focus on the
    /// Tab stop.
    pub fn sensed(&mut self, ui: &Ui, sensed: &[Sensed]) {
        if let Some(clicked) = sensed.iter().position(|s| s.response().clicked()) {
            self.select(clicked);
        }
        if !self.ids.is_empty() {
            self.follow(ui);
        }
    }

    /// The selected item after this frame's keys and click.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Whether a key or click selected another item this frame.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// The item that is the group's Tab stop.
    fn stop(&self) -> usize {
        self.selected.unwrap_or(0)
    }

    fn select(&mut self, item: usize) {
        if self.selected != Some(item) {
            self.selected = Some(item);
            self.changed = true;
        }
    }

    /// The item a key press selects, consuming the key.
    fn key_target(&self, ui: &Ui) -> Option<usize> {
        let (count, stop) = (self.ids.len(), self.stop());
        let (next, previous) = if direction::is_rtl(ui) { (Key::ArrowLeft, Key::ArrowRight) } else { (Key::ArrowRight, Key::ArrowLeft) };
        let pressed = |key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if pressed(Key::Home) {
            Some(0)
        } else if pressed(Key::End) {
            Some(count - 1)
        } else if pressed(next) || pressed(Key::ArrowDown) {
            Some((stop + 1) % count)
        } else if pressed(previous) || pressed(Key::ArrowUp) {
            Some((stop + count - 1) % count)
        } else {
            None
        }
    }

    /// Moves the focus to the Tab stop while the group has it, with the arrow keys locked from
    /// the first focused frame on.
    fn follow(&self, ui: &Ui) {
        let stop = self.ids[self.stop()];
        if self.focused && !ui.memory(|m| m.has_focus(stop)) {
            ui.memory_mut(|m| m.request_focus_with_filter(stop, Self::ARROWS));
        }
    }
}
