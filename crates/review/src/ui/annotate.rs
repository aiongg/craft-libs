//! The annotation tool on the canvas, the overlay that draws annotations, and the toolbar.

use craft_behavior::geometry::draft::turn_to_45;
use craft_behavior::{CancelReason, Cursor, CursorIcon, EditEvent, GestureId, Input, Metrics, Mixed, PointerButton, SelectionChange};
use craft_keymap::Mods;
use craft_theme::Srgba;
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::{CanvasTool, NUDGE, NUDGE_FAR, ToolAction, ToolContext, ToolKey, ToolPath, ToolPointer, canvas_space};
use craft_ui::choice::{RadioGroup, Swatch};
use craft_ui::contract::{
    Bind, BodyCx, Common, Component, Cx, MeasureCx, NoValue, Outcome, PaintUi, Props as _, Remember as _, Role, Show as _, ShowContainer as _,
};
use craft_ui::fonts::FontRef;
use craft_ui::layout_components::Row;
use craft_ui::patterns::form_dialogs::{DialogFields, FieldValue, Param, ParamKind};
use craft_ui::popover::Popover;
use craft_ui::primitives::{ContentPainterCx, DocRun, DocText, Draw, Handle, HandleShape, PaintShape, StrokeWidth};
use craft_ui::rails::{FlyoutButton, LastUsed, RailTool, SlotId, ToolSlot, remembered, shown_tool};
use craft_ui::runtime::env::{Env, Label};
use craft_ui::style::{ComponentStyle, DataColor, HandleLook, StateSet, StyleColor, StyleCx};
use craft_ui::text::NumberField;
use egui::{Color32, Pos2, Response, Shape, Stroke, StrokeKind, Ui};
use kurbo::{Point, Rect};
use std::sync::Arc;

use crate::annotate::{
    Annotation, AnnotationEdit, AnnotationGroup, AnnotationHandle, AnnotationHit, AnnotationId, AnnotationKind, AnnotationTool, handles, hit,
    reshaped, resized,
};

#[cfg(feature = "screen-testing")]
mod stories;

craft_ui::words! {
    /// The toolbar's group names, the kind names the reader sees, the palette colour names, the names of the
    /// undo steps the tool's edits make and the words of the "New Annotation..." form, English by default.
    #[non_exhaustive]
    pub struct AnnotateWords {
        comment_group: "annotate.commentGroup" = "Comment";
        highlight_group: "annotate.highlightGroup" = "Highlight";
        draw_group: "annotate.drawGroup" = "Draw";
        select: "annotate.select" = "Select";
        toolbar: "annotate.toolbar" = "Annotation tools";
        color: "annotate.color" = "Color";
        palette: "annotate.palette" = "Colors";
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

        /// The names of the palette colours, in `PALETTE` order.
        palette_yellow: "annotate.palette.0" = "Yellow";
        palette_orange: "annotate.palette.1" = "Orange";
        palette_red: "annotate.palette.2" = "Red";
        palette_pink: "annotate.palette.3" = "Pink";
        palette_purple: "annotate.palette.4" = "Purple";
        palette_blue: "annotate.palette.5" = "Blue";
        palette_light_blue: "annotate.palette.6" = "Light blue";
        palette_green: "annotate.palette.7" = "Green";
        palette_grey: "annotate.palette.8" = "Grey";
        palette_black: "annotate.palette.9" = "Black";

        edit_select: "annotate.edit.select" = "Select Annotations";
        edit_move: "annotate.edit.move" = "Move Annotations";
        edit_resize: "annotate.edit.resize" = "Resize Annotation";
        edit_reshape: "annotate.edit.reshape" = "Reshape Annotation";
        edit_create: "annotate.edit.create" = "Add Annotation";
        edit_delete: "annotate.edit.delete" = "Delete Annotations";
        edit_erase: "annotate.edit.erase" = "Erase Ink";
        edit_open: "annotate.edit.open" = "Open Annotation";

        form_title: "annotate.form.title" = "New Annotation";
        form_kind: "annotate.form.kind" = "Kind";
        form_page: "annotate.form.page" = "Page";
        form_x: "annotate.form.x" = "X (points)";
        form_y: "annotate.form.y" = "Y (points)";
        form_width: "annotate.form.width" = "Width (points)";
        form_height: "annotate.form.height" = "Height (points)";
    }
}

impl AnnotateWords {
    /// The reader's name of `kind` ("Note", "Text box", "Strikethrough").
    pub fn kind(&self, kind: AnnotationKind) -> &str {
        use AnnotationKind as K;
        match kind {
            K::Note => self.kind_note.as_str(),
            K::TextBox => self.kind_text_box.as_str(),
            K::Callout => self.kind_callout.as_str(),
            K::Attach => self.kind_attach.as_str(),
            K::Highlight => self.kind_highlight.as_str(),
            K::Underline => self.kind_underline.as_str(),
            K::StrikeOut => self.kind_strike_out.as_str(),
            K::Squiggly => self.kind_squiggly.as_str(),
            K::Caret => self.kind_caret.as_str(),
            K::ReplaceText => self.kind_replace_text.as_str(),
            K::Ink => self.kind_ink.as_str(),
            K::Line => self.kind_line.as_str(),
            K::Arrow => self.kind_arrow.as_str(),
            K::Rectangle => self.kind_rectangle.as_str(),
            K::Oval => self.kind_oval.as_str(),
            K::PolyLine => self.kind_poly_line.as_str(),
            K::Polygon => self.kind_polygon.as_str(),
            K::Cloud => self.kind_cloud.as_str(),
        }
    }

    /// The name of palette colour `at` (an index into [`PALETTE`]); empty past the last.
    pub fn palette_color(&self, at: usize) -> &str {
        [
            &self.palette_yellow,
            &self.palette_orange,
            &self.palette_red,
            &self.palette_pink,
            &self.palette_purple,
            &self.palette_blue,
            &self.palette_light_blue,
            &self.palette_green,
            &self.palette_grey,
            &self.palette_black,
        ]
        .get(at)
        .map_or("", |name| name.as_str())
    }

    fn group(&self, group: AnnotationGroup) -> &str {
        match group {
            AnnotationGroup::Comment => self.comment_group.as_str(),
            AnnotationGroup::Highlight => self.highlight_group.as_str(),
            AnnotationGroup::Draw => self.draw_group.as_str(),
        }
    }
}

