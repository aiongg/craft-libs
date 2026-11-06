//! A dialog's draft value under a gesture: written by every event, restored when the gesture is
//! cancelled.

use super::EditEvent;

/// What an event did to the draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DraftStep {
    /// A `Begin` or `Update` previewed a value; the gesture is still running.
    Previewed,
    /// A `Commit` ended the gesture; the draft holds its final value.
    Committed,
    /// A `Cancel` ended the gesture; the draft holds the value from before its `Begin`.
    Restored,
    /// A `Set` wrote a value outside any gesture.
    Set,
}

/// The state a dialog keeps beside one draft value while a control edits it: the value from
/// before the gesture's [`Begin`](EditEvent::Begin).
///
/// A control inside a [`scope`](super::scope) reports only previews, and a [`Cancel`] carries no
/// value, so a dialog that writes every `Update` into its draft keeps the preview after Esc or a
/// dismissed popup. `GestureDraft` restores the saved value on `Cancel`. A dialog that edits
/// several values at once keeps one `GestureDraft` per control, or one over the compound value.
///
/// [`Cancel`]: EditEvent::Cancel
#[derive(Clone, Debug, PartialEq)]
pub struct GestureDraft<T> {
    before: Option<T>,
}

impl<T> Default for GestureDraft<T> {
    fn default() -> Self {
        Self { before: None }
    }
}

impl<T: Clone> GestureDraft<T> {
    /// A draft with no gesture running.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a gesture is running (a `Begin` came and no end yet).
    pub fn active(&self) -> bool {
        self.before.is_some()
    }

    /// Applies `event` to `draft`. `Begin` saves the current value, `Update` and `Set` write
    /// theirs, `Commit` writes its value when it `changed`, and `Cancel` restores the saved
    /// value. A `Commit` or `Cancel` without a `Begin` (a gesture the dialog joined late) writes
    /// or restores nothing it has no value for.
    pub fn apply(&mut self, draft: &mut T, event: EditEvent<T>) -> DraftStep {
        match event {
            EditEvent::Begin { value, .. } => {
                self.before = Some(draft.clone());
                if let Some(value) = value {
                    *draft = value;
                }
                DraftStep::Previewed
            }
            EditEvent::Update { value, .. } => {
                *draft = value;
                DraftStep::Previewed
            }
            EditEvent::Commit { value, changed, .. } => {
                if changed {
                    *draft = value;
                }
                self.before = None;
                DraftStep::Committed
            }
            EditEvent::Cancel { .. } => {
                if let Some(before) = self.before.take() {
                    *draft = before;
                }
                DraftStep::Restored
            }
            EditEvent::Set { value, .. } => {
                *draft = value;
                DraftStep::Set
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::Modifiers;

    use super::*;
    use crate::edit::{CancelReason, GestureId};

    #[test]
    fn a_cancel_restores_the_value_from_before_the_begin() {
        let (mut draft, mut gesture, id) = (10, GestureDraft::new(), GestureId::next());
        assert_eq!(gesture.apply(&mut draft, EditEvent::begin(id)), DraftStep::Previewed);
        assert!(gesture.active());
        gesture.apply(&mut draft, EditEvent::update(id, 20, Modifiers::NONE));
        assert_eq!(draft, 20);
        assert_eq!(gesture.apply(&mut draft, EditEvent::cancel(id, CancelReason::Escape)), DraftStep::Restored);
        assert_eq!((draft, gesture.active()), (10, false));
    }

    #[test]
    fn a_begin_with_a_value_previews_it_and_a_commit_keeps_the_final_value() {
        let (mut draft, mut gesture, id) = (1, GestureDraft::new(), GestureId::next());
        gesture.apply(&mut draft, EditEvent::begin_with(id, 2));
        assert_eq!(draft, 2);
        let step = gesture.apply(&mut draft, EditEvent::commit(id, 3, true, Modifiers::NONE));
        assert_eq!((step, draft, gesture.active()), (DraftStep::Committed, 3, false));
        // A later cancel has nothing saved to restore.
        gesture.apply(&mut draft, EditEvent::cancel(id, CancelReason::Escape));
        assert_eq!(draft, 3);
    }

    #[test]
    fn an_unchanged_commit_and_a_set_follow_their_flags() {
        let (mut draft, mut gesture, id) = (5, GestureDraft::new(), GestureId::next());
        gesture.apply(&mut draft, EditEvent::begin(id));
        gesture.apply(&mut draft, EditEvent::commit(id, 9, false, Modifiers::NONE));
        assert_eq!(draft, 5);
        assert_eq!(gesture.apply(&mut draft, EditEvent::set(7, Modifiers::NONE)), DraftStep::Set);
        assert_eq!(draft, 7);
    }
}
