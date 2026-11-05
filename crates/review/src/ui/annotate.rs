//! The annotation tool on the canvas, the overlay that draws annotations, and the toolbar
//! (N5.03).

use craft_ui::behavior::{Selection, SelectionChange, SelectionPick, nudge};
use craft_ui::canvas::{CanvasTool, Marquee, MarqueeLook, Orientation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::controls::{NumberField, Swatch};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::{Labels as _, LocaleNumbers, fmt};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::patterns::{Toolbar, ToolbarItem};
use craft_ui::popover::{Popover, PopoverAnchor};
use craft_ui::primitives::{Handle, HandleShape};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use egui::text::{LayoutJob, TextFormat};
use egui::{AsIdSalt, Color32, CursorIcon, EventFilter, IdSalt, Key, Modifiers, Painter, Pos2, Response, Shape, Stroke, Ui};
use kurbo::{Point, Rect, Vec2};

use super::style::ReviewStyle;

use crate::annotate::{
    Annotation, AnnotationEdit, AnnotationGroup, AnnotationHandle, AnnotationHit, AnnotationId, AnnotationKind, AnnotationTool, hit, reshaped,
    resized, snapped_to_45,
};

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
        toolbar: "annotate.toolbar" = "Annotation tools";
        /// A group's button: the group, then its last-used tool.
        group_tool: "annotate.groupTool" = "{group}: {tool}";
        color: "annotate.color" = "Color";
        width: "annotate.width" = "Line width";
        /// The reader's names of the annotation kinds: card headings and sort groups.
        kind_note: "annotate.kind.note" = "Note";
        kind_text_box: "annotate.kind.textBox" = "Text box";
        kind_callout: "annotate.kind.callout" = "Callout";
        kind_attach: "annotate.kind.attach" = "Attachment";
        kind_highlight: "annotate.kind.highlight" = "Highlight";
        kind_underline: "annotate.kind.underline" = "Underline";
        kind_strike_out: "annotate.kind.strikeOut" = "Strikethrough";
        kind_squiggly: "annotate.kind.squiggly" = "Squiggly";
        kind_caret: "annotate.kind.caret" = "Insert text";
        kind_replace_text: "annotate.kind.replaceText" = "Replace text";
        kind_ink: "annotate.kind.ink" = "Drawing";
        kind_line: "annotate.kind.line" = "Line";
        kind_arrow: "annotate.kind.arrow" = "Arrow";
        kind_rectangle: "annotate.kind.rectangle" = "Rectangle";
        kind_oval: "annotate.kind.oval" = "Oval";
        kind_poly_line: "annotate.kind.polyLine" = "Polyline";
        kind_polygon: "annotate.kind.polygon" = "Polygon";
        kind_cloud: "annotate.kind.cloud" = "Cloud";
    }
}

impl AnnotateLabels {
    /// The reader's name of `kind` ("Note", "Text box", "Strikethrough"; PdfCraft's
    /// `comments_panel.rs:25` `subtype_label`).
    pub fn kind(&self, kind: AnnotationKind) -> &str {
        use AnnotationKind as K;
        match kind {
            K::Note => &self.kind_note,
            K::TextBox => &self.kind_text_box,
            K::Callout => &self.kind_callout,
            K::Attach => &self.kind_attach,
            K::Highlight => &self.kind_highlight,
            K::Underline => &self.kind_underline,
            K::StrikeOut => &self.kind_strike_out,
            K::Squiggly => &self.kind_squiggly,
            K::Caret => &self.kind_caret,
            K::ReplaceText => &self.kind_replace_text,
            K::Ink => &self.kind_ink,
            K::Line => &self.kind_line,
            K::Arrow => &self.kind_arrow,
            K::Rectangle => &self.kind_rectangle,
            K::Oval => &self.kind_oval,
            K::PolyLine => &self.kind_poly_line,
            K::Polygon => &self.kind_polygon,
            K::Cloud => &self.kind_cloud,
        }
    }
}

/// The page text between two page points as quads (four corners each).
pub type TextQuads = Box<dyn Fn(kurbo::Point, kurbo::Point) -> Vec<[kurbo::Point; 4]> + Send>;

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
/// - Eraser: a drag reports one `Erase` per Ink annotation it crosses, with the strokes in
///   ascending order.
/// - Keys: Delete removes the selection, arrows nudge it through
///   [`behavior::nudge`](craft_ui::behavior::nudge) (one point, Shift ten) as `Set`s.
/// - A double-click (from [`ToolPointer::clicks`](craft_ui::canvas::ToolPointer)) on a note or
///   text box reports `Open`; the app's page text and selection reach the tool in
///   [`CanvasTool::each_pass`] through the public fields.
/// - New annotations take the tool's [`color`](Self::color) and [`width`](Self::width) (a
///   highlight its [`highlight_color`](Self::highlight_color)); the toolbar's controls set them.
///   Notes, attachments and carets are created with an icon-sized rect and keep that size on
///   screen at any zoom, so hits test them against an icon side worked out each pass.
/// - Locked annotations are selected but never moved.
pub struct AnnotateTool {
    pub tool: AnnotationTool,
    /// The page's annotations, refreshed by the app each pass.
    pub annotations: Vec<Annotation>,
    pub selected: Vec<AnnotationId>,
    /// The page the tool works on.
    pub page: usize,
    /// The colour of new annotations other than highlights (straight RGB).
    pub color: [u8; 3],
    /// The colour of new highlights (straight RGB).
    pub highlight_color: [u8; 3],
    /// The stroke width of new annotations, in page units.
    pub width: f64,
    /// The edits since the app last took them ([`take_edits`](Self::take_edits)).
    edits: Vec<EditEvent<AnnotationEdit>>,
    text_quads: Option<TextQuads>,
    gesture: Option<Gesture>,
    /// The points of the PolyLine, Polygon or Cloud being drawn.
    poly: Vec<Point>,
    /// The Ink strokes drawn since the tool was chosen.
    strokes: Vec<Vec<Point>>,
    hover: Option<Point>,
    last_tool: AnnotationTool,
}

