//! The annotation tool on the canvas, the overlay that draws annotations, and the toolbar
//! (N5.03).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPointer};
use craft_ui::edit::EditEvent;
use egui::{AsIdSalt, IdSalt, Painter, Response, Ui};

use crate::annotate::{Annotation, AnnotationEdit, AnnotationId, AnnotationTool};

craft_ui::words! {
    /// The toolbar's group names and the tools' tooltips, English by default (PdfCraft `tools/comments.rs:148` `label`).
    #[non_exhaustive]
    pub struct AnnotateLabels {
        comment_group: "annotate.commentGroup" = "Comment";
        highlight_group: "annotate.highlightGroup" = "Highlight";
        draw_group: "annotate.drawGroup" = "Draw";
        select: "annotate.select" = "Select";
        note: "annotate.note" = "Add a sticky note";
        text_box: "annotate.textBox" = "Add a text box";
        callout: "annotate.callout" = "Add a callout";
        attach: "annotate.attach" = "Attach file";
        highlight: "annotate.highlight" = "Highlight";
        underline: "annotate.underline" = "Underline";
        strike_out: "annotate.strikeOut" = "Strikethrough";
        squiggly: "annotate.squiggly" = "Squiggly underline";
        caret: "annotate.caret" = "Insert text";
        replace_text: "annotate.replaceText" = "Replace text";
        ink: "annotate.ink" = "Draw";
        line: "annotate.line" = "Line";
        arrow: "annotate.arrow" = "Arrow";
        rectangle: "annotate.rectangle" = "Rectangle";
        oval: "annotate.oval" = "Oval";
        poly_line: "annotate.polyLine" = "Connected lines";
        polygon: "annotate.polygon" = "Polygon";
        cloud: "annotate.cloud" = "Cloud";
        eraser: "annotate.eraser" = "Eraser";
    }
}

/// The page text between two page points as quads (four corners each).
pub type TextQuads = Box<dyn Fn(kurbo::Point, kurbo::Point) -> Vec<[kurbo::Point; 4]>>;

/// Acrobat's annotation tools as one canvas tool on craft-ui's tool host.
///
/// - Select: a press on an annotation selects it (Shift adds), a drag moves the selection, a
///   drag on a handle resizes (box kinds; Shift keeps the aspect on a corner) or reshapes
///   (pointwise kinds; Shift snaps to 45°); a press on empty page starts a marquee. Moves,
///   resizes and reshapes are `EditEvent<AnnotationEdit>` gestures; Esc cancels and the release
///   after it is ignored.
/// - A drawing tool: a drag draws the kind (Shift: squares, circles, 45° lines); PolyLine and
///   Polygon add a point per click and end with a double-click or Enter; Ink records strokes until
///   the tool changes; Note and Text box place at the click and report `Create`, the app opens
///   the composer. Markup kinds select page text through the app's text hit test
///   ([`text_quads`](Self::text_quads)) and report the quads.
/// - Eraser: a drag reports `Erase` for the Ink strokes it crosses.
/// - Keys: Delete removes the selection, arrows nudge it through
///   [`behavior::nudge`](craft_ui::behavior::nudge) (one point, Shift ten) as `Set`s.
/// - A double-click (from [`ToolPointer::clicks`](craft_ui::canvas::ToolPointer)) on a note or
///   text box opens its text for editing; the app's page text and selection reach the tool in
///   [`CanvasTool::each_pass`](craft_ui::canvas::CanvasTool::each_pass) through the public fields.
/// - Locked annotations are selected but never moved.
pub struct AnnotateTool {
    pub tool: AnnotationTool,
    /// The page's annotations, refreshed by the app each pass.
    pub annotations: Vec<Annotation>,
    pub selected: Vec<AnnotationId>,
    /// The page the tool works on.
    pub page: usize,
    /// The edits since the app last took them ([`take_edits`](Self::take_edits)).
    edits: Vec<EditEvent<AnnotationEdit>>,
    text_quads: Option<TextQuads>,
}

impl AnnotateTool {
    pub fn new(tool: AnnotationTool, page: usize) -> Self {
        Self { tool, annotations: Vec::new(), selected: Vec::new(), page, edits: Vec::new(), text_quads: None }
    }

    /// The page text between two points as quads (PdfCraft's text selection), for markups.
    pub fn text_quads(mut self, quads: impl Fn(kurbo::Point, kurbo::Point) -> Vec<[kurbo::Point; 4]> + 'static) -> Self {
        self.text_quads = Some(Box::new(quads));
        self
    }

    /// The edits the tool reported since the last call, in order.
    pub fn take_edits(&mut self) -> Vec<EditEvent<AnnotationEdit>> {
        std::mem::take(&mut self.edits)
    }
}

// pdfcraft: crates/ui-egui/src/tools/comments/input.rs:13 `page_input` (and the gestures in
// `state.rs` `Gesture`), `geometry.rs:72` `resized`, `:37` `reshaped`, `paint.rs` `drawn_shape`,
// `erase`.
impl CanvasTool for AnnotateTool {
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        todo!()
    }
}

/// Draws a page's annotations over it: markups as translucent quads (highlight at the
/// annotation's opacity, lines for underline and strike-out, a wave for squiggly), shapes and ink
/// with their stroke, notes as their icon, text boxes with their text, and the handles of the
/// selected ones through the kernel's [`Handle`](craft_ui::primitives::Handle) (`handle.*`
/// tokens: squares, points as circles). Not interactive (the tool is).
pub struct AnnotationOverlay<'a> {
    annotations: &'a [Annotation],
    selected: &'a [AnnotationId],
}

impl<'a> AnnotationOverlay<'a> {
    pub fn new(annotations: &'a [Annotation], selected: &'a [AnnotationId]) -> Self {
        Self { annotations, selected }
    }

    /// `to_screen` maps page units to screen points (the canvas view).
    // pdfcraft: crates/ui-egui/src/tools/comments/paint.rs (`drawn_shape`, the handles).
    pub fn paint(&self, painter: &Painter, to_screen: impl Fn(kurbo::Point) -> egui::Pos2) {
        todo!()
    }
}

/// Acrobat's quick bar: Select, then the three groups (Comment ▸, Highlight ▸, Draw ▸), each a
/// button showing its last-used tool with a flyout of the group's tools; the colour and the
/// stroke width of the current tool follow. Each button is a Tab stop; Down opens a flyout.
pub struct AnnotationToolbar<'a> {
    id_salt: IdSalt,
    tool: &'a mut AnnotationTool,
    labels: Option<AnnotateLabels>,
}

impl<'a> AnnotationToolbar<'a> {
    pub fn new(id_salt: impl AsIdSalt, tool: &'a mut AnnotationTool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, labels: None }
    }

    pub fn labels(mut self, labels: AnnotateLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Changes `tool` directly.
    // pdfcraft: crates/ui-egui/src/tools/comments.rs:81 `GROUPS`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
