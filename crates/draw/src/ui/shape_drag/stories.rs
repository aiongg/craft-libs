//! Stories of the Shape tools and Create Shape. A fake
//! canvas view stands for the document and the tool runs in a `ToolHost` as an app runs it. Sample
//! words are story words; the plain strings are the status lines.
//!
//! - `primitives/shape-drag-tool/draw`: an interactive canvas for every kind and both rule sets (Photoshop and
//!   Illustrator): ⇧ squares the box or snaps a line, ⌥ draws from the centre, Space moves the shape
//!   being drawn, and a click or Enter opens Create Shape at that point. A committed drag or the
//!   dialog's answer adds an outline to the page.
//! - `patterns/create-shape-dialog/dialog`: the dialog alone for Rectangle (four radii), Polygon and Star (a
//!   star ratio), with the ruler unit of its size fields and a bad size on request.

use egui::{Painter, Shape, Stroke, Ui};
use kurbo::{PathEl, Point};

use super::{CreateShapeDialog, CreateShapeOptions, ShapeDragEvent, ShapeDragOptions, ShapeDragRules, ShapeDragTool, ShapeKind};
use crate::geometry::shape::ShapeGeometry;
use craft_ui::canvas::{CanvasView, ToolHost};
use craft_ui::controls::Button;
use craft_ui::dialog::{DialogHost, DialogResult, GuideUnits};
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::localized::units::LengthUnit;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, drive, lock, shared};
use craft_ui::testing::canvas_story::{ViewArgs, document};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ShapeDragTool", "CreateShapeDialog"];

craft_ui::words! {
    /// The captions of the shape drag stories, English by default.
    #[non_exhaustive]
    pub(crate) struct ShapeDragStoryLabels {
        keys: "shapeDragStory.keys" = "Drag to draw. Shift squares, Alt draws from the center, Space moves, click opens Create Shape.";
        created: "shapeDragStory.created" = "Created {kind}";
    }
}

/// Flattening tolerance for the outlines, in document units.
const TOLERANCE: f64 = 0.25;

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/shape-drag-tool/draw",
            Level::Primitives,
            "ShapeDragTool",
            "Drag",
            DragArgs {
                view: ViewArgs::default(),
                kind: KindArg::Rectangle,
                rules: RulesArg::Photoshop,
                corner_radius: 0.0,
                sides: 5,
                star_ratio: 0.5,
                line_weight: 3.0,
            },
            show_drag
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/create-shape-dialog/dialog",
            Level::Patterns,
            "CreateShapeDialog",
            "Create Shape",
            DialogArgs { open: true, kind: DialogKind::Rectangle, ruler: RulerArg::Pixels, bad_size: false },
            show_dialog
        ),
    ]
}

// --- shared parts ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum KindArg {
    Rectangle,
    #[choice(name = "Rounded Rectangle")]
    RoundedRectangle,
    Ellipse,
    Triangle,
    Polygon,
    Star,
    Line,
}