impl AnnotateTool {
    pub fn new(tool: AnnotationTool, page: usize) -> Self {
        Self {
            tool,
            annotations: Vec::new(),
            selected: Vec::new(),
            page,
            color: DEFAULT_COLOR,
            highlight_color: HIGHLIGHT_COLOR,
            width: DEFAULT_STROKE_WIDTH,
            edits: Vec::new(),
            text_quads: None,
            gesture: None,
            poly: Vec::new(),
            strokes: Vec::new(),
            hover: None,
            last_tool: tool,
        }
    }

    /// The page text between two points as quads (PdfCraft's text selection), for markups.
    pub fn text_quads(mut self, quads: impl Fn(kurbo::Point, kurbo::Point) -> Vec<[kurbo::Point; 4]> + Send + 'static) -> Self {
        self.text_quads = Some(Box::new(quads));
        self
    }

    /// The edits the tool reported since the last call, in order.
    pub fn take_edits(&mut self) -> Vec<EditEvent<AnnotationEdit>> {
        std::mem::take(&mut self.edits)
    }
}

/// Acrobat's default border width (Preferences > Commenting, "Line thickness" 1), in page units.
pub const DEFAULT_STROKE_WIDTH: f64 = 1.0;
/// Acrobat's default colours (Preferences > Commenting): red for shapes and notes, yellow for
/// highlights (straight RGB).
pub const DEFAULT_COLOR: [u8; 3] = [0xE0, 0x30, 0x30];
pub const HIGHLIGHT_COLOR: [u8; 3] = [0xFF, 0xE0, 0x30];
/// The colours the toolbar offers: Acrobat's quick colours for comments as PdfCraft carries them
/// (`crates/ui-egui/src/tools/comments.rs:260` `SWATCHES`, rounded to 8 bits): yellow, orange, red,
/// pink, purple, blue, light blue, green, grey and black. Acrobat's colour menu also has "Other
/// Color", which the app may add. Adobe publishes no list of the swatch values, so PdfCraft's
/// rendering of them is the source.
pub const PALETTE: [[u8; 3]; 10] = [
    [255, 240, 0],
    [255, 138, 0],
    [227, 33, 33],
    [255, 94, 163],
    [138, 64, 209],
    [0, 120, 214],
    [92, 191, 250],
    [46, 158, 92],
    [128, 128, 128],
    [0, 0, 0],
];
/// The widths the toolbar's width field accepts, in page units.
const WIDTH_RANGE: std::ops::RangeInclusive<f64> = 1.0..=12.0;
/// A highlight lets the text show through.
const HIGHLIGHT_OPACITY: f32 = 0.4;

/// What a press began and the drag or release continues.
#[derive(Clone, Debug)]
enum Gesture {
    /// Moving, resizing or reshaping selected annotations; `begun` once the pointer has moved.
    Edit { id: GestureId, begun: bool, origin: Point, change: Change },
    /// A selection rectangle on empty page.
    Marquee { origin: Point, current: Point, additive: bool },
    /// A box, oval or straight line drawn by dragging.
    Shape { kind: AnnotationKind, origin: Point, current: Point },
    /// Page text selected for a markup kind.
    Markup { kind: AnnotationKind, origin: Point, current: Point },
    /// One Ink stroke in progress.
    Stroke(Vec<Point>),
    /// The Eraser's path and the strokes it crossed.
    Erase { path: Vec<Point>, crossed: Vec<(AnnotationId, usize)> },
}

#[derive(Clone, Debug)]
enum Change {
    Move { ids: Vec<AnnotationId> },
    Resize { target: AnnotationId, handle: (i8, i8), from: Rect },
    Reshape { target: AnnotationId, index: usize, from: Vec<Point> },
}

fn bounds(points: impl IntoIterator<Item = Point>) -> Rect {
    let mut it = points.into_iter();
    let Some(first) = it.next() else { return Rect::ZERO };
    it.fold(Rect::from_points(first, first), |r, p| r.union_pt(p))
}

