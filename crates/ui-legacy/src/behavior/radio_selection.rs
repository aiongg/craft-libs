//! [`RadioSelection`]: the focus and selection behaviour of a radio group, as the suite's apps
//! have it.

use egui::accesskit::Action;
use egui::{Id, Key, PointerButton, Ui};

use crate::primitives::Sensed;

/// The focus and selection behaviour of a radio group: one of N items, each an
/// [`Interactive`](crate::primitives::Interactive) element, some of them possibly disabled. It
/// follows PhotoCraft's radio buttons and DesignCraft's segments:
///
/// - Every enabled item is a Tab stop; a disabled one is not.
/// - The arrow keys move the focus as they do between any controls (egui's focus movement, by
///   where the items sit): to the neighbouring item without selecting it, and on past the first or
///   last item to the next control. Disabled items take no focus, so the arrows pass over them.
/// - Enter or Space on the focused item, a click or a screen reader's click selects it, unless it
///   is disabled.
///
/// A frame of a group: [`new`](RadioSelection::new) before the items; sense each item with
/// [`Interactive::interact`](crate::primitives::Interactive::interact) (the default
/// [`Sense::click`](egui::Sense::click) takes the focus, Enter and Space); pass the results to
/// [`sensed`](RadioSelection::sensed), which applies a click; then finish each item checked when
/// it is [`selected`](RadioSelection::selected), at its
/// [position in the set](crate::primitives::Interactive::position_in_set), and report the set's
/// size on the group's node with [`report_size`](RadioSelection::report_size). Every item thus
/// reports and paints the selection of this frame. Items placed apart, where one paints before
/// another senses, take the click with [`click_from_input`](RadioSelection::click_from_input)
/// instead.
#[derive(Debug)]
#[must_use = "a radio group senses its items and then calls `sensed`"]
pub struct RadioSelection {
    ids: Vec<Id>,
    enabled: Vec<bool>,
    selected: Option<usize>,
    changed: bool,
}

impl RadioSelection {
    /// Starts a frame of a group whose items are `items`: each item's id (the id its
    /// [`Interactive`](crate::primitives::Interactive) gets, e.g. from
    /// [`Interactive::id_in`](crate::primitives::Interactive::id_in)) and whether it is enabled.
    /// Item `selected` is selected; an index past the last item selects none.
    pub fn new(items: Vec<(Id, bool)>, selected: Option<usize>) -> Self {
        let (ids, enabled): (Vec<_>, Vec<_>) = items.into_iter().unzip();
        let selected = selected.filter(|&i| i < ids.len());
        Self { ids, enabled, selected, changed: false }
    }

    /// Applies this frame's click on an item before any item senses: a primary click egui
    /// assigned to an item at the start of the pass, Enter or Space on the focused item, or a
    /// screen reader's click. For items placed apart, where one paints before another senses.
    pub fn click_from_input(&mut self, ui: &Ui) {
        let pointer = ui.ctx().interaction_snapshot(|s| s.clicked).filter(|_| ui.input(|i| i.pointer.button_clicked(PointerButton::Primary)));
        let key = ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        let clicked = (0..self.ids.len()).find(|&i| {
            let id = self.ids[i];
            self.enabled[i]
                && (pointer == Some(id)
                    || (key && ui.memory(|m| m.has_focus(id)))
                    || ui.input(|input| input.has_accesskit_action_request(id, Action::Click)))
        });
        if let Some(item) = clicked {
            self.select(item);
        }
    }

    /// Applies a click on one of the `sensed` items, one per item in item order. A click on a
    /// disabled item or past the last item is ignored.
    pub fn sensed(&mut self, sensed: &[Sensed]) {
        debug_assert_eq!(sensed.len(), self.ids.len(), "one sensed element per item");
        if let Some(clicked) = sensed.iter().position(|s| s.response().clicked()).filter(|&i| self.enabled.get(i) == Some(&true)) {
            self.select(clicked);
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

    /// Reports the number of items as the set size on the group's AccessKit node `group` (the
    /// `RadioGroup` node), so a screen reader says "2 of 5" with each item's position.
    pub fn report_size(&self, ui: &Ui, group: Id) {
        let count = self.ids.len();
        ui.ctx().accesskit_node_builder(group, |node| node.set_size_of_set(count));
    }

    fn select(&mut self, item: usize) {
        if self.selected != Some(item) {
            self.selected = Some(item);
            self.changed = true;
        }
    }
}
