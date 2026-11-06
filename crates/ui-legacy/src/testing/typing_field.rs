//! The typing-run helper of canvas overlay tests.

use egui::{Rect, pos2, vec2};

use crate::edit::{EditEvent, EditPolicy, Gesture};

/// A text field's typing run beside a canvas under test: begun with the focus in the first pass it
/// is drawn, ended by whatever takes the focus. It records the pass of its Commit and the pass of
/// the overlay's first event, so a test can tell which went out first.
#[derive(Default)]
pub struct TypingField {
    /// Whether the field is drawn with the focus.
    pub on: bool,
    passes: usize,
    began: bool,
    committed: Option<usize>,
    first_event: Option<usize>,
}

impl TypingField {
    /// Draws the field; call it in every pass, before the overlay.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.passes += 1;
        if !self.on {
            return;
        }
        let first = !self.began;
        self.began = true;
        let id = ui.make_persistent_id("typing field");
        let response = ui.interact(Rect::from_min_size(pos2(0.0, 0.0), vec2(20.0, 20.0)), id, egui::Sense::click());
        if first {
            response.request_focus();
        }
        let mut gesture = Gesture::load(ui, id, &0, EditPolicy::default());
        if first {
            gesture.begin();
        }
        if matches!(gesture.finish(), Some(EditEvent::Commit { .. })) {
            self.committed = Some(self.passes);
        }
    }

    /// Records the overlay's result of this pass.
    pub fn note(&mut self, event: bool) {
        if event && self.first_event.is_none() {
            self.first_event = Some(self.passes);
        }
    }

    /// The field's Commit went out in a pass before the overlay's first event.
    pub fn commit_came_first(&self) -> bool {
        matches!((self.committed, self.first_event), (Some(c), Some(e)) if c < e)
    }
}
