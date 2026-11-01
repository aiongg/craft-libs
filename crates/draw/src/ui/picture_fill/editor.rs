//! [`PictureFillEditor`] (task SH-T13).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Ui};

use super::{PictureFill, PictureFillEvent, PictureFillLabels};

/// What [`PictureFillEditor::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app applies the edits and stores a picked picture"]
pub struct PictureFillOutput {
    pub events: Vec<PictureFillEvent>,
}

impl PictureFillOutput {
    /// Whether anything changed this pass (an edit or a picked picture): a paint chip's Picture
    /// tab redraws its preview.
    pub fn changed(&self) -> bool {
        !self.events.is_empty()
    }
}

/// PowerPoint's Picture or texture fill pane, in its order:
///
/// 1. "Picture source" with the source's name and "Insert…": picks an image through the
///    installed `Host`'s `Files` (purpose `"picture"`, image types) and reads it as a host job, as
///    `LutPicker` does; reports [`PictureFillEvent::Source`] on the pass it is read, or
///    `SourceFailed`. A cancelled pick reports nothing; without a `Host`, Insert… is disabled.
/// 2. "Tile picture as texture" check box (switches [`PictureMode`](super::PictureMode), keeping
///    each mode's last values for this editor).
/// 3. Tile: Offset X and Offset Y (points), Scale X and Scale Y (percent), Alignment (9 values),
///    Mirror type (None, Horizontal, Vertical, Both). Stretch: Offset left, right, top, bottom
///    (percent).
/// 4. "Rotate with shape" check box.
/// 5. "Transparency" slider and field, 0–100 %.
///
/// Every field reports [`PictureFillEvent::Edit`]. Keyboard: Tab through the fields in this order;
/// the grid mirrors right to left.
pub struct PictureFillEditor<'a> {
    id_salt: IdSalt,
    fill: &'a PictureFill,
    labels: Option<PictureFillLabels>,
}

impl<'a> PictureFillEditor<'a> {
    /// The editor of `fill`.
    pub fn new(id_salt: impl AsIdSalt, fill: &'a PictureFill) -> Self {
        PictureFillEditor { id_salt: IdSalt::new(id_salt), fill, labels: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: PictureFillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> PictureFillOutput {
        // inv-deckcraft: crates/ui-egui/src/panels/format_shape.rs:622-686 `picture_fill_options`
        // (source and Insert… :625, the tile check box, the Tile fields, Alignment, Mirror type, the
        // Stretch offsets :668, Rotate with shape :683), :405-416 (Transparency of a picture fill),
        // :600-605 (Insert… picks "picture" and reads it).
        // craft-color: crates/color/src/ui/lut_picker.rs:110 `LutPicker::show` (the file pick through
        // `Host::try_of` and a job; lead call 25).
        todo!()
    }
}
