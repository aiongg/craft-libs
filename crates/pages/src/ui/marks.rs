//! [`TextFrameMarks`] (N3.17): InDesign's View › Show Hidden Characters and Extras over text
//! frames: · spaces, » tabs, ¬ forced line breaks, ° no-break spaces, ¶ paragraph ends, # the end
//! of a story; the overset mark (a red + in the out port) on a frame whose story doesn't fit;
//! frame outlines with their in and out ports.

use egui::epaint::PathShape;
use egui::{Align2, Color32, FontId, Painter, Pos2, Shape, Stroke, vec2};
use kurbo::{Point, Rect};

use craft_ui::canvas::CanvasView;
use craft_ui::theme::{StrokeExt as _, current, scale};

use super::style::FrameMarksStyle;

/// A hidden-character mark's size as a share of the font's ascent (DesignCraft): a proportion
/// (lead call 38).
pub const MARK_SHARE: f32 = 0.75;

/// The threaded triangle's corners as shares of the port's side from its centre: the two back
/// corners and the tip (DesignCraft draws -2, +2.5 and -2 on a 8.5 port).
const TRIANGLE: [(f32, f32); 3] = [(-0.235, -0.295), (0.295, 0.0), (-0.235, 0.295)];

/// A hidden-character mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HiddenMark {
    /// · (centred on the space).
    Space,
    /// ».
    Tab,
    /// ¬.
    LineBreak,
    /// °.
    NoBreakSpace,
    /// ¶.
    ParagraphEnd,
    /// #.
    StoryEnd,
}

impl HiddenMark {
    pub fn glyph(self) -> &'static str {
        match self {
            HiddenMark::Space => "·",
            HiddenMark::Tab => "»",
            HiddenMark::LineBreak => "¬",
            HiddenMark::NoBreakSpace => "°",
            HiddenMark::ParagraphEnd => "¶",
            HiddenMark::StoryEnd => "#",
        }
    }

    /// The mark for a character, if it has one (the paragraph and story ends come from the
    /// layout, not a character).
    // designcraft: crates/ui-egui/src/canvas/text_overlay.rs:315 `hidden_marks`.
    pub fn of(c: char) -> Option<HiddenMark> {
        match c {
            ' ' => Some(HiddenMark::Space),
            '\t' => Some(HiddenMark::Tab),
            '\u{2028}' => Some(HiddenMark::LineBreak),
            '\u{A0}' => Some(HiddenMark::NoBreakSpace),
            _ => None,
        }
    }
}

/// One mark to draw, frame points: where it sits on the baseline, whether it is centred on that
/// x (spaces) or starts at it, and the line's ascent (its size follows the text's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedMark {
    pub mark: HiddenMark,
    pub at: Point,
    pub centred: bool,
    pub ascent: f64,
}

/// One text frame's extras: its outline, ports and whether its story is overset there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameExtras {
    /// The frame's inner bounds on the document (points).
    pub bounds: Rect,
    /// The frame continues a story from another frame (in port filled with a triangle).
    pub threaded_in: bool,
    /// The story continues into another frame (out port filled with a triangle).
    pub threaded_out: bool,
    /// The frame ends a story that doesn't fit: the out port shows the overset +.
    pub overset: bool,
}

/// The marks and extras of the frames on screen, painted over the page after its content.
///
/// - Marks are drawn in the frame's layer colour, at `ascent ×` [`MARK_SHARE`] `× zoom`, kept
///   between the style's `mark_min` and `mark_max` (DesignCraft).
/// - Ports are squares of the style's `port` side, white with a hairline in the layer colour:
///   the in port `port_inset` right of the top-left corner on the top edge, the out port
///   `port_inset` left of the bottom-right corner on the bottom edge (DesignCraft's `port_pos`,
///   `PORT_SIZE_PX`). A threaded port holds a small triangle in the layer colour; the overset port
///   the style's overset `+`, drawn on every frame that ends an overset story, not only selected
///   ones. Sizes from [`FrameMarksStyle`](super::FrameMarksStyle).
/// - Nothing here takes input or focus: it's paint only.
pub struct TextFrameMarks<'a> {
    view: &'a CanvasView,
    marks: &'a [PlacedMark],
    frames: &'a [FrameExtras],
    hidden_characters: bool,
    color: egui::Color32,
}

impl<'a> TextFrameMarks<'a> {
    pub fn new(view: &'a CanvasView, frames: &'a [FrameExtras]) -> Self {
        TextFrameMarks { view, marks: &[], frames, hidden_characters: false, color: egui::Color32::LIGHT_BLUE }
    }

    /// The hidden-character marks (View › Show Hidden Characters).
    pub fn hidden_characters(mut self, marks: &'a [PlacedMark]) -> Self {
        self.marks = marks;
        self.hidden_characters = true;
        self
    }

    /// The layer colour the marks and outlines take.
    pub fn color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    // designcraft: crates/ui-egui/src/canvas/text_overlay.rs:271 `draw_hidden_characters`;
    // crates/ui-egui/src/canvas/frames.rs:349 `port_pos`, :292 (overset marks).
    pub fn paint(self, painter: &Painter) {
        let style = FrameMarksStyle::of(&current(painter.ctx()));
        let line = |color| scale::Stroke::Hairline.egui(painter.ctx(), color);
        if self.hidden_characters {
            for placed in self.marks {
                let size = (placed.ascent as f32 * MARK_SHARE * self.view.zoom as f32).clamp(style.mark_min, style.mark_max);
                let align = if placed.centred { Align2::CENTER_BOTTOM } else { Align2::LEFT_BOTTOM };
                painter.text(self.view.to_screen(placed.at), align, placed.mark.glyph(), FontId::proportional(size), self.color);
            }
        }
        for frame in self.frames {
            let corners = self.view.quad(frame.bounds);
            painter.add(Shape::Path(PathShape::closed_line(corners.to_vec(), line(self.color))));
            let in_port = port_centre(corners[0], corners[1], style.port_inset);
            let out_port = port_centre(corners[2], corners[3], style.port_inset);
            for (centre, threaded, is_out) in [(in_port, frame.threaded_in, false), (out_port, frame.threaded_out, true)] {
                let square = egui::Rect::from_center_size(centre, vec2(style.port, style.port));
                painter.rect_filled(square, 0.0, Color32::WHITE);
                painter.rect_stroke(square, 0.0, line(self.color), egui::StrokeKind::Inside);
                if is_out && frame.overset {
                    let arm = style.overset_arm;
                    let stroke = Stroke::new(style.overset_stroke, style.overset);
                    painter.line_segment([centre - vec2(arm, 0.0), centre + vec2(arm, 0.0)], stroke);
                    painter.line_segment([centre - vec2(0.0, arm), centre + vec2(0.0, arm)], stroke);
                } else if threaded {
                    let points = TRIANGLE.iter().map(|&(x, y)| centre + vec2(x, y) * style.port).collect();
                    painter.add(Shape::convex_polygon(points, self.color, Stroke::NONE));
                }
            }
        }
    }
}

/// The port's centre on the edge from `corner` toward `other`: `inset` along it, never past the
/// edge's middle on a small frame.
fn port_centre(corner: Pos2, other: Pos2, inset: f32) -> Pos2 {
    let edge = other - corner;
    let length = edge.length();
    if length > f32::EPSILON { corner + edge * (inset.min(length / 2.0) / length) } else { corner }
}