fn color32(annotation: &Annotation) -> Color32 {
    let [r, g, b] = annotation.color;
    Color32::from_rgba_unmultiplied(r, g, b, (annotation.opacity.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// `to` made square about `origin` (Shift on a box or oval).
fn squared(origin: Point, to: Point) -> Point {
    let d = to - origin;
    let side = d.x.abs().max(d.y.abs());
    Point::new(origin.x + side * d.x.signum(), origin.y + side * d.y.signum())
}

fn segments_cross(a: Point, b: Point, c: Point, d: Point, reach: f64) -> bool {
    let near = |p: Point, s: Point, e: Point| {
        let se = e - s;
        let len2 = se.hypot2();
        let t = if len2 <= f64::EPSILON { 0.0 } else { ((p - s).dot(se) / len2).clamp(0.0, 1.0) };
        (p - (s + se * t)).hypot() <= reach
    };
    let orient = |p: Point, q: Point, r: Point| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let proper = orient(a, b, c) * orient(a, b, d) < 0.0 && orient(c, d, a) * orient(c, d, b) < 0.0;
    proper || near(a, c, d) || near(b, c, d) || near(c, a, b) || near(d, a, b)
}

impl AnnotateTool {
    fn tolerance(cx: &ToolContext<'_>) -> f64 {
        cx.view.doc_len(Handle::style(cx.tokens, HandleShape::Square).hit / 2.0)
    }

    /// The side of a note, attachment or caret icon in page units at this pass's zoom.
    fn icon_side(cx: &ToolContext<'_>) -> f64 {
        cx.view.doc_len(IconSize::Md.points(cx.tokens))
    }

    fn push(&mut self, edit: AnnotationEdit, modifiers: Modifiers) {
        self.edits.push(EditEvent::set(edit, modifiers));
    }

    /// Selects `ids`, applied here and reported as a `Selection` change picked by `pick` on `row`.
    fn select(&mut self, ids: Vec<AnnotationId>, pick: SelectionPick, row: Option<AnnotationId>, modifiers: Modifiers) {
        self.selected.clone_from(&ids);
        let selection = Selection { lead: ids.last().copied(), anchor: row, items: ids };
        self.push(AnnotationEdit::Selection(SelectionChange { pick, row, selection }), modifiers);
    }

    /// Replaces the selection with `ids` (a marquee or a plain click), or clears it.
    fn replace_selection(&mut self, ids: Vec<AnnotationId>, modifiers: Modifiers) {
        let (pick, row) = if ids.is_empty() { (SelectionPick::Clear, None) } else { (SelectionPick::Replace, ids.last().copied()) };
        self.select(ids, pick, row, modifiers);
    }

    fn fresh(&self, kind: AnnotationKind, rect: Rect) -> Annotation {
        let markup = kind == AnnotationKind::Highlight;
        Annotation {
            id: AnnotationId::default(),
            page: self.page,
            kind,
            rect,
            points: Vec::new(),
            strokes: Vec::new(),
            color: if markup { self.highlight_color } else { self.color },
            opacity: if markup { HIGHLIGHT_OPACITY } else { 1.0 },
            stroke_width: self.width,
            text: String::new(),
            locked: false,
        }
    }

    fn create(&mut self, annotation: Annotation, modifiers: Modifiers) {
        self.push(AnnotationEdit::Create(annotation), modifiers);
    }

    /// Ends the pending Ink strokes as one annotation.
    fn flush_ink(&mut self, modifiers: Modifiers) {
        if self.strokes.is_empty() {
            return;
        }
        let strokes = std::mem::take(&mut self.strokes);
        let mut annotation = self.fresh(AnnotationKind::Ink, bounds(strokes.iter().flatten().copied()));
        annotation.strokes = strokes;
        self.create(annotation, modifiers);
    }

    /// Ends the pending PolyLine, Polygon or Cloud once it has enough points.
    fn finish_poly(&mut self, kind: AnnotationKind, modifiers: Modifiers) {
        let points = std::mem::take(&mut self.poly);
        let least = if kind == AnnotationKind::PolyLine { 2 } else { 3 };
        if points.len() >= least {
            let mut annotation = self.fresh(kind, bounds(points.iter().copied()));
            annotation.points = points;
            self.create(annotation, modifiers);
        }
    }

    fn is_poly(&self) -> bool {
        matches!(self.tool, AnnotationTool::Draw(AnnotationKind::PolyLine | AnnotationKind::Polygon | AnnotationKind::Cloud))
    }

    fn press_select(&mut self, pointer: &ToolPointer, tolerance: f64, icon_side: f64) -> ToolAction {
        let shift = pointer.modifiers.shift;
        let found = hit(&self.annotations, &self.selected, pointer.doc, tolerance, icon_side);
        let locked = |id: AnnotationId, list: &[Annotation]| list.iter().any(|a| a.id == id && a.locked);
        let edit = |change: Change, origin: Point| Gesture::Edit { id: GestureId::next(), begun: false, origin, change };
        match found {
            Some(AnnotationHit::Handle(id, AnnotationHandle::Box(hx, hy))) => {
                let from = self.annotations.iter().find(|a| a.id == id).map_or(Rect::ZERO, |a| a.rect);
                self.gesture = Some(edit(Change::Resize { target: id, handle: (hx, hy), from }, pointer.doc));
            }
            Some(AnnotationHit::Handle(id, AnnotationHandle::Point(index))) => {
                let from = self.annotations.iter().find(|a| a.id == id).map(|a| a.points.clone()).unwrap_or_default();
                self.gesture = Some(edit(Change::Reshape { target: id, index, from }, pointer.doc));
            }
            Some(AnnotationHit::Body(id)) => {
                let selected = self.selected.contains(&id);
                if pointer.clicks >= 2 {
                    // A second click on a note or text box asks the app to open its text.
                    let opens = self.annotations.iter().any(|a| a.id == id && matches!(a.kind, AnnotationKind::Note | AnnotationKind::TextBox));
                    if opens {
                        if !selected {
                            self.select(vec![id], SelectionPick::Replace, Some(id), pointer.modifiers);
                        }
                        self.push(AnnotationEdit::Open(id), pointer.modifiers);
                        return ToolAction::Done;
                    }
                }
                if shift {
                    let mut ids = self.selected.clone();
                    match ids.iter().position(|s| *s == id) {
                        Some(at) => {
                            ids.remove(at);
                        }
                        None => ids.push(id),
                    }
                    self.select(ids, SelectionPick::Toggle, Some(id), pointer.modifiers);
                } else if !selected {
                    self.select(vec![id], SelectionPick::Replace, Some(id), pointer.modifiers);
                }
                let ids: Vec<AnnotationId> = self.selected.iter().copied().filter(|s| !locked(*s, &self.annotations)).collect();
                if !ids.is_empty() {
                    self.gesture = Some(edit(Change::Move { ids }, pointer.doc));
                }
            }
            None => {
                self.gesture = Some(Gesture::Marquee { origin: pointer.doc, current: pointer.doc, additive: shift });
            }
        }
        ToolAction::Claimed
    }

    fn press_draw(&mut self, kind: AnnotationKind, pointer: &ToolPointer, icon_side: f64) -> ToolAction {
        use AnnotationKind as K;
        let at = pointer.doc;
        match kind {
            K::Note | K::Attach | K::Caret => {
                let mut annotation = self.fresh(kind, Rect::from_origin_size(at, (icon_side, icon_side)));
                annotation.points = vec![at];
                self.create(annotation, pointer.modifiers);
                ToolAction::Done
            }
            K::PolyLine | K::Polygon | K::Cloud => {
                if pointer.clicks >= 2 {
                    // The double-click's first press already added its point.
                    self.finish_poly(kind, pointer.modifiers);
                } else {
                    self.poly.push(at);
                }
                ToolAction::Done
            }
            K::Ink => {
                self.gesture = Some(Gesture::Stroke(vec![at]));
                ToolAction::Claimed
            }
            kind if kind.is_markup() => {
                self.gesture = Some(Gesture::Markup { kind, origin: at, current: at });
                ToolAction::Claimed
            }
            kind => {
                self.gesture = Some(Gesture::Shape { kind, origin: at, current: at });
                ToolAction::Claimed
            }
        }
    }

    fn shape_annotation(&self, kind: AnnotationKind, origin: Point, current: Point, shift: bool) -> Option<Annotation> {
        use AnnotationKind as K;
        match kind {
            K::Line | K::Arrow => {
                let end = if shift { snapped_to_45(origin, current) } else { current };
                if end == origin {
                    return None;
                }
                let mut annotation = self.fresh(kind, bounds([origin, end]));
                annotation.points = vec![origin, end];
                Some(annotation)
            }
            _ => {
                let end = if shift { squared(origin, current) } else { current };
                let rect = Rect::from_points(origin, end);
                (rect.width() > 0.0 && rect.height() > 0.0).then(|| self.fresh(kind, rect))
            }
        }
    }

    fn markup_annotation(&self, kind: AnnotationKind, origin: Point, current: Point) -> Option<Annotation> {
        let quads = (self.text_quads.as_ref()?)(origin, current);
        if quads.is_empty() {
            return None;
        }
        let points: Vec<Point> = quads.iter().flatten().copied().collect();
        let mut annotation = self.fresh(kind, bounds(points.iter().copied()));
        annotation.points = points;
        Some(annotation)
    }

    fn marquee_rect(origin: Point, current: Point) -> Rect {
        Rect::from_points(origin, current)
    }
}

// pdfcraft: crates/ui-egui/src/tools/comments/input.rs:13 `page_input` (and the gestures in
// `state.rs` `Gesture`), `geometry.rs:72` `resized`, `:37` `reshaped`, `paint.rs` `drawn_shape`,
// `erase`.
impl CanvasTool for AnnotateTool {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        if self.tool != self.last_tool {
            self.flush_ink(cx.modifiers);
            self.poly.clear();
            self.last_tool = self.tool;
        }
    }

    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        match self.tool {
            AnnotationTool::Select => ToolCursor::System(CursorIcon::Default),
            AnnotationTool::Draw(_) | AnnotationTool::Eraser => ToolCursor::System(CursorIcon::Crosshair),
        }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.gesture = None;
        let tolerance = Self::tolerance(cx);
        match self.tool {
            AnnotationTool::Select => self.press_select(pointer, tolerance, Self::icon_side(cx)),
            AnnotationTool::Draw(kind) => self.press_draw(kind, pointer, Self::icon_side(cx)),
            AnnotationTool::Eraser => {
                self.gesture = Some(Gesture::Erase { path: vec![pointer.doc], crossed: Vec::new() });
                ToolAction::Claimed
            }
        }
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let tolerance = Self::tolerance(cx);
        let Some(gesture) = self.gesture.as_mut() else { return };
        let mut event = None;
        match gesture {
            Gesture::Edit { id, begun, origin, change } => {
                let value = match change {
                    Change::Move { ids } => AnnotationEdit::Move { ids: ids.clone(), delta: pointer.doc - *origin },
                    Change::Resize { target, handle, from } => {
                        let aspect = (pointer.modifiers.shift && from.height() > 0.0).then(|| from.width() / from.height());
                        AnnotationEdit::Resize { id: *target, rect: resized(*from, *handle, pointer.doc - *origin, aspect) }
                    }
                    Change::Reshape { target, index, from } => {
                        AnnotationEdit::Reshape { id: *target, points: reshaped(from, *index, pointer.doc, pointer.modifiers.shift) }
                    }
                };
                event = Some(if *begun {
                    EditEvent::update(*id, value, pointer.modifiers)
                } else {
                    *begun = true;
                    EditEvent::begin_with(*id, value)
                });
            }
            Gesture::Marquee { current, .. } | Gesture::Shape { current, .. } | Gesture::Markup { current, .. } => *current = pointer.doc,
            Gesture::Stroke(points) => points.push(pointer.doc),
            Gesture::Erase { path, crossed } => {
                let last = path.last().copied().unwrap_or(pointer.doc);
                for a in self.annotations.iter().filter(|a| a.kind == AnnotationKind::Ink && !a.locked) {
                    for (i, stroke) in a.strokes.iter().enumerate() {
                        let hits = stroke.windows(2).any(|w| segments_cross(last, pointer.doc, w[0], w[1], tolerance + a.stroke_width / 2.0))
                            || stroke.len() == 1 && (stroke[0] - pointer.doc).hypot() <= tolerance;
                        if hits && !crossed.contains(&(a.id, i)) {
                            crossed.push((a.id, i));
                        }
                    }
                }
                path.push(pointer.doc);
            }
        }
        self.hover = Some(pointer.doc);
        if let Some(event) = event {
            self.edits.push(event);
        }
        cx.request_repaint();
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some(gesture) = self.gesture.take() else { return };
        let modifiers = pointer.modifiers;
        let icon_side = Self::icon_side(cx);
        match gesture {
            Gesture::Edit { id, begun, origin, change } => {
                if !begun {
                    return;
                }
                let value = match change {
                    Change::Move { ids } => AnnotationEdit::Move { ids, delta: pointer.doc - origin },
                    Change::Resize { target, handle, from } => {
                        let aspect = (modifiers.shift && from.height() > 0.0).then(|| from.width() / from.height());
                        AnnotationEdit::Resize { id: target, rect: resized(from, handle, pointer.doc - origin, aspect) }
                    }
                    Change::Reshape { target, index, from } => {
                        AnnotationEdit::Reshape { id: target, points: reshaped(&from, index, pointer.doc, modifiers.shift) }
                    }
                };
                self.edits.push(EditEvent::commit(id, value, true, modifiers));
            }
            Gesture::Marquee { origin, additive, .. } => {
                let area = Self::marquee_rect(origin, pointer.doc);
                let inside: Vec<AnnotationId> = if area.width() == 0.0 && area.height() == 0.0 {
                    Vec::new()
                } else {
                    self.annotations
                        .iter()
                        .filter(|a| {
                            let extent = a.extent(icon_side);
                            !area.intersect(extent).is_zero_area() || area.contains(extent.center())
                        })
                        .map(|a| a.id)
                        .collect()
                };
                let mut ids = if additive { self.selected.clone() } else { Vec::new() };
                for id in inside {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                self.replace_selection(ids, modifiers);
            }
            Gesture::Shape { kind, origin, .. } => {
                if let Some(annotation) = self.shape_annotation(kind, origin, pointer.doc, modifiers.shift) {
                    self.create(annotation, modifiers);
                }
            }
            Gesture::Markup { kind, origin, .. } => {
                if let Some(annotation) = self.markup_annotation(kind, origin, pointer.doc) {
                    self.create(annotation, modifiers);
                }
            }
            Gesture::Stroke(points) => self.strokes.push(points),
            Gesture::Erase { crossed, .. } => {
                // One event per annotation, in the order the Eraser met them: an app that removes
                // strokes by index must not see the same annotation twice.
                let mut ids: Vec<AnnotationId> = Vec::new();
                for (id, _) in &crossed {
                    if !ids.contains(id) {
                        ids.push(*id);
                    }
                }
                for id in ids {
                    let mut strokes: Vec<usize> = crossed.iter().filter(|(a, _)| *a == id).map(|(_, i)| *i).collect();
                    strokes.sort_unstable();
                    self.push(AnnotationEdit::Erase { id, strokes }, modifiers);
                }
            }
        }
    }

    fn hover(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.hover = Some(pointer.doc);
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        if let Some(Gesture::Edit { id, begun: true, .. }) = self.gesture.take() {
            self.edits.push(EditEvent::cancel(id, reason));
        }
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        if !key.pressed {
            return false;
        }
        let editable = |tool: &Self| -> Vec<AnnotationId> {
            tool.selected.iter().copied().filter(|id| tool.annotations.iter().any(|a| a.id == *id && !a.locked)).collect()
        };
        match key.key {
            Key::Escape if !self.poly.is_empty() || !self.strokes.is_empty() => {
                self.poly.clear();
                self.strokes.clear();
                true
            }
            Key::Enter if self.is_poly() && !self.poly.is_empty() => {
                if let AnnotationTool::Draw(kind) = self.tool {
                    self.finish_poly(kind, key.modifiers);
                }
                true
            }
            Key::Enter if !self.strokes.is_empty() => {
                self.flush_ink(key.modifiers);
                true
            }
            Key::Delete | Key::Backspace if self.tool == AnnotationTool::Select => {
                let ids = editable(self);
                if ids.is_empty() {
                    return false;
                }
                self.push(AnnotationEdit::Delete(ids), key.modifiers);
                true
            }
            Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown if self.tool == AnnotationTool::Select => {
                let ids = editable(self);
                if ids.is_empty() || !(key.modifiers.is_none() || key.modifiers.matches_exact(Modifiers::SHIFT)) {
                    return false;
                }
                let step = if key.modifiers.shift { nudge::BIG_STEP } else { nudge::STEP };
                let direction = match key.key {
                    Key::ArrowLeft => Vec2::new(-1.0, 0.0),
                    Key::ArrowRight => Vec2::new(1.0, 0.0),
                    Key::ArrowUp => Vec2::new(0.0, -1.0),
                    _ => Vec2::new(0.0, 1.0),
                };
                self.push(AnnotationEdit::Move { ids, delta: direction * step }, key.modifiers);
                true
            }
            _ => false,
        }
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        let time = cx.time;
        let outline = |marquee: Marquee, color: Color32| marquee.paint(painter, cx.view, MarqueeLook::Contrast(color), time);
        let accent: Color32 = cx.tokens.color.overlay.line.to_egui();
        let drawn = |kind: AnnotationKind| self.fresh(kind, Rect::ZERO);
        match &self.gesture {
            Some(Gesture::Marquee { origin, current, .. }) => {
                Marquee::rect(Self::marquee_rect(*origin, *current), "").paint(painter, cx.view, MarqueeLook::Band, time);
            }
            Some(Gesture::Shape { kind, origin, current }) => {
                let shift = cx.modifiers.shift;
                let color = color32(&drawn(*kind));
                match kind {
                    AnnotationKind::Line | AnnotationKind::Arrow => {
                        let end = if shift { snapped_to_45(*origin, *current) } else { *current };
                        outline(Marquee::polyline([*origin, end], ""), color);
                    }
                    AnnotationKind::Oval => {
                        outline(Marquee::ellipse(Rect::from_points(*origin, if shift { squared(*origin, *current) } else { *current }), ""), color)
                    }
                    _ => outline(Marquee::rect(Rect::from_points(*origin, if shift { squared(*origin, *current) } else { *current }), ""), color),
                }
            }
            Some(Gesture::Markup { kind, origin, current }) => {
                if let (Some(quads), color) = (self.text_quads.as_ref(), color32(&drawn(*kind))) {
                    for quad in quads(*origin, *current) {
                        outline(Marquee::polygon(quad, ""), color);
                    }
                }
            }
            Some(Gesture::Stroke(points)) => outline(Marquee::polyline(points.iter().copied(), ""), color32(&drawn(AnnotationKind::Ink))),
            Some(Gesture::Erase { path, .. }) => outline(Marquee::polyline(path.iter().copied(), ""), accent),
            Some(Gesture::Edit { .. }) | None => {}
        }
        for stroke in &self.strokes {
            outline(Marquee::polyline(stroke.iter().copied(), ""), color32(&drawn(AnnotationKind::Ink)));
        }
        if let AnnotationTool::Draw(kind) = self.tool
            && self.is_poly()
            && !self.poly.is_empty()
        {
            let mut path = self.poly.clone();
            path.extend(self.hover);
            outline(Marquee::polyline(path, ""), color32(&drawn(kind)));
        }
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
}

/// An arrow head's length and a cloud scallop's width, in stroke widths.
const ARROW_HEAD: f32 = 6.0;
const CLOUD_BUMP: f32 = 8.0;
/// A squiggle's wavelength, in stroke widths.
const WAVE: f32 = 4.0;

fn scalloped(corners: &[Pos2], bump: f32) -> Vec<Pos2> {
    let area: f32 = corners.iter().zip(corners.iter().cycle().skip(1)).map(|(a, b)| a.x * b.y - b.x * a.y).sum();
    let outward = if area >= 0.0 { -1.0 } else { 1.0 };
    let mut out = Vec::new();
    for (a, b) in corners.iter().zip(corners.iter().cycle().skip(1)) {
        let edge = *b - *a;
        let count = (edge.length() / bump).round().max(1.0) as usize;
        let normal = egui::vec2(-edge.y, edge.x).normalized() * outward;
        for i in 0..count {
            let from = *a + edge * (i as f32 / count as f32);
            let to = *a + edge * ((i + 1) as f32 / count as f32);
            let radius = (to - from).length() / 2.0;
            let centre = from + (to - from) / 2.0;
            let steps = 8;
            for s in 0..steps {
                let t = std::f32::consts::PI * s as f32 / steps as f32;
                // Rotates from the segment's start, over the outward side, to its end.
                let along = (from - centre).normalized();
                out.push(centre + along * radius * t.cos() + normal * radius * t.sin());
            }
        }
    }
    out
}

fn ellipse_points(rect: Rect, to_screen: &impl Fn(Point) -> Pos2) -> Vec<Pos2> {
    let (c, rx, ry) = (rect.center(), rect.width() / 2.0, rect.height() / 2.0);
    let steps = 48;
    (0..steps)
        .map(|i| {
            let t = std::f64::consts::TAU * f64::from(i) / f64::from(steps);
            to_screen(Point::new(c.x + rx * t.cos(), c.y + ry * t.sin()))
        })
        .collect()
}

fn screen_rect(rect: Rect, to_screen: &impl Fn(Point) -> Pos2) -> egui::Rect {
    egui::Rect::from_two_pos(to_screen(Point::new(rect.x0, rect.y0)), to_screen(Point::new(rect.x1, rect.y1)))
}

fn tool_icon(tool: AnnotationTool) -> Icon {
    use AnnotationKind as K;
    match tool {
        AnnotationTool::Select => Icon::Pointer,
        AnnotationTool::Eraser => Icon::Eraser,
        AnnotationTool::Draw(kind) => match kind {
            K::Note => Icon::Note,
            K::TextBox | K::Callout => Icon::Text,
            K::Attach => Icon::Link,
            K::Highlight => Icon::Brush,
            K::Underline => Icon::TextUnderline,
            K::StrikeOut => Icon::TextStrikethrough,
            K::Squiggly => Icon::Spline,
            K::Caret => Icon::TextCursor,
            K::ReplaceText => Icon::BrushReplace,
            K::Ink => Icon::Pencil,
            K::Line => Icon::Slash,
            K::Arrow => Icon::TriangleRight,
            K::Rectangle => Icon::Square,
            K::Oval => Icon::Circle,
            K::PolyLine => Icon::PenTool,
            K::Polygon => Icon::Pentagon,
            K::Cloud => Icon::Cloud,
        },
    }
}

fn tool_label(labels: &AnnotateLabels, tool: AnnotationTool) -> String {
    use AnnotationKind as K;
    match tool {
        AnnotationTool::Select => &labels.select,
        AnnotationTool::Eraser => &labels.eraser,
        AnnotationTool::Draw(kind) => match kind {
            K::Note => &labels.note,
            K::TextBox => &labels.text_box,
            K::Callout => &labels.callout,
            K::Attach => &labels.attach,
            K::Highlight => &labels.highlight,
            K::Underline => &labels.underline,
            K::StrikeOut => &labels.strike_out,
            K::Squiggly => &labels.squiggly,
            K::Caret => &labels.caret,
            K::ReplaceText => &labels.replace_text,
            K::Ink => &labels.ink,
            K::Line => &labels.line,
            K::Arrow => &labels.arrow,
            K::Rectangle => &labels.rectangle,
            K::Oval => &labels.oval,
            K::PolyLine => &labels.poly_line,
            K::Polygon => &labels.polygon,
            K::Cloud => &labels.cloud,
        },
    }
    .to_string()
}

fn group_label(labels: &AnnotateLabels, group: AnnotationGroup) -> String {
    match group {
        AnnotationGroup::Comment => &labels.comment_group,
        AnnotationGroup::Highlight => &labels.highlight_group,
        AnnotationGroup::Draw => &labels.draw_group,
    }
    .to_string()
}

fn paint_annotation(painter: &Painter, a: &Annotation, to_screen: &impl Fn(Point) -> Pos2, scale: f32, icon_points: f32, text_format: &TextFormat) {
    use AnnotationKind as K;
    let color = color32(a);
    let width = (a.stroke_width as f32 * scale).max(craft_theme::Stroke::W1.width(painter.ctx().pixels_per_point()));
    let stroke = Stroke::new(width, color);
    let map = |points: &[Point]| -> Vec<Pos2> { points.iter().map(|&p| to_screen(p)).collect() };
    match a.kind {
        K::Highlight => {
            for quad in a.points.chunks(4) {
                let r = quad.iter().map(|&p| to_screen(p)).fold(egui::Rect::NOTHING, |r, p| r.union(egui::Rect::from_min_max(p, p)));
                painter.rect_filled(r, 0.0, color);
            }
        }
        K::Underline | K::StrikeOut | K::Squiggly | K::ReplaceText => {
            for quad in a.points.chunks(4) {
                let r = quad.iter().map(|&p| to_screen(p)).fold(egui::Rect::NOTHING, |r, p| r.union(egui::Rect::from_min_max(p, p)));
                let y = if matches!(a.kind, K::StrikeOut | K::ReplaceText) { r.center().y } else { r.bottom() };
                if a.kind == K::Squiggly {
                    let (amplitude, length) = (width, width * WAVE);
                    let steps = (r.width() / length * 2.0).ceil().max(1.0) as usize;
                    let wave: Vec<Pos2> = (0..=steps)
                        .map(|i| egui::pos2(r.left() + r.width() * i as f32 / steps as f32, y + if i % 2 == 0 { -amplitude } else { amplitude }))
                        .collect();
                    painter.add(Shape::line(wave, stroke));
                } else {
                    painter.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], stroke);
                }
            }
        }
        K::Line | K::Arrow | K::PolyLine => {
            let path = map(&a.points);
            if let [.., from, to] = path[..] {
                painter.add(Shape::line(path.clone(), stroke));
                if a.kind == K::Arrow {
                    let back = (from - to).normalized() * width * ARROW_HEAD;
                    for turn in [0.5_f32, -0.5] {
                        let (s, c) = turn.sin_cos();
                        painter.line_segment([to, to + egui::vec2(back.x * c - back.y * s, back.x * s + back.y * c)], stroke);
                    }
                }
            }
        }
        K::Polygon => {
            painter.add(Shape::closed_line(map(&a.points), stroke));
        }
        K::Cloud => {
            let corners = map(&a.points);
            if corners.len() > 2 {
                painter.add(Shape::closed_line(scalloped(&corners, width * CLOUD_BUMP), stroke));
            }
        }
        K::Ink => {
            for s in &a.strokes {
                painter.add(Shape::line(map(s), stroke));
            }
        }
        K::Rectangle => {
            painter.add(Shape::closed_line(
                vec![
                    to_screen(Point::new(a.rect.x0, a.rect.y0)),
                    to_screen(Point::new(a.rect.x1, a.rect.y0)),
                    to_screen(Point::new(a.rect.x1, a.rect.y1)),
                    to_screen(Point::new(a.rect.x0, a.rect.y1)),
                ],
                stroke,
            ));
        }
        K::Oval => {
            painter.add(Shape::closed_line(ellipse_points(a.rect, to_screen), stroke));
        }
        K::TextBox | K::Callout => {
            let r = screen_rect(a.rect, to_screen);
            painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Inside);
            // The text is document-scaled: the role's size at zoom 1, wrapped at the box's width.
            let mut format = text_format.clone();
            format.font_id.size *= scale;
            format.line_height = format.line_height.map(|h| h * scale);
            format.color = color;
            let mut job = LayoutJob::single_section(a.text.clone(), format);
            job.wrap.max_width = r.width();
            let galley = painter.layout_job(job);
            painter.with_clip_rect(r).galley(r.left_top(), galley, color);
        }
        K::Note | K::Attach | K::Caret => {
            let at = to_screen(Point::new(a.rect.x0, a.rect.y0));
            let slot = egui::Rect::from_min_size(at, egui::vec2(icon_points, icon_points));
            let icon = match a.kind {
                K::Note => Icon::Note,
                K::Attach => Icon::Link,
                _ => Icon::TextCursor,
            };
            icons::paint_on(painter, slot, icon, icons::active(painter.ctx()), icon_points, IconSize::Md.stroke(), color);
        }
    }
}

