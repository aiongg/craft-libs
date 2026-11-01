//! Markers on a ruler or a clip bar, and the Marker dialog (N1.12).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use egui::{AsIdSalt, Context, IdSalt, Rect, Response, Ui};

use super::LabelPalette;
use crate::clips::SnapTargets;
use craft_ui::edit::EditEvent;

use crate::markers::{Marker, MarkerColor, MarkerId};
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;

/// What a marker gesture asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkerEdit {
    /// A drag of the marker: its new time (snapped), as one gesture; Esc cancels.
    Move { id: MarkerId, to: EditEvent<Tick> },
    /// An Alt-drag: its new duration, as one gesture.
    SetDuration { id: MarkerId, duration: EditEvent<Tick> },
    /// Cmd-click (Ctrl-click on Windows and Linux), or Delete with the marker picked.
    Delete(MarkerId),
    /// Double-click or Enter: open the Marker dialog on it.
    Open(MarkerId),
    /// A marker dragged out of the marker bin and dropped at `at`, or Add Marker (M).
    Add { at: Tick },
}

/// A row of markers in `rect` (the ruler's marker band, a clip or layer bar): a flag
/// `style.marker_width` wide ([`TimelineStyle`](super::TimelineStyle)) pointing down at its time
/// (the point at [`MARKER_POINT`] of the height), coloured by its label, a duration bar for
/// range markers, the name beside it when there is room.
///
/// After Effects' gestures (EffectCraft `markers_ui.rs:1`): drag moves (snapping to `snap`),
/// Alt-drag sets the duration, Cmd-click (Ctrl-click on Windows and Linux) deletes, a
/// double-click opens the dialog, a secondary click is reported for the app's menu (Settings,
/// Convert, Delete). With [`bin`](Self::bin) a marker dragged out of the bin at the right end
/// adds one where it is dropped. Tooltips show name and comment. Keyboard: the strip is one Tab
/// stop, Left / Right pick the previous / next marker, Enter opens it, Delete deletes it
/// (claimed through [`KeyClaim`](craft_ui::behavior::KeyClaim)). AccessKit: a `ListBox` of
/// markers named by name and time.
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

/// Where a marker's point sits, as a share of the strip's height (EffectCraft `markers_ui.rs:274`).
/// A proportion of the strip, not a size.
pub const MARKER_POINT: f32 = 0.6;

/// What the strip did this pass. A drag reports its gesture in [`MarkerEdit::Move`] and
/// [`MarkerEdit::SetDuration`]: the app previews `Update`s and applies the `Commit`.
pub struct MarkerStripResponse {
    pub response: Response,
    /// This pass's edits, in order.
    pub edits: Vec<MarkerEdit>,
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

/// What the Marker dialog answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerDialogResult {
    pub marker: MarkerId,
    pub action: MarkerDialogAction,
}

/// OK with the edited values, Delete, or Previous / Next (Premiere's: the edits are kept and the
/// app opens the dialog again on the neighbouring marker).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkerDialogAction {
    Save(MarkerDraft),
    Delete,
    SaveAndPrevious(MarkerDraft),
    SaveAndNext(MarkerDraft),
}

/// Premiere's Marker dialog on the dialog host: Name, Time and Duration (timecode fields),
/// Comments (multi-line), Marker Color (the label swatches), Delete, and OK / Cancel. Enter in a
/// single-line field is OK; Tab walks the fields (the dialog's field Tab, decisions §4.1).
/// Result: a [`MarkerDialogResult`].
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
    type Result = MarkerDialogResult;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("timeline-marker")
    }

    // effectcraft: crates/ui-egui/src/panels/markers_ui.rs:124 `dialog`, `:47` `MarkerDraft`;
    // layout as Premiere's Marker dialog, on `Dialog` + `FormGrid` + `Footer`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Self::Result>> {
        todo!()
    }
}