/// The annotation tools as commands: the toolbar's buttons and flyout rows show their labels and icons, and an app
/// binds chords to them. `New` opens the "New Annotation..." form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui::contract::CommandSet)]
#[command(prefix = "annotate")]
#[non_exhaustive]
pub enum AnnotateCommand {
    /// Select and edit annotations.
    #[command(label = "Select", icon = "pointer")]
    Select,
    /// Place a sticky note.
    #[command(label = "Add a sticky note", icon = "note")]
    Note,
    /// Place a text box.
    #[command(label = "Add a text box", icon = "text")]
    TextBox,
    /// Place a callout.
    #[command(label = "Add a callout", icon = "text")]
    Callout,
    /// Attach a file.
    #[command(label = "Attach file", icon = "link")]
    Attach,
    /// Highlight page text.
    #[command(label = "Highlight", icon = "brush")]
    Highlight,
    /// Underline page text.
    #[command(label = "Underline", icon = "text_underline")]
    Underline,
    /// Strike out page text.
    #[command(label = "Strikethrough", icon = "text_strikethrough")]
    StrikeOut,
    /// Underline page text with a wave.
    #[command(label = "Squiggly underline", icon = "spline")]
    Squiggly,
    /// Mark an insertion point.
    #[command(label = "Insert text", icon = "text_cursor")]
    Caret,
    /// Replace page text.
    #[command(label = "Replace text", icon = "brush_replace")]
    ReplaceText,
    /// Draw freehand.
    #[command(label = "Draw", icon = "pencil")]
    Ink,
    /// Draw a line.
    #[command(label = "Line", icon = "slash")]
    Line,
    /// Draw an arrow.
    #[command(label = "Arrow", icon = "triangle_right")]
    Arrow,
    /// Draw a rectangle.
    #[command(label = "Rectangle", icon = "square")]
    Rectangle,
    /// Draw an oval.
    #[command(label = "Oval", icon = "circle")]
    Oval,
    /// Draw connected lines.
    #[command(label = "Connected lines", icon = "pen_tool")]
    PolyLine,
    /// Draw a polygon.
    #[command(label = "Polygon", icon = "pentagon")]
    Polygon,
    /// Draw a cloud.
    #[command(label = "Cloud", icon = "cloud")]
    Cloud,
    /// Erase ink strokes.
    #[command(label = "Eraser", icon = "eraser")]
    Eraser,
    /// Place a shape or note by numbers.
    #[command(label = "New Annotation...")]
    New,
}

impl AnnotateCommand {
    /// The command that chooses `tool`.
    pub fn of(tool: AnnotationTool) -> Self {
        use AnnotationKind as K;
        match tool {
            AnnotationTool::Select => AnnotateCommand::Select,
            AnnotationTool::Eraser => AnnotateCommand::Eraser,
            AnnotationTool::Draw(kind) => match kind {
                K::Note => AnnotateCommand::Note,
                K::TextBox => AnnotateCommand::TextBox,
                K::Callout => AnnotateCommand::Callout,
                K::Attach => AnnotateCommand::Attach,
                K::Highlight => AnnotateCommand::Highlight,
                K::Underline => AnnotateCommand::Underline,
                K::StrikeOut => AnnotateCommand::StrikeOut,
                K::Squiggly => AnnotateCommand::Squiggly,
                K::Caret => AnnotateCommand::Caret,
                K::ReplaceText => AnnotateCommand::ReplaceText,
                K::Ink => AnnotateCommand::Ink,
                K::Line => AnnotateCommand::Line,
                K::Arrow => AnnotateCommand::Arrow,
                K::Rectangle => AnnotateCommand::Rectangle,
                K::Oval => AnnotateCommand::Oval,
                K::PolyLine => AnnotateCommand::PolyLine,
                K::Polygon => AnnotateCommand::Polygon,
                K::Cloud => AnnotateCommand::Cloud,
            },
        }
    }
}

/// The page text between two page points as quads (four corners each).
pub type TextQuads = Box<dyn Fn(Point, Point) -> Vec<[Point; 4]> + Send>;

/// Acrobat's default border width (Preferences > Commenting, "Line thickness" 1), in page units.
pub const DEFAULT_STROKE_WIDTH: f64 = 1.0;
/// Acrobat's default colours (Preferences > Commenting): red for shapes and notes, yellow for
/// highlights (straight RGB).
pub const DEFAULT_COLOR: [u8; 3] = [0xE0, 0x30, 0x30];
/// The default highlight colour, straight RGB.
pub const HIGHLIGHT_COLOR: [u8; 3] = [0xFF, 0xE0, 0x30];
/// The colours the toolbar offers, Acrobat's quick colours for comments: yellow, orange, red, pink, purple, blue,
/// light blue, green, grey and black. Acrobat's colour menu also has "Other Color", which the app may add.
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
/// The most decimals the width field shows.
const WIDTH_DECIMALS: usize = 1;
/// A highlight lets the text show through.
const HIGHLIGHT_OPACITY: f32 = 0.4;
/// The side of a note, attachment or caret icon on screen, in points. The tool hits and the overlay paints icon
/// kinds at this size whatever the zoom.
pub const ICON_SIDE: f32 = 16.0;
/// The key of the "New Annotation..." command form.
pub const NEW_ANNOTATION_FORM: &str = "newAnnotation";

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

/// What a call knows when it reads a pointer: the pointer in page units, the page-unit sizes of the screen
/// distances the tool tests with, and the tool's words.
struct Reading {
    at: Point,
    tolerance: f64,
    icon_side: f64,
    words: Arc<AnnotateWords>,
}