impl<'a> AnnotationOverlay<'a> {
    /// `tokens` are the ones the host `Ui` resolves (`Tokens::of(ui)`); `to_screen` maps page units
    /// to screen points (the canvas view).
    // pdfcraft: crates/ui-egui/src/tools/comments/paint.rs (`drawn_shape`, the handles).
    pub fn paint(&self, painter: &Painter, tokens: &UiTokens, to_screen: impl Fn(Point) -> Pos2) {
        let scale = (to_screen(Point::new(1.0, 0.0)) - to_screen(Point::ZERO)).length();
        let icon_points = IconSize::Md.points(tokens);
        let text_format = role_format(tokens, ReviewStyle::of(tokens).annotation_text, Color32::PLACEHOLDER);
        for a in self.annotations {
            paint_annotation(painter, a, &to_screen, scale, icon_points, &text_format);
        }
        let outline = Stroke::new(Handle::style(tokens, HandleShape::Square).stroke.width, tokens.color.overlay.line.to_egui());
        let ppp = painter.ctx().pixels_per_point();
        for a in self.annotations.iter().filter(|a| self.selected.contains(&a.id)) {
            let points = crate::annotate::handles(a);
            if a.kind.is_markup() {
                for quad in a.points.chunks(4) {
                    let r = quad.iter().map(|&p| to_screen(p)).fold(egui::Rect::NOTHING, |r, p| r.union(egui::Rect::from_min_max(p, p)));
                    painter.rect_stroke(r, 0.0, outline, egui::StrokeKind::Outside);
                }
            } else if a.kind.is_pointwise() && !a.points.is_empty() {
                let path: Vec<Pos2> = a.points.iter().map(|&p| to_screen(p)).collect();
                painter.add(Shape::line(path, outline));
            } else {
                painter.rect_stroke(screen_rect(a.rect, &to_screen), 0.0, outline, egui::StrokeKind::Outside);
            }
            for (handle, at) in points {
                let shape = match handle {
                    AnnotationHandle::Box(..) => HandleShape::Square,
                    AnnotationHandle::Point(_) => HandleShape::Round,
                };
                painter.extend(Handle::shapes(tokens, shape, to_screen(at), ppp));
            }
        }
    }
}

