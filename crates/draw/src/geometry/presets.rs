//! [`ShapePreset`]: the kinds of the shape menu, with the geometry or outline each draws.
//!
//! Upstream's list, order and keys. Kind-backed presets draw through [`ShapeGeometry::to_path`]
//! and connectors through [`connector::route`](super::connector::route), so no preset keeps a
//! second outline of a shape the crate already draws.

use super::connector::Routing;
use super::shape::ShapeGeometry;

/// One entry of the shape menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShapePreset {
    /// A rectangle.
    Rectangle,
    /// A rectangle with rounded corners.
    RoundedRectangle,
    /// An ellipse.
    Ellipse,
    /// A triangle.
    Triangle,
    /// A diamond.
    Diamond,
    /// A line.
    Line,
    /// A line with an arrowhead.
    Arrow,
    /// A star.
    Star,
    /// A heart.
    Heart,
    /// A text box.
    TextBox,
    /// A straight connector.
    StraightConnector,
    /// A straight connector with an end arrow.
    StraightArrowConnector,
    /// An elbow connector.
    ElbowConnector,
    /// An elbow connector with an end arrow.
    ElbowArrowConnector,
    /// A curved connector.
    CurvedConnector,
    /// A curved connector with an end arrow.
    CurvedArrowConnector,
}

/// Every preset, in menu order.
pub const PRESETS: &[ShapePreset] = &[
    ShapePreset::Rectangle,
    ShapePreset::RoundedRectangle,
    ShapePreset::Ellipse,
    ShapePreset::Triangle,
    ShapePreset::Diamond,
    ShapePreset::Line,
    ShapePreset::Arrow,
    ShapePreset::Star,
    ShapePreset::Heart,
    ShapePreset::TextBox,
    ShapePreset::StraightConnector,
    ShapePreset::StraightArrowConnector,
    ShapePreset::ElbowConnector,
    ShapePreset::ElbowArrowConnector,
    ShapePreset::CurvedConnector,
    ShapePreset::CurvedArrowConnector,
];

/// What a preset draws in a rectangle: a fill path and stroke paths.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct PresetOutline {
    /// The path to fill, when the preset has an inside.
    pub fill: Option<kurbo::BezPath>,
    /// The paths to stroke.
    pub strokes: Vec<kurbo::BezPath>,
}

impl ShapePreset {
    /// The preset's stable key (upstream's).
    pub const fn key(self) -> &'static str {
        match self {
            ShapePreset::Rectangle => "rectangle",
            ShapePreset::RoundedRectangle => "roundedRectangle",
            ShapePreset::Ellipse => "ellipse",
            ShapePreset::Triangle => "triangle",
            ShapePreset::Diamond => "diamond",
            ShapePreset::Line => "line",
            ShapePreset::Arrow => "arrow",
            ShapePreset::Star => "star",
            ShapePreset::Heart => "heart",
            ShapePreset::TextBox => "textBox",
            ShapePreset::StraightConnector => "straightConnector",
            ShapePreset::StraightArrowConnector => "straightArrowConnector",
            ShapePreset::ElbowConnector => "elbowConnector",
            ShapePreset::ElbowArrowConnector => "elbowArrowConnector",
            ShapePreset::CurvedConnector => "curvedConnector",
            ShapePreset::CurvedArrowConnector => "curvedArrowConnector",
        }
    }

    /// Whether the preset is a connector.
    pub const fn is_connector(self) -> bool {
        self.routing().is_some()
    }

    /// Whether the preset draws an arrowhead at its end.
    pub const fn has_end_arrow(self) -> bool {
        matches!(
            self,
            ShapePreset::Arrow | ShapePreset::StraightArrowConnector | ShapePreset::ElbowArrowConnector | ShapePreset::CurvedArrowConnector
        )
    }

    /// The kind-backed geometry in `rect`; `None` for the other presets.
    pub fn geometry(self, rect: kurbo::Rect) -> Option<ShapeGeometry> {
        let _ = rect;
        let _ = self;
        todo!("fx.F56")
    }

    /// How a connector preset routes; `None` for the others.
    pub const fn routing(self) -> Option<Routing> {
        match self {
            ShapePreset::StraightConnector | ShapePreset::StraightArrowConnector => Some(Routing::Straight),
            ShapePreset::ElbowConnector | ShapePreset::ElbowArrowConnector => Some(Routing::Elbow),
            ShapePreset::CurvedConnector | ShapePreset::CurvedArrowConnector => Some(Routing::Curved),
            _ => None,
        }
    }

    /// The paths the preset draws in `rect`.
    pub fn outline(self, rect: kurbo::Rect) -> PresetOutline {
        let _ = (self, rect);
        todo!("fx.F56")
    }
}
