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
    pub fn from_modifiers(modifiers: Modifiers) -> SelectionOperation {
        match (modifiers.shift, modifiers.alt) {
            (true, true) => SelectionOperation::Intersect,
            (true, false) => SelectionOperation::Add,
            (false, true) => SelectionOperation::Subtract,
            (false, false) => SelectionOperation::New,
        }
    }

    /// The operation a selection tool's gesture uses (Photoshop's): the options bar's `bar`,
    /// overridden by `modifiers` as [`from_modifiers`](Self::from_modifiers) reads them (⇧ add, ⌥
    /// subtract, ⇧⌥ intersect). With nothing selected (`has_selection` false) there is nothing to
    /// subtract from or intersect with, so ⌥ or ⇧⌥ over a bar of `New` gives `New`; a bar of
    /// `Subtract` or `Intersect` is kept as it is (PhotoCraft #1106). The tool calls it with the
    /// modifiers of the press (or [`DraftEvent::Started`](super::draft::DraftEvent::Started)'s
    /// `intent`) and keeps the answer for the gesture; the mode badge before the press follows the
    /// live modifiers through the same call.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn resolve(bar: SelectionOperation, modifiers: Modifiers, has_selection: bool) -> SelectionOperation {
        // pc: crates/ui-egui/src/canvas/tool_feedback.rs:55-86 `selection_mode` (modifiers over the
        // bar), :92-100 `document_selection_mode` (⌥ with nothing selected and a New bar is New).
        todo!()
    }

    /// The badge beside the selection pointer: `Add` a plus, `Subtract` a minus, `Intersect` a
    /// cross, `New` none ([`SelectionBadge::New`]).
    pub fn badge(self) -> SelectionBadge {
        match self {
            SelectionOperation::New => SelectionBadge::New,
            SelectionOperation::Add => SelectionBadge::Add,
            SelectionOperation::Subtract => SelectionBadge::Subtract,
            SelectionOperation::Intersect => SelectionBadge::Intersect,
        }
    }

    /// The operation's stable name for commands and settings: `"new"`, `"add"`, `"subtract"`,
    /// `"intersect"` (PhotoCraft's `select.loadSelection` operation values).
    pub const fn name(self) -> &'static str {
        match self {
            SelectionOperation::New => "new",
            SelectionOperation::Add => "add",
            SelectionOperation::Subtract => "subtract",
            SelectionOperation::Intersect => "intersect",
        }
    }

    /// The operation's name in the language of `words`, for menus and tooltips
    /// ([`SelectionOperationLabels`]).
    pub fn label(self, words: &Words) -> Cow<'static, str> {
        let labels = words.labels::<SelectionOperationLabels>();
        let text = match self {
            SelectionOperation::New => &labels.new,
            SelectionOperation::Add => &labels.add,
            SelectionOperation::Subtract => &labels.subtract,
            SelectionOperation::Intersect => &labels.intersect,
        };
        Cow::Owned(text.clone())
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
    use egui::Modifiers;

    use super::*;

    // None gives New, Shift Add, Alt Subtract, Shift and Alt Intersect; Command and Ctrl change nothing.
    #[test]
    fn selection_operation_follows_modifiers() {
        let shift = Modifiers::SHIFT;
        let alt = Modifiers::ALT;
        assert_eq!(SelectionOperation::from_modifiers(Modifiers::NONE), SelectionOperation::New);
        assert_eq!(SelectionOperation::from_modifiers(shift), SelectionOperation::Add);
        assert_eq!(SelectionOperation::from_modifiers(alt), SelectionOperation::Subtract);
        assert_eq!(SelectionOperation::from_modifiers(shift | alt), SelectionOperation::Intersect);
        assert_eq!(SelectionOperation::from_modifiers(Modifiers::COMMAND | Modifiers::CTRL), SelectionOperation::New);
        assert_eq!(SelectionOperation::from_modifiers(Modifiers::COMMAND | shift), SelectionOperation::Add);
        for operation in SelectionOperation::ALL {
            assert_eq!(operation.badge() == SelectionBadge::New, operation == SelectionOperation::New);
        }
    }
}