fn read(cx: &ToolContext<'_>, pos: Pos2) -> Option<Reading> {
    let space = cx.space()?;
    Some(Reading {
        at: space.view.to_doc(pos),
        tolerance: space.view.doc_len(cx.slop),
        icon_side: space.view.doc_len(ICON_SIDE),
        words: cx.words::<AnnotateWords>(),
    })
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

fn srgba([r, g, b]: [u8; 3]) -> Srgba {
    Srgba::rgb(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0)
}

/// `to` made square about `origin` (Shift on a box or oval).
fn squared(origin: Point, to: Point) -> Point {
    let d = to - origin;
    let side = d.x.abs().max(d.y.abs());
    Point::new(origin.x + side * d.x.signum(), origin.y + side * d.y.signum())
}

fn corners(rect: Rect) -> [Point; 4] {
    [Point::new(rect.x0, rect.y0), Point::new(rect.x1, rect.y0), Point::new(rect.x1, rect.y1), Point::new(rect.x0, rect.y1)]
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

/// Acrobat's annotation tools as one canvas tool on craft-ui's tool host.
///
/// - Select: a press on an annotation selects it (Shift adds), a drag moves the selection, a
///   drag on a handle resizes (box kinds; Shift keeps the aspect on a corner) or reshapes
///   (pointwise kinds; Shift snaps to 45 degrees); a press on empty page starts a marquee. Moves,
///   resizes and reshapes are `EditEvent<AnnotationEdit>` gestures; Esc cancels and the release
///   after it is ignored.
/// - A drawing tool: a drag draws the kind (Shift: squares, circles, 45 degree lines); PolyLine and
///   Polygon add a point per click and end with a double click or Enter; Ink records strokes until
///   the tool changes; Note and Text box place at the click and report `Create`, the app opens
///   the composer. Markup kinds select page text through the app's text hit test
///   ([`text_quads`](Self::text_quads)) and report the quads.
/// - Eraser: a drag reports one `Erase` per Ink annotation it crosses, with the strokes in
///   ascending order.
/// - Keys: Delete and Backspace remove the selection, arrows nudge it (one point, Shift ten) as `Move`s.
/// - A double click on a note or text box reports `Open`; the app's page text and selection reach the tool
///   in [`CanvasTool::each_pass`] through the public fields.
/// - New annotations take the tool's [`color`](Self::color) and [`width`](Self::width) (a
///   highlight its [`highlight_color`](Self::highlight_color)); the toolbar's controls set them.
///   Notes, attachments and carets are created with an icon-sized rect and keep that size on
///   screen at any zoom, so hits test them against an icon side worked out each pass.
/// - Locked annotations are selected but never moved.
/// - Without a pointer, the "New Annotation..." form places the shape kinds
///   ([`new_annotation_params`](Self::new_annotation_params), [`create_from_form`](Self::create_from_form)).
pub struct AnnotateTool {
    /// The tool in use.
    pub tool: AnnotationTool,
    /// The page's annotations, refreshed by the app each pass.
    pub annotations: Vec<Annotation>,
    /// The ids of the selected annotations.
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
    last_id: GestureId,
}

impl AnnotateTool {
    /// A tool of kind `tool` working on `page`, with the default colours and width.
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
            last_id: GestureId::default(),
        }
    }

    /// The page text between two points as quads (the app's text selection), for markups.
    #[must_use]
    pub fn text_quads(mut self, quads: impl Fn(Point, Point) -> Vec<[Point; 4]> + Send + 'static) -> Self {
        self.text_quads = Some(Box::new(quads));
        self
    }

    /// The edits the tool reported since the last call, in order.
    pub fn take_edits(&mut self) -> Vec<EditEvent<AnnotationEdit>> {
        std::mem::take(&mut self.edits)
    }

    /// The rows of the "New Annotation..." form (`ParamDialog::command(NEW_ANNOTATION_FORM, title, params)`):
    /// kind, page, position and size in points.
    pub fn new_annotation_params(&self, words: &AnnotateWords) -> Vec<Param> {
        let options: Vec<(String, Label)> =
            FORM_KINDS.iter().map(|&kind| (form_id(kind).to_owned(), Label::from(words.kind(kind).to_owned()))).collect();
        let number = |default: f64| ParamKind::number(Some(0.0), None, default, 1.0, 1);
        vec![
            Param::new("kind", words.form_kind.clone(), ParamKind::choice(options, Some(form_id(AnnotationKind::Rectangle).to_owned()))),
            Param::new("page", words.form_page.clone(), ParamKind::number(Some(1.0), None, (self.page + 1) as f64, 1.0, 0)),
            Param::new("x", words.form_x.clone(), number(0.0)),
            Param::new("y", words.form_y.clone(), number(0.0)),
            Param::new("width", words.form_width.clone(), number(0.0)),
            Param::new("height", words.form_height.clone(), number(0.0)),
        ]
    }

    /// Reports `Create` for the annotation the confirmed "New Annotation..." form describes, in the tool's colour
    /// and width. Returns whether it made one: a form that describes no valid shape makes none.
    pub fn create_from_form(&mut self, fields: &DialogFields, words: &AnnotateWords) -> bool {
        let Some(FieldValue::Text(id)) = fields.get("kind") else { return false };
        let Some(kind) = FORM_KINDS.iter().copied().find(|&kind| form_id(kind) == id) else { return false };
        let number = |key: &str| fields.number(key).filter(|value| value.is_finite());
        let (Some(x), Some(y), Some(width), Some(height)) = (number("x"), number("y"), number("width"), number("height")) else { return false };
        let page = number("page").map_or(self.page, |page| (page.round().max(1.0) as usize) - 1);
        let from = Point::new(x, y);
        let to = Point::new(x + width, y + height);
        let area = Rect::from_points(from, to);
        let mut annotation = match kind {
            AnnotationKind::Note => {
                let side = f64::from(ICON_SIDE);
                let mut note = self.fresh(kind, Rect::from_origin_size(from, (side, side)));
                note.points = vec![from];
                note
            }
            AnnotationKind::Line | AnnotationKind::Arrow => {
                if from == to {
                    return false;
                }
                let mut line = self.fresh(kind, bounds([from, to]));
                line.points = vec![from, to];
                line
            }
            AnnotationKind::Polygon | AnnotationKind::Cloud => {
                if area.width() <= 0.0 || area.height() <= 0.0 {
                    return false;
                }
                let mut shape = self.fresh(kind, area);
                shape.points = corners(area).to_vec();
                shape
            }
            _ => {
                if area.width() <= 0.0 || area.height() <= 0.0 {
                    return false;
                }
                self.fresh(kind, area)
            }
        };
        annotation.page = page;
        self.create(annotation, words);
        true
    }
}

/// The kinds the "New Annotation..." form places.
const FORM_KINDS: [AnnotationKind; 8] = [
    AnnotationKind::Rectangle,
    AnnotationKind::Oval,
    AnnotationKind::Line,
    AnnotationKind::Arrow,
    AnnotationKind::Polygon,
    AnnotationKind::Cloud,
    AnnotationKind::Note,
    AnnotationKind::TextBox,
];

