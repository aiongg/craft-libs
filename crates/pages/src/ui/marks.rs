//! [`TextFrameMarks`] (N3.17): InDesign's View › Show Hidden Characters and Extras over text
//! frames: · spaces, » tabs, ¬ forced line breaks, ° no-break spaces, ¶ paragraph ends, # the end
//! of a story; the overset mark (a red + in the out port) on a frame whose story doesn't fit;
//! frame outlines with their in and out ports.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Painter;
use kurbo::{Point, Rect};

use craft_ui::canvas::CanvasView;

/// A hidden-character mark's size as a share of the font's ascent (DesignCraft): a proportion
/// (lead call 38).
pub const MARK_SHARE: f32 = 0.75;

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
        todo!()
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
        todo!()
    }
}