impl KindArg {
    fn kind(self) -> ShapeKind {
        match self {
            KindArg::Rectangle => ShapeKind::Rectangle,
            KindArg::RoundedRectangle => ShapeKind::RoundedRectangle,
            KindArg::Ellipse => ShapeKind::Ellipse,
            KindArg::Triangle => ShapeKind::Triangle,
            KindArg::Polygon => ShapeKind::Polygon,
            KindArg::Star => ShapeKind::Star,
            KindArg::Line => ShapeKind::Line,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RulesArg {
    Photoshop,
    Illustrator,
}

impl RulesArg {
    fn rules(self) -> ShapeDragRules {
        match self {
            RulesArg::Photoshop => ShapeDragRules::PHOTOSHOP,
            RulesArg::Illustrator => ShapeDragRules::ILLUSTRATOR,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RulerArg {
    Pixels,
    Inches,
    Millimetres,
}

impl RulerArg {
    fn unit(self) -> LengthUnit {
        match self {
            RulerArg::Pixels => LengthUnit::Pixels,
            RulerArg::Inches => LengthUnit::Inches,
            RulerArg::Millimetres => LengthUnit::Millimetres,
        }
    }
}

/// The ruler the fake document measures in: 640 × 400 pixels at 96 per inch, as the guide dialog
/// stories use it.
fn units(ruler: RulerArg) -> GuideUnits {
    let doc = craft_ui::testing::canvas_story::DOC;
    GuideUnits { ruler: ruler.unit(), width_px: doc.width(), height_px: doc.height(), pixels_per_inch: 96.0, points_per_inch: 72.0 }
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// Draws the outline of `geometry` on the page.
fn outline(painter: &Painter, view: &CanvasView, geometry: &ShapeGeometry, stroke: Stroke) {
    let mut points: Vec<egui::Pos2> = Vec::new();
    kurbo::flatten(geometry.to_path(TOLERANCE).elements().iter().copied(), TOLERANCE, |element| match element {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => points.push(view.to_screen(p)),
        PathEl::ClosePath => {
            painter.add(Shape::closed_line(std::mem::take(&mut points), stroke));
        }
        _ => {}
    });
    if points.len() > 1 {
        painter.add(Shape::line(points, stroke));
    }
}

// --- drag ---

#[derive(StoryArgs, Debug, PartialEq)]
struct DragArgs {
    #[arg(flatten)]
    view: ViewArgs,
    kind: KindArg,
    /// Photoshop draws polygons and stars in a box; Illustrator draws them from the center, with
    /// the arrow keys changing the points and Ctrl holding a star's inner radius.
    rules: RulesArg,
    #[arg(name = "corner radius", range = 0.0..=100.0)]
    corner_radius: f64,
    #[arg(range = 3..=100)]
    sides: u32,
    #[arg(name = "star ratio", range = 0.01..=1.0)]
    star_ratio: f64,
    #[arg(name = "line weight", range = 1.0..=50.0)]
    line_weight: f64,
}

/// What the drag story keeps: the tool, the outlines drawn so far, the Create Shape dialog and where
/// a click asked for it.
#[derive(Default)]
struct DragKept {
    tool: ShapeDragTool,
    shapes: Vec<ShapeGeometry>,
    dialogs: DialogHost,
    note: String,
}

fn show_drag(ui: &mut Ui, args: &mut DragArgs) {
    let words = ShapeDragStoryLabels::of(ui);
    let story = StoryLabels::of(ui);
    let canvas = document(ui, args.view, &story.document);
    let state = shared(ui, "drag", DragKept::default);
    let mut kept = lock(&state);
    let kept = &mut *kept;
    let options = ShapeDragOptions {
        kind: args.kind.kind(),
        corner_radius: args.corner_radius,
        sides: args.sides,
        star_ratio: args.star_ratio,
        line_weight: args.line_weight,
    };
    kept.tool.options(options).rules(args.rules.rules());
    let _ = ToolHost::new(canvas.id).document(craft_ui::testing::canvas_story::DOC.size()).show(ui, &canvas.view, &canvas.response, &mut kept.tool);

    let ctx = ui.ctx().clone();
    for event in kept.tool.take_events() {
        match event {
            ShapeDragEvent::Gesture(EditEvent::Commit { value, changed: true, .. }) => kept.shapes.push(value),
            ShapeDragEvent::Click { at, kind, .. } if CreateShapeDialog::supports(kind) => {
                let dialog =
                    CreateShapeOptions::initial(kind, at).units(units(RulerArg::Pixels)).tool(args.corner_radius, args.sides, args.star_ratio);
                kept.dialogs.open(CreateShapeDialog::new(dialog));
            }
            _ => {}
        }
    }
    kept.dialogs.show(&ctx);
    if let Some(DialogResult::Ok(created)) = kept.dialogs.take::<CreateShapeDialog>() {
        kept.note = craft_i18n::fmt(&words.created, &[("kind", &format!("{:?}", created.kind))]);
        kept.shapes.push(created.geometry);
    }

    let tokens = Tokens::of(ui);
    let stroke = Stroke::new(tokens.stroke.medium, tokens.color.accent.solid.to_egui());
    let painter = ui.painter_at(canvas.view.rect);
    for geometry in &kept.shapes {
        outline(&painter, &canvas.view, geometry, stroke);
    }
    status(ui, words.keys.clone());
    ui.horizontal(|ui| {
        if ui.add(Button::new(story.reset.as_str())).clicked() {
            kept.shapes.clear();
            kept.note.clear();
        }
        if !kept.note.is_empty() {
            status(ui, kept.note.clone());
        }
    });
}

// --- dialog ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum DialogKind {
    Rectangle,
    #[choice(name = "Rounded Rectangle")]
    RoundedRectangle,
    Ellipse,
    Triangle,
    Polygon,
    Star,
}

impl DialogKind {
    fn kind(self) -> ShapeKind {
        match self {
            DialogKind::Rectangle => ShapeKind::Rectangle,
            DialogKind::RoundedRectangle => ShapeKind::RoundedRectangle,
            DialogKind::Ellipse => ShapeKind::Ellipse,
            DialogKind::Triangle => ShapeKind::Triangle,
            DialogKind::Polygon => ShapeKind::Polygon,
            DialogKind::Star => ShapeKind::Star,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DialogArgs {
    open: bool,
    kind: DialogKind,
    /// The unit of the size fields; a secondary click on a field changes it for that field.
    ruler: RulerArg,
    /// Starts with a width of 0, so OK reports the size error.
    #[arg(name = "bad size")]
    bad_size: bool,
}

/// Create Shape opened at the page's origin; OK words the shape it would draw.
fn show_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    let words = StoryLabels::of(ui);
    let (kind, ruler, bad_size) = (args.kind.kind(), args.ruler, args.bad_size);
    drive(
        ui,
        "create-shape",
        &mut args.open,
        (kind, ruler, bad_size),
        || {
            let mut options = CreateShapeOptions::initial(kind, Point::ORIGIN).units(units(ruler)).tool(8.0, 5, 0.5);
            if bad_size {
                options.size.0 = 0.0;
            }
            CreateShapeDialog::new(options)
        },
        |_| {},
        |created| {
            let bounds = created.geometry.bounds();
            let value = format!("{:?} {} × {}", created.kind, bounds.width(), bounds.height());
            craft_i18n::fmt(&words.result_ok, &[("value", &value)])
        },
    );
}