fn form_id(kind: AnnotationKind) -> &'static str {
    match kind {
        AnnotationKind::Rectangle => "rectangle",
        AnnotationKind::Oval => "oval",
        AnnotationKind::Line => "line",
        AnnotationKind::Arrow => "arrow",
        AnnotationKind::Polygon => "polygon",
        AnnotationKind::Cloud => "cloud",
        AnnotationKind::Note => "note",
        _ => "textBox",
    }
}

impl AnnotateTool {
    fn next_id(&mut self) -> GestureId {
        self.last_id = self.last_id.next();
        self.last_id
    }

    /// Reports `edit` as a step of its own, named `label`.
    fn push(&mut self, edit: AnnotationEdit, label: &str) {
        let id = self.next_id();
        self.edits.push(EditEvent::set(id, Mixed::Mixed, edit, Arc::from(label), None));
    }

    /// Applies `change` to the selection and reports it.
    fn select(&mut self, change: SelectionChange<AnnotationId>, words: &AnnotateWords) {
        match &change {
            SelectionChange::Set(ids) => self.selected.clone_from(ids),
            SelectionChange::Toggle(id) => match self.selected.iter().position(|selected| selected == id) {
                Some(at) => {
                    self.selected.remove(at);
                }
                None => self.selected.push(*id),
            },
            SelectionChange::Clear => self.selected.clear(),
            _ => {}
        }
        self.push(AnnotationEdit::Selection(change), words.edit_select.as_str());
    }

