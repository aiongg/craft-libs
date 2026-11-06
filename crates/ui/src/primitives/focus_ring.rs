//! `FocusRing`: rings and halos around a rect, circle, polygon or path.

use crate::style::{Flags, FocusLook};
use egui::emath::GuiRounding as _;
use egui::{CornerRadius, Pos2, Rect, Shape, Stroke, StrokeKind};

/// The shape a ring follows.
// F2-MERGE: F2d
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FocusShape {
    /// A rect with corners.
    Rect(Rect, CornerRadius),
    /// A circle.
    Circle(Pos2, f32),
    /// A closed polygon.
    Polygon(Vec<Pos2>),
    /// An open path.
    Path(Vec<Pos2>),
}

/// How a focus indication is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FocusKind {
    /// A ring on the ring sublayer.
    #[default]
    Ring,
    /// A halo: a wide translucent stroke under the ring (canvas parts).
    Halo,
}

/// The focus ring painter.
pub struct FocusRing;

impl FocusRing {
    /// Whether a part with `flags` shows its ring: only when it is the focused component or its active part, with
    /// focus visible.
    pub fn visible(flags: Flags) -> bool {
        flags.contains(Flags::FOCUSED | Flags::FOCUS_VISIBLE)
    }

    /// The shapes of the ring around `shape`, grown by `look.offset`. A halo adds a stroke twice as wide at a
    /// third of the opacity beneath the ring.
    pub fn shapes(look: &FocusLook, kind: FocusKind, shape: &FocusShape, pixels_per_point: f32) -> Vec<Shape> {
        let stroke = look.stroke;
        let mut out = Vec::new();
        if kind == FocusKind::Halo {
            out.push(Self::one(shape, look.offset, Stroke::new(stroke.width * 2.0, stroke.color.gamma_multiply(1.0 / 3.0)), pixels_per_point));
        }
        out.push(Self::one(shape, look.offset, stroke, pixels_per_point));
        out
    }

    fn one(shape: &FocusShape, offset: f32, stroke: Stroke, pixels_per_point: f32) -> Shape {
        match shape {
            FocusShape::Rect(rect, radius) => {
                let rect = rect.round_to_pixels(pixels_per_point);
                let grown = rect.expand(offset);
                let radius = CornerRadius {
                    nw: radius.nw.saturating_add(offset.max(0.0) as u8),
                    ne: radius.ne.saturating_add(offset.max(0.0) as u8),
                    sw: radius.sw.saturating_add(offset.max(0.0) as u8),
                    se: radius.se.saturating_add(offset.max(0.0) as u8),
                };
                Shape::rect_stroke(grown, radius, stroke, StrokeKind::Inside)
            }
            FocusShape::Circle(center, r) => Shape::circle_stroke(*center, r + offset, stroke),
            FocusShape::Polygon(points) => Shape::closed_line(points.clone(), stroke),
            FocusShape::Path(points) => Shape::line(points.clone(), stroke),
        }
    }
}
