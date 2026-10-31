//! Markers on a ruler or a clip bar, and the Marker dialog (N1.12).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use egui::{AsIdSalt, Context, IdSalt, Rect, Response, Ui};

use super::LabelPalette;
use crate::clips::SnapTargets;
use crate::markers::{Marker, MarkerColor, MarkerEdit, MarkerId};
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;

/// A row of markers in `rect` (the ruler's marker band, a clip or layer bar): a flag 8 points
/// wide pointing down at its time (the point at 60 % of the height), coloured by its label, a
/// duration bar for range markers, the name beside it when there is room.
///
/// After Effects' gestures (EffectCraft `markers_ui.rs:1`): drag moves (snapping to `snap`),
/// Alt-drag sets the duration, Cmd-click (Ctrl-click on Windows and Linux) deletes, a
/// double-click opens the dialog, a secondary click is reported for the app's menu (Settings,
/// Convert, Delete). With [`bin`](Self::bin) a marker dragged out of the bin at the right end
/// adds one where it is dropped. Tooltips show name and comment. Keyboard: the strip is one Tab
/// stop, Left / Right pick the previous / next marker, Enter opens it, Delete deletes it.
pub struct MarkerStrip<'a> {
    id_salt: IdSalt,
    markers: &'a [Marker],
    view: &'a TimeView,
    snap: Option<&'a SnapTargets>,
    palette: Option<&'a LabelPalette>,
    bin: Option<Rect>,
}

impl<'a> MarkerStrip<'a> {
    pub fn new(id_salt: impl AsIdSalt, markers: &'a [Marker], view: &'a TimeView) -> Self {
        Self { id_salt: IdSalt::new(id_salt), markers, view, snap: None, palette: None, bin: None }
    }

    pub fn snap(mut self, targets: Option<&'a SnapTargets>) -> Self {
        self.snap = targets;
        self
    }

    pub fn palette(mut self, palette: &'a LabelPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// The marker bin (After Effects' at the right end of the ruler, EffectCraft `:429`).
    pub fn bin(mut self, rect: Rect) -> Self {
        self.bin = Some(rect);
        self
    }

    // effectcraft: crates/ui-egui/src/panels/markers_ui.rs:274 `marker_shape`, `:282` `marker`,
    // `:253` `snap`, `:380` `name_box_x`, `:408` `comp_markers`, `:429` `marker_bin`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> MarkerStripResponse {
        todo!()
    }
}

/// What the strip did this pass. Drags report their final `Move` / `SetDuration` on release;
/// while dragging, `dragging` gives the live value for the preview.
pub struct MarkerStripResponse {
    pub response: Response,
    pub edits: Vec<MarkerEdit>,
    pub dragging: Option<(MarkerId, Tick)>,
    pub context_menu: Option<MarkerId>,
}

craft_ui::words! {
    /// The Marker dialog's words, English by default.
    #[non_exhaustive]
    pub struct MarkerDialogLabels {
        title: "markerDialog.title" = "Marker";
        name: "markerDialog.name" = "Name:";
        comment: "markerDialog.comment" = "Comments:";
        time: "markerDialog.time" = "Time:";
        duration: "markerDialog.duration" = "Duration:";
        color: "markerDialog.color" = "Marker Color:";
        delete: "markerDialog.delete" = "Delete";
        previous: "markerDialog.previous" = "Previous";
        next: "markerDialog.next" = "Next";
    }
}

/// The marker as the dialog edits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerDraft {
    pub name: String,
    pub comment: String,
    pub start: Tick,
    pub duration: Tick,
    pub color: MarkerColor,
}

/// Premiere's Marker dialog on the dialog host: Name, Time and Duration (timecode fields),
/// Comments (multi-line), Marker Color (the label swatches), Delete, and OK / Cancel. Enter in a
/// single-line field is OK; Tab walks the fields. Result: the edited [`MarkerDraft`] on OK, or
/// `None` in it when Delete was pressed.
pub struct MarkerDialog {
    id: MarkerId,
    draft: MarkerDraft,
    format: TimeFormat,
    palette: LabelPalette,
    deleted: bool,
    labels: Option<MarkerDialogLabels>,
}

impl MarkerDialog {
    pub fn new(marker: &Marker, format: TimeFormat) -> Self {
        let draft = MarkerDraft {
            name: marker.name.clone(),
            comment: marker.comment.clone(),
            start: marker.start,
            duration: marker.duration,
            color: marker.color,
        };
        Self { id: marker.id, draft, format, palette: LabelPalette::default(), deleted: false, labels: None }
    }

    pub fn palette(mut self, palette: LabelPalette) -> Self {
        self.palette = palette;
        self
    }

    pub fn labels(mut self, labels: MarkerDialogLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl StackDialog for MarkerDialog {
    /// The marker and its new values, `None` for Delete.
    type Result = (MarkerId, Option<MarkerDraft>);

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("timeline-marker")
    }

    // effectcraft: crates/ui-egui/src/panels/markers_ui.rs:124 `dialog`, `:47` `MarkerDraft`;
    // layout as Premiere's Marker dialog, on `Dialog` + `FormGrid` + `Footer`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Self::Result>> {
        todo!()
    }
}