    /// Replaces the selection with `ids` (a marquee or a plain click), or clears it.
    fn replace_selection(&mut self, ids: Vec<AnnotationId>, words: &AnnotateWords) {
        let change = if ids.is_empty() { SelectionChange::Clear } else { SelectionChange::Set(ids) };
        self.select(change, words);
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

    fn create(&mut self, annotation: Annotation, words: &AnnotateWords) {
        self.push(AnnotationEdit::Create(annotation), words.edit_create.as_str());
    }

    /// Ends the pending Ink strokes as one annotation.
    fn flush_ink(&mut self, words: &AnnotateWords) {
        if self.strokes.is_empty() {
            return;
        }
        let strokes = std::mem::take(&mut self.strokes);
        let mut annotation = self.fresh(AnnotationKind::Ink, bounds(strokes.iter().flatten().copied()));
        annotation.strokes = strokes;
        self.create(annotation, words);
    }

    /// Ends the pending PolyLine, Polygon or Cloud once it has enough points.
    fn finish_poly(&mut self, kind: AnnotationKind, words: &AnnotateWords) {
        let points = std::mem::take(&mut self.poly);
        let least = if kind == AnnotationKind::PolyLine { 2 } else { 3 };
        if points.len() >= least {
            let mut annotation = self.fresh(kind, bounds(points.iter().copied()));
            annotation.points = points;
            self.create(annotation, words);
        }
    }

    fn is_poly(&self) -> bool {
        matches!(self.tool, AnnotationTool::Draw(AnnotationKind::PolyLine | AnnotationKind::Polygon | AnnotationKind::Cloud))
    }

    fn press_select(&mut self, pointer: &ToolPointer, reading: &Reading) -> ToolAction {
        let shift = pointer.mods.shift;
        let found = hit(&self.annotations, &self.selected, reading.at, reading.tolerance, reading.icon_side);
        let locked = |id: AnnotationId, list: &[Annotation]| list.iter().any(|a| a.id == id && a.locked);
        let id = self.next_id();
        let edit = |change: Change| Gesture::Edit { id, begun: false, origin: reading.at, change };
        match found {
            Some(AnnotationHit::Handle(target, AnnotationHandle::Box(hx, hy))) => {
                let from = self.annotations.iter().find(|a| a.id == target).map_or(Rect::ZERO, |a| a.rect);
                self.gesture = Some(edit(Change::Resize { target, handle: (hx, hy), from }));
            }
            Some(AnnotationHit::Handle(target, AnnotationHandle::Point(index))) => {
                let from = self.annotations.iter().find(|a| a.id == target).map(|a| a.points.clone()).unwrap_or_default();
                self.gesture = Some(edit(Change::Reshape { target, index, from }));
            }
            Some(AnnotationHit::Body(target)) => {
                let selected = self.selected.contains(&target);
                if pointer.clicks >= 2 {
                    // A second click on a note or text box asks the app to open its text.
                    let opens = self.annotations.iter().any(|a| a.id == target && matches!(a.kind, AnnotationKind::Note | AnnotationKind::TextBox));
                    if opens {
                        if !selected {
                            self.select(SelectionChange::Set(vec![target]), &reading.words);
                        }
                        self.push(AnnotationEdit::Open(target), reading.words.edit_open.as_str());
                        return ToolAction::Done;
                    }
                }
                if shift {
                    self.select(SelectionChange::Toggle(target), &reading.words);
                } else if !selected {
                    self.select(SelectionChange::Set(vec![target]), &reading.words);
                }
                let ids: Vec<AnnotationId> = self.selected.iter().copied().filter(|s| !locked(*s, &self.annotations)).collect();
                if !ids.is_empty() {
                    self.gesture = Some(edit(Change::Move { ids }));
                }
            }
            None => {
                self.gesture = Some(Gesture::Marquee { origin: reading.at, current: reading.at, additive: shift });
            }
        }
        ToolAction::Claimed
    }

    fn press_draw(&mut self, kind: AnnotationKind, pointer: &ToolPointer, reading: &Reading) -> ToolAction {
        use AnnotationKind as K;
        let at = reading.at;
        match kind {
            K::Note | K::Attach | K::Caret => {
                let mut annotation = self.fresh(kind, Rect::from_origin_size(at, (reading.icon_side, reading.icon_side)));
                annotation.points = vec![at];
                self.create(annotation, &reading.words);
                ToolAction::Done
            }
            K::PolyLine | K::Polygon | K::Cloud => {
                if pointer.clicks >= 2 {
                    // The double click's first press already added its point.
                    self.finish_poly(kind, &reading.words);
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
                let end = if shift { turn_to_45(origin, current) } else { current };
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

    fn editable(&self) -> Vec<AnnotationId> {
        self.selected.iter().copied().filter(|id| self.annotations.iter().any(|a| a.id == *id && !a.locked)).collect()
    }
}

/// The value a move, resize or reshape gesture reports with the pointer at `at`.
fn edit_value(change: &Change, origin: Point, at: Point, mods: Mods) -> AnnotationEdit {
    match change {
        Change::Move { ids } => AnnotationEdit::Move { ids: ids.clone(), delta: at - origin },
        Change::Resize { target, handle, from } => {
            let aspect = (mods.shift && from.height() > 0.0).then(|| from.width() / from.height());
            AnnotationEdit::Resize { id: *target, rect: resized(*from, *handle, at - origin, aspect) }
        }
        Change::Reshape { target, index, from } => AnnotationEdit::Reshape { id: *target, points: reshaped(from, *index, at, mods.shift) },
    }
}

fn edit_label<'a>(change: &Change, words: &'a AnnotateWords) -> &'a str {
    match change {
        Change::Move { .. } => words.edit_move.as_str(),
        Change::Resize { .. } => words.edit_resize.as_str(),
        Change::Reshape { .. } => words.edit_reshape.as_str(),
    }
}

impl CanvasTool for AnnotateTool {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        if self.tool != self.last_tool {
            self.flush_ink(&cx.words::<AnnotateWords>());
            self.poly.clear();
            self.last_tool = self.tool;
        }
    }

    fn cursor(&self, _cx: &ToolContext<'_>) -> Cursor {
        match self.tool {
            AnnotationTool::Select => Cursor::System(CursorIcon::Default),
            AnnotationTool::Draw(_) | AnnotationTool::Eraser => Cursor::System(CursorIcon::Crosshair),
        }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.gesture = None;
        let Some(reading) = read(cx, pointer.pos) else { return ToolAction::Ignored };
        match self.tool {
            AnnotationTool::Select => self.press_select(pointer, &reading),
            AnnotationTool::Draw(kind) => self.press_draw(kind, pointer, &reading),
            AnnotationTool::Eraser => {
                self.gesture = Some(Gesture::Erase { path: vec![reading.at], crossed: Vec::new() });
                ToolAction::Claimed
            }
        }
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some(reading) = read(cx, pointer.pos) else { return };
        let Some(gesture) = self.gesture.as_mut() else { return };
        let mut events = Vec::new();
        match gesture {
            Gesture::Edit { id, begun, origin, change } => {
                if !*begun {
                    *begun = true;
                    events.push(EditEvent::begin(*id, Mixed::Mixed, Arc::from(edit_label(change, &reading.words)), None));
                }
                events.push(EditEvent::update(*id, edit_value(change, *origin, reading.at, pointer.mods)));
            }
            Gesture::Marquee { current, .. } | Gesture::Shape { current, .. } | Gesture::Markup { current, .. } => *current = reading.at,
            Gesture::Stroke(points) => points.push(reading.at),
            Gesture::Erase { path, crossed } => {
                let last = path.last().copied().unwrap_or(reading.at);
                for a in self.annotations.iter().filter(|a| a.kind == AnnotationKind::Ink && !a.locked) {
                    for (i, stroke) in a.strokes.iter().enumerate() {
                        let reach = reading.tolerance + a.stroke_width / 2.0;
                        let hits = stroke.windows(2).any(|w| segments_cross(last, reading.at, w[0], w[1], reach))
                            || stroke.len() == 1 && (stroke[0] - reading.at).hypot() <= reading.tolerance;
                        if hits && !crossed.contains(&(a.id, i)) {
                            crossed.push((a.id, i));
                        }
                    }
                }
                path.push(reading.at);
            }
        }
        self.hover = Some(reading.at);
        self.edits.extend(events);
        cx.request_repaint();
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some(gesture) = self.gesture.take() else { return };
        let Some(reading) = read(cx, pointer.pos) else { return };
        let words = Arc::clone(&reading.words);
        let mods = pointer.mods;
        match gesture {
            Gesture::Edit { id, begun, origin, change } => {
                if begun {
                    self.edits.push(EditEvent::commit(id, Mixed::Mixed, edit_value(&change, origin, reading.at, mods)));
                }
            }
            Gesture::Marquee { origin, additive, .. } => {
                let area = Rect::from_points(origin, reading.at);
                let inside: Vec<AnnotationId> = if area.width() == 0.0 && area.height() == 0.0 {
                    Vec::new()
                } else {
                    self.annotations
                        .iter()
                        .filter(|a| {
                            let extent = a.extent(reading.icon_side);
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
                self.replace_selection(ids, &words);
            }
            Gesture::Shape { kind, origin, .. } => {
                if let Some(annotation) = self.shape_annotation(kind, origin, reading.at, mods.shift) {
                    self.create(annotation, &words);
                }
            }
            Gesture::Markup { kind, origin, .. } => {
                if let Some(annotation) = self.markup_annotation(kind, origin, reading.at) {
                    self.create(annotation, &words);
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
                    self.push(AnnotationEdit::Erase { id, strokes }, words.edit_erase.as_str());
                }
            }
        }
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.hover = cx.space().map(|space| space.view.to_doc(pointer.pos));
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        if let Some(Gesture::Edit { id, begun: true, .. }) = self.gesture.take() {
            self.edits.push(EditEvent::cancel(id, reason, None));
        }
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        let words = cx.words::<AnnotateWords>();
        match key {
            ToolKey::Cancel if !self.poly.is_empty() || !self.strokes.is_empty() => {
                self.poly.clear();
                self.strokes.clear();
                true
            }
            ToolKey::Submit if self.is_poly() && !self.poly.is_empty() => {
                if let AnnotationTool::Draw(kind) = self.tool {
                    self.finish_poly(kind, &words);
                }
                true
            }
            ToolKey::Submit if !self.strokes.is_empty() => {
                self.flush_ink(&words);
                true
            }
            ToolKey::Delete { .. } if self.tool == AnnotationTool::Select => {
                let ids = self.editable();
                if ids.is_empty() {
                    return false;
                }
                self.push(AnnotationEdit::Delete(ids), words.edit_delete.as_str());
                true
            }
            ToolKey::Nudge { direction, mods, .. } if self.tool == AnnotationTool::Select => {
                let ids = self.editable();
                let Some(space) = cx.space() else { return false };
                if ids.is_empty() || !(mods.is_none() || *mods == Mods::SHIFT) {
                    return false;
                }
                let step = if mods.shift { NUDGE_FAR } else { NUDGE };
                let along = space.view.unmap_vec(*direction);
                if along.hypot() == 0.0 {
                    return false;
                }
                self.push(AnnotationEdit::Move { ids, delta: along / along.hypot() * step }, words.edit_move.as_str());
                true
            }
            _ => false,
        }
    }

    fn paint(&self, draw: &Draw<'_>, cx: &ToolContext<'_>) {
        let Some(space) = cx.space() else { return };
        let view = space.view;
        let screen = |points: &[Point]| -> Vec<Pos2> { points.iter().map(|&p| view.to_screen(p)).collect() };
        let outline = |points: Vec<Pos2>, closed: bool, color: Color32| {
            let mut path = points;
            if closed && let Some(&first) = path.first() {
                path.push(first);
            }
            draw.data_stroke(&PaintShape::Path(path), StrokeWidth::Medium, DataColor::new(color));
        };
        let drawn = |kind: AnnotationKind| color32(&self.fresh(kind, Rect::ZERO));
        let shift = cx.mods.shift;
        match &self.gesture {
            Some(Gesture::Marquee { origin, current, .. }) => {
                cx.marching_ants(&screen(&corners(Rect::from_points(*origin, *current))), true, 0.0);
            }
            Some(Gesture::Shape { kind, origin, current }) => {
                let color = drawn(*kind);
                match kind {
                    AnnotationKind::Line | AnnotationKind::Arrow => {
                        let end = if shift { turn_to_45(*origin, *current) } else { *current };
                        outline(screen(&[*origin, end]), false, color);
                    }
                    AnnotationKind::Oval => {
                        let end = if shift { squared(*origin, *current) } else { *current };
                        outline(screen(&ellipse_points(Rect::from_points(*origin, end))), true, color);
                    }
                    _ => {
                        let end = if shift { squared(*origin, *current) } else { *current };
                        outline(screen(&corners(Rect::from_points(*origin, end))), true, color);
                    }
                }
            }
            Some(Gesture::Markup { kind, origin, current }) => {
                if let Some(quads) = self.text_quads.as_ref() {
                    for quad in quads(*origin, *current) {
                        outline(screen(&quad), true, drawn(*kind));
                    }
                }
            }
            Some(Gesture::Stroke(points)) => outline(screen(points), false, drawn(AnnotationKind::Ink)),
            Some(Gesture::Erase { path, .. }) => cx.marching_ants(&screen(path), false, 0.0),
            Some(Gesture::Edit { .. }) | None => {}
        }
        for stroke in &self.strokes {
            outline(screen(stroke), false, drawn(AnnotationKind::Ink));
        }
        if let AnnotationTool::Draw(kind) = self.tool
            && self.is_poly()
            && !self.poly.is_empty()
        {
            let mut path = self.poly.clone();
            path.extend(self.hover);
            outline(screen(&path), false, drawn(kind));
        }
    }

    fn takes(&self, button: PointerButton, _mods: Mods) -> bool {
        button == PointerButton::Primary
    }

    fn command_form(&self) -> ToolPath {
        match self.tool {
            AnnotationTool::Select => {
                ToolPath::Command("the arrow keys nudge and Delete removes the selection; the New Annotation... command places shapes")
            }
            AnnotationTool::Eraser | AnnotationTool::Draw(AnnotationKind::Ink) => ToolPath::Freehand,
            AnnotationTool::Draw(kind) if kind.is_markup() => ToolPath::Freehand,
            AnnotationTool::Draw(_) => ToolPath::Command("the New Annotation... command"),
        }
    }
}

/// The ellipse inscribed in `rect`, as a closed run of page points.
fn ellipse_points(rect: Rect) -> Vec<Point> {
    const STEPS: u32 = 48;
    let (c, rx, ry) = (rect.center(), rect.width() / 2.0, rect.height() / 2.0);
    (0..STEPS)
        .map(|i| {
            let t = std::f64::consts::TAU * f64::from(i) / f64::from(STEPS);
            Point::new(c.x + rx * t.cos(), c.y + ry * t.sin())
        })
        .collect()
}

/// An arrow head's length and a cloud scallop's width, in stroke widths.
const ARROW_HEAD: f32 = 6.0;
const CLOUD_BUMP: f32 = 8.0;
/// A squiggle's wavelength, in stroke widths.
const WAVE: f32 = 4.0;
/// The points a cloud scallop is drawn with.
const SCALLOP_STEPS: u32 = 8;

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
            for s in 0..SCALLOP_STEPS {
                let t = std::f32::consts::PI * s as f32 / SCALLOP_STEPS as f32;
                // Rotates from the segment's start, over the outward side, to its end.
                let along = (from - centre).normalized();
                out.push(centre + along * radius * t.cos() + normal * radius * t.sin());
            }
        }
    }
    out
}

fn screen_rect(rect: Rect, to_screen: &impl Fn(Point) -> Pos2) -> egui::Rect {
    egui::Rect::from_two_pos(to_screen(Point::new(rect.x0, rect.y0)), to_screen(Point::new(rect.x1, rect.y1)))
}

/// The screen rect that holds `points`.
fn around(points: impl IntoIterator<Item = Pos2>) -> egui::Rect {
    points.into_iter().fold(egui::Rect::NOTHING, |r, p| r.union(egui::Rect::from_min_max(p, p)))
}

/// The outline of the icon of a note, an attachment or a caret in the unit square, as open or closed runs.
fn pictogram(kind: AnnotationKind) -> Vec<(Vec<(f32, f32)>, bool)> {
    match kind {
        AnnotationKind::Note => vec![(vec![(0.1, 0.1), (0.9, 0.1), (0.9, 0.7), (0.5, 0.7), (0.25, 0.92), (0.25, 0.7), (0.1, 0.7)], true)],
        AnnotationKind::Attach => vec![(vec![(0.72, 0.15), (0.3, 0.6), (0.4, 0.85), (0.82, 0.4), (0.62, 0.12), (0.2, 0.55)], false)],
        _ => vec![(vec![(0.15, 0.85), (0.5, 0.15), (0.85, 0.85)], false), (vec![(0.5, 0.15), (0.5, 0.95)], false)],
    }
}

fn paint_annotation(painter: &ContentPainterCx<'_>, draw: &Draw<'_>, a: &Annotation, to_screen: &impl Fn(Point) -> Pos2, text_size: f32) {
    use AnnotationKind as K;
    let scale = (to_screen(Point::new(1.0, 0.0)) - to_screen(Point::ZERO)).length();
    let color = color32(a);
    let width = (a.stroke_width as f32 * scale).max(1.0 / painter.pixels_per_point());
    let stroke = Stroke::new(width, color);
    let map = |points: &[Point]| -> Vec<Pos2> { points.iter().map(|&p| to_screen(p)).collect() };
    let raw = &painter.painter;
    let quads = || -> Vec<egui::Rect> { a.points.chunks(4).map(|quad| around(quad.iter().map(|&p| to_screen(p)))).collect() };
    match a.kind {
        K::Highlight => {
            for r in quads() {
                raw.rect_filled(r, 0.0, color);
            }
        }
        K::Underline | K::StrikeOut | K::Squiggly | K::ReplaceText => {
            for r in quads() {
                let y = if matches!(a.kind, K::StrikeOut | K::ReplaceText) { r.center().y } else { r.bottom() };
                if a.kind == K::Squiggly {
                    let (amplitude, length) = (width, width * WAVE);
                    let steps = (r.width() / length * 2.0).ceil().max(1.0) as usize;
                    let wave: Vec<Pos2> = (0..=steps)
                        .map(|i| egui::pos2(r.left() + r.width() * i as f32 / steps as f32, y + if i % 2 == 0 { -amplitude } else { amplitude }))
                        .collect();
                    raw.add(Shape::line(wave, stroke));
                } else {
                    raw.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], stroke);
                }
            }
        }
        K::Line | K::Arrow | K::PolyLine => {
            let path = map(&a.points);
            if let [.., from, to] = path[..] {
                raw.add(Shape::line(path.clone(), stroke));
                if a.kind == K::Arrow {
                    let back = (from - to).normalized() * width * ARROW_HEAD;
                    for turn in [0.5_f32, -0.5] {
                        let (s, c) = turn.sin_cos();
                        raw.line_segment([to, to + egui::vec2(back.x * c - back.y * s, back.x * s + back.y * c)], stroke);
                    }
                }
            }
        }
        K::Polygon => {
            raw.add(Shape::closed_line(map(&a.points), stroke));
        }
        K::Cloud => {
            let corners = map(&a.points);
            if corners.len() > 2 {
                raw.add(Shape::closed_line(scalloped(&corners, width * CLOUD_BUMP), stroke));
            }
        }
        K::Ink => {
            for s in &a.strokes {
                raw.add(Shape::line(map(s), stroke));
            }
        }
        K::Rectangle => {
            raw.add(Shape::closed_line(map(&corners(a.rect)), stroke));
        }
        K::Oval => {
            raw.add(Shape::closed_line(map(&ellipse_points(a.rect)), stroke));
        }
        K::TextBox | K::Callout => {
            let r = screen_rect(a.rect, to_screen);
            raw.rect_stroke(r, 0.0, stroke, StrokeKind::Inside);
            // The text is document-scaled: the body size at zoom 1, wrapped at the box's width.
            let run = DocRun::new(a.text.as_str(), FontRef::proportional(), text_size * scale, DataColor::new(color));
            draw.clipped(r, |draw| {
                draw.doc(r, &DocText::new(&[run]));
            });
        }
        K::Note | K::Attach | K::Caret => {
            let slot = egui::Rect::from_min_size(to_screen(Point::new(a.rect.x0, a.rect.y0)), egui::vec2(ICON_SIDE, ICON_SIDE));
            for (run, closed) in pictogram(a.kind) {
                let points: Vec<Pos2> = run.iter().map(|&(x, y)| slot.min + egui::vec2(x * slot.width(), y * slot.height())).collect();
                raw.add(if closed { Shape::closed_line(points, stroke) } else { Shape::line(points, stroke) });
            }
        }
    }
}

