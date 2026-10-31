//! [`Preview`]: the Preview check box, hold-to-compare and the Before toggle of a dialog.

use egui::{Key, Response, Ui};

use crate::controls::Checkbox;

/// The preview flags of one dialog, held in [`DialogState`](super::DialogState) (or by the host,
/// when it draws the check box in the body).
///
/// Two modes use the same flags:
///
/// - **Proxy preview** (PhotoCraft adjustments and filters): the document stays untouched and the
///   host renders the result from the fields while [`shows_result`](Preview::shows_result). Cancel
///   drops the proxy, OK runs one command. The dialog has no outer gesture.
/// - **Edit preview** (Guide Layout, Fill, Curves): the dialog has an
///   [outer gesture](super::Dialog::outer_gesture). The host applies each update from the body
///   through `GestureHost::handle`, and when [`result_changed`](Preview::result_changed) reports
///   `Some(false)` it applies the original value as an update too (the controls keep their
///   values); `Some(true)` applies the edited value again. Cancel reverts to the gesture's mark,
///   OK squashes every edit into one step.
///
/// Hold-to-compare shows the unfiltered image while held; there is no split view. The Before
/// toggle (Camera Raw) is the same idea with a toggle control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preview {
    /// The Preview check box; on by default.
    pub on: bool,
    /// The compare target is held.
    pub compare: bool,
    /// The Before toggle is on.
    pub before: bool,
    /// What [`result_changed`](Preview::result_changed) last reported.
    reported: bool,
    /// The compare target ([`hold_to_compare`](Preview::hold_to_compare)) is held.
    target_held: bool,
    /// The Preview check box's label is held.
    label_held: bool,
}

impl Default for Preview {
    fn default() -> Self {
        Self { on: true, compare: false, before: false, reported: true, target_held: false, label_held: false }
    }
}

impl Preview {
    /// Draws the Preview check box. True when it changed this pass.
    /// Holding the box or its label shows the unfiltered image ([`compare`](Preview::compare)).
    pub fn checkbox(&mut self, ui: &mut Ui, label: impl Into<String>) -> bool {
        let response = Checkbox::bool(label, &mut self.on).id_salt("craft_ui::dialog::preview").show(ui).response;
        self.label_held = response.is_pointer_button_down_on();
        self.compare = self.target_held || self.label_held;
        response.changed()
    }

    /// Sets [`compare`](Preview::compare) from the compare target `response`: true while the
    /// pointer is held on it, or Space or Enter is held while it has the keyboard focus. The
    /// target must sense clicks (a button).
    pub fn hold_to_compare(&mut self, response: &Response) {
        let key = response.has_focus() && response.ctx.input(|i| i.key_down(Key::Space) || i.key_down(Key::Enter));
        self.target_held = response.is_pointer_button_down_on() || key;
        self.compare = self.target_held || self.label_held;
    }

    /// Draws the Before check box. True when it changed this pass.
    pub fn before_toggle(&mut self, ui: &mut Ui, label: impl Into<String>) -> bool {
        Checkbox::bool(label, &mut self.before).id_salt("craft_ui::dialog::before").show(ui).response.changed()
    }

    /// Whether the document shows the edited result: Preview on, and neither compare nor Before.
    pub fn shows_result(&self) -> bool {
        self.on && !self.compare && !self.before
    }

    /// `Some(shows_result)` on the pass it differs from the last one reported, else `None`. An
    /// edit preview applies the edited value on `Some(true)` and the original on `Some(false)`.
    pub fn result_changed(&mut self) -> Option<bool> {
        let now = self.shows_result();
        (std::mem::replace(&mut self.reported, now) != now).then_some(now)
    }
}

#[cfg(test)]
mod tests {
    use egui::{Context, Modifiers};

    use super::*;
    use crate::edit::{EditEvent, GestureHost, OuterGesture, UndoMarks};

