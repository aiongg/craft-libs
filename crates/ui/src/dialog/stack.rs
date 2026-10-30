//! [`DialogStack`]: which open modal dialog takes the input.

use egui::{Context, Id, LayerId};

/// The id a [`DialogStack`] knows a dialog by: [`Dialog::id`](super::Dialog::id).
pub type DialogId = Id;

/// The id of the dialog [`Dialog::new`](super::Dialog::new) builds from `key`, for a host that
/// pushes it on a [`DialogStack`] before it first shows the dialog.
pub fn dialog_id(key: impl std::hash::Hash + std::fmt::Debug) -> DialogId {
    Id::unique(("craft_ui::dialog", key))
}

/// The open modal dialogs in the order they were opened, kept by the host next to its dialog
/// states. [`Dialog::stack`](super::Dialog::stack) makes only the top one interactive: the older
/// ones are disabled whole, and a dialog that becomes the top moves to the front.
///
/// A nested dialog (a contour editor opened from a layer style dialog) is pushed with its parent
/// and shown from the parent's body. While a parent has a child its OK is out of reach: the
/// parent is disabled with everything else, and Enter and Esc go to the child.
///
/// ```
/// # use craft_ui::dialog::{DialogStack, dialog_id};
/// let (style, contour) = (dialog_id("layer-style"), dialog_id("contour"));
/// let mut stack = DialogStack::default();
/// stack.push(style, None);
/// stack.push(contour, Some(style));
/// assert!(stack.is_top(contour) && stack.has_child(style));
/// stack.remove_tree(style);
/// assert!(stack.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DialogStack {
    entries: Vec<(DialogId, Option<DialogId>)>,
}

impl DialogStack {
    /// Opens `id` on top, under `parent` if it is a child. Pushing an id already in the stack
    /// changes nothing, so a host may push every pass.
    pub fn push(&mut self, id: DialogId, parent: Option<DialogId>) {
        if !self.contains(id) {
            self.entries.push((id, parent));
        }
    }

    /// Removes `id` alone; its children, if any, stay.
    pub fn remove(&mut self, id: DialogId) {
        self.entries.retain(|(entry, _)| *entry != id);
    }

    /// Removes `id` and every dialog below it in the parent chain: cancelling a parent cancels the
    /// dialogs it opened.
    pub fn remove_tree(&mut self, id: DialogId) {
        let mut doomed = vec![id];
        for (entry, parent) in &self.entries {
            if parent.is_some_and(|p| doomed.contains(&p)) {
                doomed.push(*entry);
            }
        }
        self.entries.retain(|(entry, _)| !doomed.contains(entry));
    }

    pub fn contains(&self, id: DialogId) -> bool {
        self.entries.iter().any(|(entry, _)| *entry == id)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The dialog that takes the input: the last one opened.
    pub fn top(&self) -> Option<DialogId> {
        self.entries.last().map(|(id, _)| *id)
    }

    pub fn is_top(&self, id: DialogId) -> bool {
        self.top() == Some(id)
    }

    /// Whether another dialog was opened from `id` and is still open.
    pub fn has_child(&self, id: DialogId) -> bool {
        self.entries.iter().any(|(_, parent)| *parent == Some(id))
    }

    /// The dialogs opened from `id`, in opening order.
    pub fn children(&self, id: DialogId) -> impl Iterator<Item = DialogId> + '_ {
        self.entries.iter().filter(move |(_, parent)| *parent == Some(id)).map(|(entry, _)| *entry)
    }

    /// Every dialog in opening order.
    pub fn ids(&self) -> impl Iterator<Item = DialogId> + '_ {
        self.entries.iter().map(|(id, _)| *id)
    }

    /// Whether `id` is in the stack under another dialog. A dialog that was never pushed is not
    /// blocked.
    pub(super) fn blocks(&self, id: DialogId) -> bool {
        self.contains(id) && !self.is_top(id)
    }
}

/// Whether egui draws one of the `others` above `live`.
pub(super) fn older_above(ctx: &Context, live: LayerId, others: impl Iterator<Item = LayerId>) -> bool {
    let others: Vec<LayerId> = others.collect();
    ctx.memory(|m| layer_above(m.layer_ids(), live, &others))
}

/// `order` runs back to front.
fn layer_above(order: impl Iterator<Item = LayerId>, live: LayerId, others: &[LayerId]) -> bool {
    order.skip_while(|layer| *layer != live).skip(1).any(|layer| others.contains(&layer))
}

#[cfg(test)]
mod tests {
    use egui::Order;

    use super::*;

    fn layer(name: &str) -> LayerId {
        LayerId::new(Order::Foreground, Id::unique(name))
    }

    #[test]
    fn the_last_opened_is_the_top_and_parents_know_their_children() {
        let (a, b, c) = (dialog_id("a"), dialog_id("b"), dialog_id("c"));
        let mut stack = DialogStack::default();
        stack.push(a, None);
        stack.push(b, Some(a));
        stack.push(b, Some(a));
        stack.push(c, None);
        assert_eq!(stack.ids().collect::<Vec<_>>(), [a, b, c]);
        assert!(stack.is_top(c) && stack.has_child(a) && !stack.has_child(b));
        assert_eq!(stack.children(a).collect::<Vec<_>>(), [b]);
        assert!(stack.blocks(a) && stack.blocks(b) && !stack.blocks(c) && !stack.blocks(dialog_id("never pushed")));
        stack.remove(c);
        assert!(stack.is_top(b));
    }

    #[test]
    fn remove_tree_takes_the_descendants_and_leaves_the_rest() {
        let (a, b, c, d) = (dialog_id("a"), dialog_id("b"), dialog_id("c"), dialog_id("d"));
        let mut stack = DialogStack::default();
        stack.push(a, None);
        stack.push(b, Some(a));
        stack.push(c, Some(b));
        stack.push(d, None);
        stack.remove_tree(a);
        assert_eq!(stack.ids().collect::<Vec<_>>(), [d]);
    }

    #[test]
    fn a_dialog_is_raised_only_when_an_older_one_is_above_it() {
        let (a, b, c) = (layer("a"), layer("b"), layer("c"));
        assert!(layer_above([a, b, c].into_iter(), a, &[b]), "b is drawn above a");
        assert!(!layer_above([a, b, c].into_iter(), c, &[a, b]), "c is already on top");
        assert!(!layer_above([a, b, c].into_iter(), b, &[a]), "a is below b");
        assert!(!layer_above([a, b].into_iter(), c, &[a]), "an unknown layer has nothing above it");
    }
}
