//! [`SelectionOperation`]: how a new selection combines with the current one.

use std::borrow::Cow;

use egui::Modifiers;

use super::SelectionBadge;
use crate::localized::Words;

/// How a selection made by a click or a drag combines with the current one: the rule the Channels
/// panel's ⌘-click, a mask thumbnail's ⌘-click and the selection tools share (Photoshop's).
///
/// With the gesture's own modifier held (⌘ on a thumbnail; none for a selection tool), Shift adds,
/// Alt (Option) subtracts, Shift and Alt together intersect, and nothing else replaces
/// ([`from_modifiers`](Self::from_modifiers)). Command and Ctrl are not read: the caller has
/// already used them to recognise the gesture.
///
/// ```
/// use craft_ui::canvas::SelectionOperation;
/// # fn demo(modifiers: egui::Modifiers) {
/// let operation = SelectionOperation::from_modifiers(modifiers);
/// # let _ = operation;
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectionOperation {
    /// Replaces the selection.
    #[default]
    New,
    /// Adds to it.
    Add,
    /// Subtracts from it.
    Subtract,
    /// Keeps only the overlap.
    Intersect,
}

impl SelectionOperation {
    /// Every operation, in Photoshop's options-bar order.
    pub const ALL: [SelectionOperation; 4] =
        [SelectionOperation::New, SelectionOperation::Add, SelectionOperation::Subtract, SelectionOperation::Intersect];

    /// The operation `modifiers` ask for: Shift and Alt `Intersect`, Shift `Add`, Alt `Subtract`,
    /// else `New`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn from_modifiers(modifiers: Modifiers) -> SelectionOperation {
        // pc: crates/ui-egui/src/panels/channels_panel.rs:40 `load_operation` (match (shift, alt)).
        todo!()
    }

    /// The badge beside the selection pointer: `Add` a plus, `Subtract` a minus, `Intersect` a
    /// cross, `New` none ([`SelectionBadge::New`]).
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn badge(self) -> SelectionBadge {
        // new: one name per operation; `selection_cursor` draws the badge.
        todo!()
    }

    /// The operation's stable name for commands and settings: `"new"`, `"add"`, `"subtract"`,
    /// `"intersect"` (PhotoCraft's `select.loadSelection` operation values).
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub const fn name(self) -> &'static str {
        // pc: crates/ui-egui/src/panels/channels_panel.rs:40 `load_operation` (the same four strings).
        todo!()
    }

    /// The operation's name in the language of `words`, for menus and tooltips
    /// ([`SelectionOperationLabels`]).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn label(self, words: &Words) -> Cow<'static, str> {
        // new: as `ThumbSize::label`, one word per variant in `ALL` order.
        todo!()
    }
}

crate::words! {
    /// The names of the [`SelectionOperation`]s, Photoshop's options-bar tooltips.
    pub struct SelectionOperationLabels {
        new: "selection.operationNew" = "New Selection";
        add: "selection.operationAdd" = "Add to Selection";
        subtract: "selection.operationSubtract" = "Subtract from Selection";
        intersect: "selection.operationIntersect" = "Intersect with Selection";
    }
}

#[cfg(test)]
mod tests {
    // None gives New, Shift Add, Alt Subtract, Shift and Alt Intersect; Command and Ctrl change nothing.
    #[test]
    #[ignore = "skeleton"]
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    fn selection_operation_follows_modifiers() {
        todo!()
    }
}
