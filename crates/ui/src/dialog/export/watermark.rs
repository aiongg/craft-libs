//! [`WatermarkEditor`]: the Watermark group of an export dialog, after Lightroom's watermark
//! editor and LightCraft's port of it.

use egui::Ui;
use serde::{Deserialize, Serialize};

use super::super::sizing::Anchor;
use crate::edit::EditResponse;

crate::words! {
    /// The watermark group's words, English by default.
    #[non_exhaustive]
    pub struct WatermarkLabels {
        style: "watermark.style" = "Style";
        text_style: "watermark.textStyle" = "Text";
        graphic_style: "watermark.graphicStyle" = "Graphic";
        text: "watermark.text" = "Text";
        image: "watermark.image" = "Image";
        choose: "watermark.choose" = "Choose…";
        orientation: "watermark.orientation" = "Orientation";
        horizontal: "watermark.horizontal" = "Horizontal";
        vertical: "watermark.vertical" = "Vertical";
        position: "watermark.position" = "Position";
        rotation: "watermark.rotation" = "Rotation";
        size: "watermark.size" = "Size";
        opacity: "watermark.opacity" = "Opacity";
        inset_x: "watermark.insetX" = "Inset X";
        inset_y: "watermark.insetY" = "Inset Y";
        shadow: "watermark.shadow" = "Shadow";
    }
}

/// Text or an image.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WatermarkStyle {
    Text(String),
    /// The image file's path.
    Graphic(String),
}

/// A watermark. Sizes are percent of the image's short edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Watermark {
    pub style: WatermarkStyle,
    pub vertical: bool,
    /// One of the nine anchors.
    pub position: Anchor,
    /// 0, 90, 180 or 270 degrees.
    pub rotation: u16,
    /// 1–15 %.
    pub size: f32,
    /// 5–100 %.
    pub opacity: f32,
    /// 0–40 % each.
    pub inset: [f32; 2],
    pub shadow: bool,
}

impl Default for Watermark {
    /// LightCraft's defaults: size 3.5 %, opacity 70 %, inset 2.5 % on both axes.
    // lightcraft: crates/engine/src/export/watermark.rs:55 Watermark::default
    fn default() -> Self {
        todo!()
    }
}

/// The watermark rows inside a form: Style (Text, Graphic), the text field or the image path
/// with Choose… (a picker through `host.files()`, purpose `"watermark"`), Orientation, Position
/// as the 3×3 [`AnchorGrid`](crate::dialog::AnchorGrid), Rotation, Size, Opacity, Inset X and Y as
/// slider rows, Shadow. The preview stays the dialog's: LightCraft draws it on a job.
///
/// ```ignore
/// let r = WatermarkEditor::new("export-watermark", &mut settings.watermark).show(ui);
/// if r.changed() { preview.request(); }
/// ```
pub struct WatermarkEditor<'a> {
    id_salt: egui::IdSalt,
    value: &'a mut Watermark,
}

impl<'a> WatermarkEditor<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, value: &'a mut Watermark) -> Self {
        Self { id_salt: egui::IdSalt::new(id_salt), value }
    }

    // lightcraft: crates/ui-egui/src/dialogs.rs:1290 export_watermark
    pub fn show(self, ui: &mut Ui) -> EditResponse<Watermark> {
        todo!()
    }
}