/// The style of the annotation overlay.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct AnnotationOverlayStyle {
    #[style(metrics)]
    metrics: Metrics,
    handle: HandleLook,
    line: StyleColor,
    text_size: f32,
}

/// Draws a page's annotations over it: markups as translucent quads (highlight at the annotation's opacity, lines
/// for underline and strike-out, a wave for squiggly), shapes and ink with their stroke, notes as their icon, text
/// boxes with their text, and the handles of the selected ones. A canvas control shown with `show_at` at the
/// canvas's rect and mapped through its view. It takes no pointer: the tool does.
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
#[component(canvas_control, content_painter)]
pub struct AnnotationOverlay<'a> {
    #[common]
    common: Common,
    annotations: &'a [Annotation],
    selected: &'a [AnnotationId],
}

impl<'a> AnnotationOverlay<'a> {
    /// An overlay named `label` for `annotations`, with `selected` drawing handles.
    pub fn new(label: impl Into<Label>, annotations: &'a [Annotation], selected: &'a [AnnotationId]) -> Self {
        AnnotationOverlay { common: Common::new(), annotations, selected }.label(label)
    }
}

impl Component for AnnotationOverlay<'_> {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    const INPUT: Input = Input::Display;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = AnnotationOverlayStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: AnnotationOverlayStyleKey) -> AnnotationOverlayStyle {
        AnnotationOverlayStyle {
            metrics: Metrics::new(),
            handle: cx.handle(),
            line: cx.color(cx.tokens.color.overlay.line),
            text_size: cx.tokens.typography.body.size,
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> egui::Vec2 {
        egui::Vec2::ZERO
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(space) = canvas_space(&cx.env) else { return };
        let view = space.view;
        let to_screen = |p: Point| view.to_screen(p);
        let painter = ui.content_painter(cx);
        let draw = ui.draw();
        for a in self.annotations {
            paint_annotation(&painter, &draw, a, &to_screen, cx.style.text_size);
        }
        let outline = Stroke::new(cx.style.handle.stroke_width, cx.style.line.egui());
        let raw = &painter.painter;
        for a in self.annotations.iter().filter(|a| self.selected.contains(&a.id)) {
            if a.kind.is_markup() {
                for quad in a.points.chunks(4) {
                    raw.rect_stroke(around(quad.iter().map(|&p| to_screen(p))), 0.0, outline, StrokeKind::Outside);
                }
            } else if a.kind.is_pointwise() && !a.points.is_empty() {
                raw.add(Shape::line(a.points.iter().map(|&p| to_screen(p)).collect(), outline));
            } else {
                raw.rect_stroke(screen_rect(a.rect, &to_screen), 0.0, outline, StrokeKind::Outside);
            }
            for (handle, at) in handles(a) {
                let shape = match handle {
                    AnnotationHandle::Box(..) => HandleShape::Square,
                    AnnotationHandle::Point(_) => HandleShape::Circle,
                };
                raw.extend(Handle::shapes(&cx.style.handle, shape, to_screen(at), &StateSet::default()));
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// What [`AnnotationToolbar::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct AnnotationToolbarOutput {
    /// The bar's response.
    pub response: Response,
    /// The tool, the colour or the width changed.
    pub changed: bool,
}

impl Outcome for AnnotationToolbarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// Whether the colour palette popover is open.
#[derive(Clone, Copy, Debug, Default)]
struct PaletteOpen(bool);

/// Acrobat's quick bar: Select, then the three groups (Comment, Highlight, Draw), each a button showing its
/// last-used tool with a flyout of the group's tools, then the colour well and the line width of the tool's new
/// annotations ([`AnnotateTool::color`], [`AnnotateTool::width`]); both are disabled for tools that draw nothing
/// (Select, Eraser) and the width for markups. The colour well opens a palette popover.
#[derive(craft_ui::contract::Pattern)]
pub struct AnnotationToolbar<'a> {
    tool: &'a mut AnnotateTool,
}

impl<'a> AnnotationToolbar<'a> {
    /// A toolbar for `tool`, whose `tool`, `color`, `highlight_color` and `width` it edits.
    pub fn new(tool: &'a mut AnnotateTool) -> Self {
        Self { tool }
    }

    /// Shows the bar.
    pub fn show(self, ui: &mut Ui) -> AnnotationToolbarOutput {
        let words = Env::of(ui).words::<AnnotateWords>();
        let annotate = self.tool;
        let before = (annotate.tool, annotate.color, annotate.highlight_color, annotate.width);
        let select = [RailTool::new(AnnotationTool::Select, AnnotateCommand::Select)];
        let slots: Vec<ToolSlot<AnnotationTool>> = AnnotationGroup::ALL
            .iter()
            .enumerate()
            .map(|(n, group)| ToolSlot::new(n as SlotId, group.tools().iter().map(|&tool| RailTool::new(tool, AnnotateCommand::of(tool))).collect()))
            .collect();
        let bar = Bar::new(words.toolbar.clone(), BarKind::Tool).show(ui, |ui, body| {
            let last = body.memory::<LastUsed<AnnotationTool>>().clone();
            let current = annotate.tool;
            let out =
                FlyoutButton::new(&select, &AnnotationTool::Select).label(words.select.clone()).selected(current == AnnotationTool::Select).show(ui);
            if let Some(tool) = out.select {
                annotate.tool = tool;
            }
            for (slot, group) in slots.iter().zip(AnnotationGroup::ALL) {
                let tools: Vec<&RailTool<AnnotationTool>> = slot.tools.iter().collect();
                let shown = shown_tool(&current, slot.id, &tools, &last, |_| true).map_or(current, |tool| tool.id);
                let out = FlyoutButton::new(&slot.tools, &shown)
                    .label(words.group(group).to_owned())
                    .selected(slot.tools.iter().any(|tool| tool.id == current))
                    .show(ui);
                if let Some(tool) = out.select {
                    annotate.tool = tool;
                }
            }
            if let Some(next) = remembered(&slots, &annotate.tool, &last) {
                *body.memory::<LastUsed<AnnotationTool>>() = next;
            }
            style_controls(ui, body, annotate, &words);
        });
        let changed = before != (annotate.tool, annotate.color, annotate.highlight_color, annotate.width);
        AnnotationToolbarOutput { response: bar.output.response, changed }
    }
}

/// The colour well with its palette popover, and the line width field, for the current tool.
fn style_controls(ui: &mut Ui, body: &mut BodyCx<'_>, annotate: &mut AnnotateTool, words: &AnnotateWords) {
    let draws = matches!(annotate.tool, AnnotationTool::Draw(_));
    let highlight = annotate.tool == AnnotationTool::Draw(AnnotationKind::Highlight);
    let markup = matches!(annotate.tool, AnnotationTool::Draw(kind) if kind.is_markup());
    let current = if highlight { annotate.highlight_color } else { annotate.color };

    let well = Swatch::new(words.color.clone(), srgba(current)).enabled(draws).show(ui);
    let mut open = body.memory::<PaletteOpen>().0;
    if well.clicked {
        open = !open;
    }
    let shown: Mixed<u8> = PALETTE.iter().position(|&color| color == current).map_or(Mixed::Mixed, |at| Mixed::Same(at as u8));
    let mut pick = shown.clone();
    Popover::new(words.palette.clone(), Bind::bound(&mut open)).opened_by(&well.response).show(ui, |ui, _| {
        RadioGroup::new(words.palette.clone(), Bind::bound_mixed(&mut pick)).show(ui, |ui, _| {
            Row::new().show(ui, |ui, _| {
                for (n, color) in PALETTE.into_iter().enumerate() {
                    Swatch::new(words.palette_color(n).to_owned(), srgba(color)).member(n as u8).show(ui);
                }
            });
        });
    });
    if pick != shown
        && let Mixed::Same(at) = pick
    {
        let color = PALETTE[usize::from(at)];
        if highlight {
            annotate.highlight_color = color;
        } else {
            annotate.color = color;
        }
        open = false;
    }
    body.memory::<PaletteOpen>().0 = open;

    let mut width = Some(annotate.width);
    NumberField::new(words.width.clone(), Bind::bound(&mut width))
        .range(WIDTH_RANGE)
        .step(1.0)
        .commit_decimals(WIDTH_DECIMALS)
        .enabled(draws && !markup)
        .show(ui);
    if let Some(value) = width {
        annotate.width = value.clamp(*WIDTH_RANGE.start(), *WIDTH_RANGE.end());
    }
}