    #[test]
    fn the_result_shows_only_with_preview_on_and_nothing_held() {
        let mut p = Preview::default();
        assert!(p.shows_result(), "on by default");
        p.compare = true;
        assert!(!p.shows_result());
        p.compare = false;
        p.before = true;
        assert!(!p.shows_result());
        p.before = false;
        p.on = false;
        assert!(!p.shows_result());
    }

    #[test]
    fn result_changed_reports_each_switch_once() {
        let mut p = Preview::default();
        assert_eq!(p.result_changed(), None);
        p.compare = true;
        assert_eq!(p.result_changed(), Some(false));
        assert_eq!(p.result_changed(), None);
        p.compare = false;
        assert_eq!(p.result_changed(), Some(true));
    }

    /// A document whose edits each record an undo step: the value before the edit.
    struct Doc {
        value: i32,
        undo: Vec<i32>,
    }

    impl Doc {
        fn set(&mut self, value: i32) {
            self.undo.push(std::mem::replace(&mut self.value, value));
        }
    }

    impl UndoMarks for Doc {
        type Mark = usize;

        fn undo_mark(&self) -> usize {
            self.undo.len()
        }

        fn revert_to(&mut self, mark: usize) {
            if self.undo.len() > mark {
                self.value = self.undo[mark];
                self.undo.truncate(mark);
            }
        }

        fn squash_since(&mut self, mark: usize) {
            self.undo.truncate((mark + 1).min(self.undo.len()));
        }
    }

    /// The edit preview described on [`Preview`]: updates apply the edited value, switching the preview
    /// off applies the original, and OK leaves one undo step.
    struct EditPreview {
        ctx: Context,
        host: GestureHost<Doc>,
        doc: Doc,
        outer: OuterGesture,
        preview: Preview,
        original: i32,
        edited: i32,
    }

    impl EditPreview {
        fn open(value: i32) -> Self {
            let ctx = Context::default();
            let mut host = GestureHost::new();
            let mut doc = Doc { value, undo: Vec::new() };
            let outer = OuterGesture::begin(&ctx);
            host.begin_outer(&ctx, &mut doc, outer);
            Self { ctx, host, doc, outer, preview: Preview::default(), original: value, edited: value }
        }

        fn update(&mut self, value: i32) {
            self.edited = value;
            let event = EditEvent::update(self.outer.id(), value, Modifiers::NONE);
            self.host.handle(&self.ctx, &mut self.doc, event, |doc, v| doc.set(v));
        }

        /// What the host does each pass after the preview flags may have changed.
        fn pass(&mut self) {
            if let Some(shows) = self.preview.result_changed() {
                let value = if shows { self.edited } else { self.original };
                self.host.handle(&self.ctx, &mut self.doc, EditEvent::update(self.outer.id(), value, Modifiers::NONE), |doc, v| doc.set(v));
            }
        }
    }

    #[test]
    fn an_edit_preview_toggles_the_original_and_ok_is_one_step() {
        let mut e = EditPreview::open(1);
        for v in [2, 3, 4] {
            e.update(v);
        }
        e.preview.compare = true;
        e.pass();
        assert_eq!(e.doc.value, 1, "the original while held");
        e.preview.compare = false;
        e.pass();
        assert_eq!(e.doc.value, 4, "the edit again");
        e.preview.on = false;
        e.pass();
        assert_eq!(e.doc.value, 1);
        e.preview.on = true;
        e.pass();
        assert!(e.host.commit(&mut e.doc, e.outer.id()));
        assert_eq!((e.doc.value, e.doc.undo.len()), (4, 1), "one step for every edit and toggle");
        assert_eq!(e.doc.undo, [1]);
    }

    #[test]
    fn cancelling_an_edit_preview_reverts_to_the_mark() {
        let mut e = EditPreview::open(1);
        e.update(5);
        e.preview.on = false;
        e.pass();
        e.update(6);
        assert!(e.host.cancel(&mut e.doc, e.outer.id()));
        assert_eq!((e.doc.value, e.doc.undo.len()), (1, 0));
    }
}
