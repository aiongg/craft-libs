//! The shape menu: the preset glyphs, their words and [`ShapeMenu`].

use crate::geometry::presets::ShapePreset;
use craft_ui::primitives::Glyph;
use craft_ui::runtime::env::Label;
use craft_ui::style::UiTokens;
use egui::{Color32, Painter, Rect, Ui};

craft_ui::words! {
    /// The labels of the shape presets, English by default (upstream's).
    pub struct ShapePresetWords {
        /// The Rectangle preset's label.
        rectangle: "shapePreset.rectangle" = "Rectangle";
        /// The Rounded Rectangle preset's label.
        rounded_rectangle: "shapePreset.roundedRectangle" = "Rounded Rectangle";
        /// The Oval preset's label.
        ellipse: "shapePreset.ellipse" = "Oval";
        /// The Triangle preset's label.
        triangle: "shapePreset.triangle" = "Triangle";
        /// The Diamond preset's label.
        diamond: "shapePreset.diamond" = "Diamond";
        /// The Line preset's label.
        line: "shapePreset.line" = "Line";
        /// The Arrow preset's label.
        arrow: "shapePreset.arrow" = "Arrow";
        /// The Star preset's label.
        star: "shapePreset.star" = "Star";
        /// The Heart preset's label.
        heart: "shapePreset.heart" = "Heart";
        /// The Text Box preset's label.
        text_box: "shapePreset.textBox" = "Text Box";
        /// The Straight Connector preset's label.
        straight_connector: "shapePreset.straightConnector" = "Straight Connector";
        /// The Straight Arrow Connector preset's label.
        straight_arrow_connector: "shapePreset.straightArrowConnector" = "Straight Arrow Connector";
        /// The Elbow Connector preset's label.
        elbow_connector: "shapePreset.elbowConnector" = "Elbow Connector";
        /// The Elbow Arrow Connector preset's label.
        elbow_arrow_connector: "shapePreset.elbowArrowConnector" = "Elbow Arrow Connector";
        /// The Curved Connector preset's label.
        curved_connector: "shapePreset.curvedConnector" = "Curved Connector";
        /// The Curved Arrow Connector preset's label.
        curved_arrow_connector: "shapePreset.curvedArrowConnector" = "Curved Arrow Connector";
    }
}

impl ShapePreset {
    /// The preset's label in `words`.
    pub fn label(self, words: &ShapePresetWords) -> &Label {
        match self {
            ShapePreset::Rectangle => &words.rectangle,
            ShapePreset::RoundedRectangle => &words.rounded_rectangle,
            ShapePreset::Ellipse => &words.ellipse,
            ShapePreset::Triangle => &words.triangle,
            ShapePreset::Diamond => &words.diamond,
            ShapePreset::Line => &words.line,
            ShapePreset::Arrow => &words.arrow,
            ShapePreset::Star => &words.star,
            ShapePreset::Heart => &words.heart,
            ShapePreset::TextBox => &words.text_box,
            ShapePreset::StraightConnector => &words.straight_connector,
            ShapePreset::StraightArrowConnector => &words.straight_arrow_connector,
            ShapePreset::ElbowConnector => &words.elbow_connector,
            ShapePreset::ElbowArrowConnector => &words.elbow_arrow_connector,
            ShapePreset::CurvedConnector => &words.curved_connector,
            ShapePreset::CurvedArrowConnector => &words.curved_arrow_connector,
            #[allow(unreachable_patterns)]
            _ => &words.rectangle,
        }
    }
}

impl Glyph for ShapePreset {
    fn paint(&self, painter: &Painter, tokens: &UiTokens, rect: Rect, color: Color32, mirrored: bool) {
        let _ = (painter, tokens, rect, color, mirrored);
        todo!("fx.F57")
    }
}

/// The shape menu: the supported presets as menu items with the preset glyph as icon, a separator before the first
/// connector.
#[must_use = "show the menu with `show`"]
#[derive(craft_ui::contract::Pattern)]
pub struct ShapeMenu<'a> {
    supported: Box<dyn Fn(ShapePreset) -> bool + 'a>,
}

impl<'a> ShapeMenu<'a> {
    /// The menu of the presets `supported` accepts.
    pub fn new(supported: impl Fn(ShapePreset) -> bool + 'a) -> Self {
        ShapeMenu { supported: Box::new(supported) }
    }

    /// Shows the menu and reports the picked preset.
    pub fn show(self, ui: &mut Ui) -> Option<ShapePreset> {
        let ShapeMenu { supported } = self;
        let _ = (ui, supported);
        todo!("fx.F57")
    }
}