/// Acrobat's quick bar on [`patterns::Toolbar`](craft_ui::patterns::Toolbar): Select, then the
/// three groups (Comment, Highlight, Draw), each a button showing its last-used tool and a
/// chevron that opens a flyout of the group's tools. A second row holds the colour and the line
/// width of the tool's new annotations ([`AnnotateTool::color`], [`AnnotateTool::width`]); both
/// are disabled for tools that draw nothing (Select, Eraser) and the width for markups. Every
/// button is a Tab stop and the arrow keys move between them; Down on a group button opens its
/// flyout.
pub struct AnnotationToolbar<'a> {
    id_salt: IdSalt,
    tool: &'a mut AnnotateTool,
    labels: Option<AnnotateLabels>,
}

/// What [`AnnotationToolbar::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct AnnotationToolbarOutput {
    /// The toolbar's buttons together; changed when the tool, the colour or the width changed.
    pub response: Response,
    /// Each group's flyout popup, for `egui::Popup::open_id` and `Popup::is_id_open`.
    pub flyouts: Vec<(AnnotationGroup, egui::Id)>,
}

impl<'a> AnnotationToolbar<'a> {
    pub fn new(id_salt: impl AsIdSalt, tool: &'a mut AnnotateTool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, labels: None }
    }

    pub fn labels(mut self, labels: AnnotateLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Changes the tool's `tool`, `color` and `width` directly.
    // pdfcraft: crates/ui-egui/src/tools/comments.rs:81 `GROUPS`.
    pub fn show(self, ui: &mut Ui) -> AnnotationToolbarOutput {
        let labels = self.labels.unwrap_or_else(|| AnnotateLabels::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let annotate = self.tool;
        let before = (annotate.tool, annotate.color, annotate.highlight_color, annotate.width);
        let lasts: Vec<AnnotationTool> =
            AnnotationGroup::ALL.iter().map(|&group| last_tool(ui, id.with(("last", group as u8)), group, annotate.tool)).collect();
        let selected = if annotate.tool == AnnotationTool::Select {
            (0, 0)
        } else {
            AnnotationGroup::ALL.iter().position(|g| g.tools().contains(&annotate.tool)).map_or((0, 0), |g| (g + 1, 0))
        };
        let mut toolbar = Toolbar::new(id.with("bar"), Orientation::Horizontal)
            .label(&labels.toolbar)
            .selected(Some(selected))
            .group([ToolbarItem::new(Icon::Pointer, tool_label(&labels, AnnotationTool::Select))]);
        for (&group, &last) in AnnotationGroup::ALL.iter().zip(&lasts) {
            let name = fmt(&labels.group_tool, &[("group", &group_label(&labels, group)), ("tool", &tool_label(&labels, last))]);
            toolbar = toolbar.group([ToolbarItem::new(tool_icon(last), name), ToolbarItem::new(Icon::ChevronDown, group_label(&labels, group))]);
        }
        let bar = toolbar.show(ui);
        // The buttons in order: Select, then each group's tool button and chevron.
        if let Some((group, item)) = bar.clicked {
            match (group, item) {
                (0, _) => annotate.tool = AnnotationTool::Select,
                (g, 0) => annotate.tool = lasts[g - 1],
                _ => {}
            }
        }
        let mut flyouts = Vec::new();
        let mut all = bar.items.first().cloned();
        for (n, &group) in AnnotationGroup::ALL.iter().enumerate() {
            let (Some(main), Some(chevron)) = (bar.items.get(1 + 2 * n), bar.items.get(2 + 2 * n)) else { continue };
            let down = main.has_focus() && {
                ui.memory_mut(|m| m.set_focus_lock_filter(main.id, EventFilter { vertical_arrows: true, ..Default::default() }));
                ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown))
            };
            let entries: Vec<MenuEntry<AnnotationTool>> =
                group.tools().iter().map(|&t| MenuItem::new(tool_label(&labels, t), t).icon(tool_icon(t)).radio(annotate.tool == t).into()).collect();
            if let Some(chosen) = PressMenu::new(&entries).keyboard_owner(main.id, down).show(ui, chevron).chosen {
                annotate.tool = chosen;
            }
            flyouts.push((group, egui::Popup::default_response_id(chevron)));
            all = Some(all.map_or_else(|| main.clone(), |r| r.union(main.clone())).union(chevron.clone()));
        }
        let response = ui.horizontal(|ui| style_controls(ui, id, annotate, &labels)).inner;
        let mut response = all.map_or(response.clone(), |all| all.union(response));
        if before != (annotate.tool, annotate.color, annotate.highlight_color, annotate.width) {
            response.mark_changed();
        }
        AnnotationToolbarOutput { response, flyouts }
    }
}

