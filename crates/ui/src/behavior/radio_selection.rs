//! [`RadioSelection`]: the keyboard, focus and selection behaviour of a WAI-ARIA radio group.

use egui::accesskit::Action;
use egui::{EventFilter, Id, Key, Modifiers, PointerButton, Sense, Ui};

use crate::direction;
use crate::primitives::Sensed;

/// The keyboard, focus and selection behaviour of a WAI-ARIA radio group: one of N items, each an
/// [`Interactive`](crate::primitives::Interactive) element, some of them possibly disabled.
///
/// - The group is one Tab stop: the selected item, or the first enabled one when none is
///   selected or the selected one is disabled. The other items take clicks but not the focus. A
///   group with no enabled item has no Tab stop.
/// - While an item has focus, focus follows the Tab stop, whether a key, a click or a change from
///   outside moved it. The arrow keys stay with the group from the first focused frame on, so
///   egui's own arrow-key focus movement never applies.
/// - Right and Down select the next enabled item, Left and Up the previous one, wrapping; Left and
///   Right swap in a right-to-left interface. Home and End select the first and last enabled
///   item. Disabled items are skipped.
/// - A click selects the clicked item, unless it is disabled.
///
/// A frame of a group: [`begin`](RadioSelection::begin) before the items, which handles focus and
/// keys; sense each item with [`sense`](RadioSelection::sense) and
/// [`Interactive::interact`](crate::primitives::Interactive::interact); pass the results to
/// [`sensed`](RadioSelection::sensed), which applies a click; then finish each item checked when
/// it is [`selected`](RadioSelection::selected). Every item thus reports and paints the
/// selection of this frame. Items placed apart, where one paints before another senses, take the
/// click with [`click_from_input`](RadioSelection::click_from_input) after `begin` instead.
#[derive(Debug)]
#[must_use = "a radio group senses its items and then calls `sensed`"]
pub struct RadioSelection {
    ids: Vec<Id>,
    enabled: Vec<bool>,
    selected: Option<usize>,
    focused: bool,
    changed: bool,
}

impl RadioSelection {
    /// The arrow keys, kept with the group while it has focus.
    const ARROWS: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

    /// Starts a frame of a group whose items are `items`: each item's id (the id its
    /// [`Interactive`](crate::primitives::Interactive) gets, e.g. from
    /// [`Interactive::id_in`](crate::primitives::Interactive::id_in)) and whether it is enabled.
    /// Item `selected` is selected; an index past the last item selects none. A group with no
    /// enabled item takes no keys and lets the focus go.
    pub fn begin(ui: &Ui, items: Vec<(Id, bool)>, selected: Option<usize>) -> Self {
        let (ids, enabled): (Vec<_>, Vec<_>) = items.into_iter().unzip();
        let selected = selected.filter(|&i| i < ids.len());
        let focused = ids.iter().zip(&enabled).any(|(id, on)| *on && ui.memory(|m| m.has_focus(*id)));
        let mut this = Self { ids, enabled, selected, focused, changed: false };
        this.follow(ui);
        if let Some(stop) = this.stop().filter(|_| this.focused) {
            ui.memory_mut(|m| m.set_focus_lock_filter(this.ids[stop], Self::ARROWS));
            if let Some(target) = this.key_target(ui, stop) {
                this.select(target);
                this.follow(ui);
            }
        }
        this
    }

    /// Applies this frame's click on an item before any item senses: a primary click egui
    /// assigned to an item at the start of the pass, Enter or Space on the focused Tab stop, or a
    /// screen reader's click. For items placed apart, where one paints before another senses;
    /// keeps the focus on the Tab stop.
    pub fn click_from_input(&mut self, ui: &Ui) {
        let pointer = ui.ctx().interaction_snapshot(|s| s.clicked).filter(|_| ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)));
        let key = ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        let stop = self.stop();
        let clicked = (0..self.ids.len()).find(|&i| {
            let id = self.ids[i];
            self.enabled[i]
                && (pointer == Some(id)
                    || (key && stop == Some(i) && ui.memory(|m| m.has_focus(id)))
                    || ui.input(|input| input.has_accesskit_action_request(id, Action::Click)))
        });
        if let Some(item) = clicked {
            self.select(item);
            self.follow(ui);
        }
    }

    /// How item `item` senses: [`Sense::click`] for the Tab stop, [`Sense::CLICK`] (no focus) for
    /// the others.
    pub fn sense(&self, item: usize) -> Sense {
        if Some(item) == self.stop() { Sense::click() } else { Sense::CLICK }
    }

    /// Applies a click on one of the `sensed` items, one per item in item order, and keeps the
    /// focus on the Tab stop. A click on a disabled item or past the last item is ignored.
    pub fn sensed(&mut self, ui: &Ui, sensed: &[Sensed]) {
        debug_assert_eq!(sensed.len(), self.ids.len(), "one sensed element per item");
        if let Some(clicked) = sensed.iter().position(|s| s.response().clicked()).filter(|&i| self.enabled.get(i) == Some(&true)) {
            self.select(clicked);
        }
        self.follow(ui);
    }

    /// The selected item after this frame's keys and click.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Whether a key or click selected another item this frame.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// The item that is the group's Tab stop: the selected item when enabled, else the first
    /// enabled one.
    fn stop(&self) -> Option<usize> {
        self.selected.filter(|&i| self.enabled[i]).or_else(|| self.enabled.iter().position(|on| *on))
    }

    fn select(&mut self, item: usize) {
        if self.selected != Some(item) {
            self.selected = Some(item);
            self.changed = true;
        }
    }

    /// The item a key press selects from the Tab stop `stop`, consuming the key.
    fn key_target(&self, ui: &Ui, stop: usize) -> Option<usize> {
        let count = self.ids.len();
        let enabled = |i: &usize| self.enabled[*i];
        let (next, previous) = if direction::is_rtl(ui) { (Key::ArrowLeft, Key::ArrowRight) } else { (Key::ArrowRight, Key::ArrowLeft) };
        let pressed = |key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if pressed(Key::Home) {
            (0..count).find(enabled)
        } else if pressed(Key::End) {
            (0..count).rev().find(enabled)
        } else if pressed(next) || pressed(Key::ArrowDown) {
            (1..=count).map(|step| (stop + step) % count).find(enabled)
        } else if pressed(previous) || pressed(Key::ArrowUp) {
            (1..=count).map(|step| (stop + count - step) % count).find(enabled)
        } else {
            None
        }
    }

    /// Moves the focus to the Tab stop while the group has it, with the arrow keys locked from
    /// the first focused frame on.
    fn follow(&self, ui: &Ui) {
        let Some(stop) = self.stop().filter(|_| self.focused) else { return };
        let stop = self.ids[stop];
        if !ui.memory(|m| m.has_focus(stop)) {
            ui.memory_mut(|m| m.request_focus_with_filter(stop, Self::ARROWS));
        }
    }
}
