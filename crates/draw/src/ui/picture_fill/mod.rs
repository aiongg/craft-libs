//! [`PictureFillEditor`]: PowerPoint's "Picture or texture fill" pane (DeckCraft): the picture's
//! source, picked through the host's files, then Stretch offsets or Tile offsets, scale,
//! alignment and mirror, Rotate with shape and Transparency.
//!
//! ```no_run
//! use craft_draw::ui::picture_fill::{PictureFill, PictureFillEditor, PictureFillEvent};
//! # fn demo(ui: &mut egui::Ui, fill: &PictureFill) {
//! for event in PictureFillEditor::new("picture", fill).show(ui).events {
//!     match event {
//!         PictureFillEvent::Source { name, bytes } => { /* the app stores the picture */ let _ = (name, bytes); }
//!         PictureFillEvent::Edit(edit) => { /* one undo step per gesture */ let _ = edit; }
//!         _ => {}
//!     }
//! }
//! # }
//! ```

mod editor;
mod labels;
pub(crate) mod stories;
#[cfg(test)]
mod tests;

pub use editor::{PictureFillEditor, PictureFillOutput};
pub use labels::PictureFillLabels;

/// Where a tiled picture's first tile sits in the shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TileAlignment {
    #[default]
    #[allow(missing_docs)]
    TopLeft,
    #[allow(missing_docs)]
    Top,
    #[allow(missing_docs)]
    TopRight,
    #[allow(missing_docs)]
    Left,
    #[allow(missing_docs)]
    Center,
    #[allow(missing_docs)]
    Right,
    #[allow(missing_docs)]
    BottomLeft,
    #[allow(missing_docs)]
    Bottom,
    #[allow(missing_docs)]
    BottomRight,
}

/// How tiles flip from one to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TileMirror {
    /// No mirroring.
    #[default]
    None,
    /// Every second tile in a row is flipped left to right.
    Horizontal,
    /// Every second tile in a column is flipped top to bottom.
    Vertical,
    /// Both.
    Both,
}

/// How the picture fills the shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PictureMode {
    /// Stretched to the shape, inset by `offsets` (left, top, right, bottom) in percent of the
    /// shape (negative grows past the edge).
    Stretch {
        /// Inset from the left, top, right and bottom edges, in percent of the shape.
        offsets: [f64; 4],
    },
    /// Tiled: `offset` (x, y) in points, `scale` (x, y) in percent, the first tile's alignment and
    /// the mirror between tiles.
    Tile {
        /// Offset of the tiling, x and y in points.
        offset: [f64; 2],
        /// Scale of a tile, x and y in percent.
        scale: [f64; 2],
        /// Where the first tile sits.
        alignment: TileAlignment,
        /// Mirroring between tiles.
        mirror: TileMirror,
    },
}

impl PictureMode {
    /// The smallest tile scale in percent: DeckCraft's smallest step (its picture-fill fields show
    /// one decimal), so a scale is never zero or negative. There is no upper limit.
    pub const SCALE: std::ops::RangeInclusive<f64> = 0.1..=f64::INFINITY;
}

impl Default for PictureMode {
    /// Stretched with no offsets.
    fn default() -> Self {
        PictureMode::Stretch { offsets: [0.0; 4] }
    }
}

/// A picture fill's values; the picture itself is the app's.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PictureFill {
    /// The picture's name as the source row shows it; `None` before one is picked.
    pub source: Option<String>,
    /// How the picture fills the shape.
    pub mode: PictureMode,
    /// The picture turns with the shape.
    pub rotate_with_shape: bool,
    /// 0 (opaque) to 1.
    pub transparency: f64,
}

impl PictureFill {
    /// A stretched fill of the picture `source`, turning with the shape, opaque.
    pub fn new(source: impl Into<String>) -> Self {
        PictureFill { source: Some(source.into()), mode: PictureMode::default(), rotate_with_shape: true, transparency: 0.0 }
    }
}

/// What the editor reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PictureFillEvent {
    /// A field changed: `Set` for a check box, a menu pick or a typed value; a gesture for a scrub
    /// or the Transparency slider.
    Edit(craft_ui::edit::EditEvent<PictureFill>),
    /// Insert… picked a picture and read it: its file name and bytes. The app stores the picture
    /// and sets [`PictureFill::source`].
    Source {
        /// The file name.
        name: String,
        /// The file's contents.
        bytes: Vec<u8>,
    },
    /// The picked file could not be read: the message to show.
    SourceFailed(String),
}