/// The colour well with its palette popover, and the line width field, for the current tool.
fn style_controls(ui: &mut Ui, id: egui::Id, annotate: &mut AnnotateTool, labels: &AnnotateLabels) -> Response {
    let tokens = Tokens::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let draws = matches!(annotate.tool, AnnotationTool::Draw(_));
    let highlight = annotate.tool == AnnotationTool::Draw(AnnotationKind::Highlight);
    let markup = matches!(annotate.tool, AnnotationTool::Draw(kind) if kind.is_markup());
    let current = if highlight { annotate.highlight_color } else { annotate.color };
    let rgba = |[r, g, b]: [u8; 3]| craft_theme::Rgba([r, g, b, 0xFF]);

    let well = ui.add(Swatch::new(id.with("color"), &labels.color, rgba(current)).enabled(draws));
    let open_id = id.with("palette-open");
    let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
    if well.clicked() {
        open = !open;
    }
    let mut picked = None;
    Popover::new(id.with("palette"), PopoverAnchor::Control(well.clone())).toggle(well.id).label(&labels.color).show(ui, &mut open, |ui, _| {
        ui.spacing_mut().item_spacing = egui::vec2(tokens.space.gap.inline, tokens.space.gap.inline);
        ui.horizontal(|ui| {
            for (n, color) in PALETTE.into_iter().enumerate() {
                let swatch = Swatch::new(id.with(("palette", n)), Swatch::hex(rgba(color)), rgba(color)).radio(color == current).position_in_set(n);
                if ui.add(swatch).clicked() {
                    picked = Some(color);
                }
            }
        });
    });
    if let Some(color) = picked {
        if highlight {
            annotate.highlight_color = color;
        } else {
            annotate.color = color;
        }
        open = false;
    }
    ui.data_mut(|d| d.insert_temp(open_id, open));

    let width = NumberField::new(id.with("width"), &labels.width, annotate.width, &numbers)
        .range(WIDTH_RANGE)
        .step(1.0)
        .decimals(0..=1)
        .enabled(draws && !markup)
        .show(ui);
    if let Some(value) = width.event.as_ref().and_then(|event| event.value()) {
        annotate.width = value.clamp(*WIDTH_RANGE.start(), *WIDTH_RANGE.end());
    }
    well.union(width.response)
}

/// The group's last-used tool is kept in egui memory, so the group button keeps showing it.
fn last_tool(ui: &Ui, key: egui::Id, group: AnnotationGroup, current: AnnotationTool) -> AnnotationTool {
    if group.tools().contains(&current) {
        ui.data_mut(|d| d.insert_temp(key, current));
        return current;
    }
    ui.data(|d| d.get_temp::<AnnotationTool>(key)).filter(|t| group.tools().contains(t)).unwrap_or(group.tools()[0])
}
