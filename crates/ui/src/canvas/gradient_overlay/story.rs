//! The gradient overlay's story: a gradient line over the picture it colours.

use egui::{Color32, IdSalt, Mesh, Ui};
use kurbo::Point;

use super::{GradientLineNames, GradientOverlay};
use crate::canvas::{CanvasView, GradientExtent, GradientLine, gradient_line};
use crate::composites::gradient::stops::{StopRules, sample};
use crate::composites::{ColorStop, GradientStops, StopRef, StopRow, StopRows};
use crate::story::{Args, Level, Story};
use craft_theme::Srgba;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Gradient line"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "primitives/gradient-line/edit",
        level: Level::Primitives,
        component: "Gradient line",
        name: "Edit",
        args: || {
            super::super::story::args()
                .with_choice("rows", &["Split", "Combined"], 0)
                .with_choice("extent", &["None", "Circle", "Square", "Mirror"], 0)
                .with_choice("selected", &["None", "Stop 2", "Midpoint 1"], 1)
                .with_bool("reverse", false)
                .with_bool("midpoints", true)
                .with_bool("editable", true)
        },
        show: show_edit,
    }]
}

/// The English names the story and tests use.
pub(crate) fn english_names() -> GradientLineNames {
    GradientLineNames { group: "Gradient".into(), line: "Gradient line".into(), start: "Start point".into(), end: "End point".into() }
}

/// The story's stops: four colours with a half-transparent last stop, uneven so a midpoint moves.
pub(crate) fn story_stops(rows: StopRows) -> GradientStops {
    let alpha = if rows == StopRows::Combined { 0.45 } else { 1.0 };
    let mut color = vec![
        ColorStop::new(0.0, Srgba::rgb(0.13, 0.35, 0.72)),
        ColorStop { midpoint: 0.35, ..ColorStop::new(0.3, Srgba::rgb(0.95, 0.75, 0.2)) },
        ColorStop::new(0.7, Srgba::rgb(0.85, 0.25, 0.3)),
        ColorStop::new(1.0, Srgba { a: alpha, ..Srgba::rgb(0.9, 0.9, 0.95) }),
    ];
    color.sort_by(|a, b| a.location.total_cmp(&b.location));
    GradientStops { color, opacity: Vec::new() }
}

/// The story's line.
pub(crate) const LINE: GradientLine = GradientLine { start: Point::new(150.0, 230.0), end: Point::new(470.0, 150.0) };

fn show_edit(ui: &mut Ui, args: &mut Args) {
    let rows = if args.choice("rows") == "Combined" { StopRows::Combined } else { StopRows::Split };
    let extent = match args.choice("extent") {
        "Circle" => GradientExtent::Circle,
        "Square" => GradientExtent::Square,
        "Mirror" => GradientExtent::Mirror,
        _ => GradientExtent::None,
    };
    let canvas = super::super::story::document(ui, args, "Document");
    let view = &canvas.view;
    let (stops, reverse) = (story_stops(rows), args.bool("reverse"));
    paint_picture(ui, view, &stops, rows, reverse);
    let overlay = GradientOverlay::new("story-gradient", &LINE, &stops, english_names())
        .rules(StopRules { rows, ..StopRules::default() })
        .midpoints(args.bool("midpoints"))
        .reverse(reverse)
        .extent(extent)
        .editable(args.bool("editable"));
    let selected = match args.choice("selected") {
        "Stop 2" => Some(StopRef::Color(1)),
        "Midpoint 1" => Some(StopRef::Midpoint(StopRow::Color, 0)),
        _ => None,
    };
    GradientOverlay::select(ui.ctx(), ui.make_persistent_id(IdSalt::new("story-gradient")), selected);
    let _ = overlay.show(ui, &canvas.response, view);
}

/// The gradient on the document: a grid of cells coloured by their location along the line.
fn paint_picture(ui: &Ui, view: &CanvasView, stops: &GradientStops, rows: StopRows, reverse: bool) {
    const COLUMNS: u32 = 64;
    const ROWS: u32 = 40;
    let doc = super::super::story::DOC;
    let mut cells = Mesh::default();
    for j in 0..ROWS {
        for i in 0..COLUMNS {
            let (x0, y0) = (doc.x0 + doc.width() * f64::from(i) / f64::from(COLUMNS), doc.y0 + doc.height() * f64::from(j) / f64::from(ROWS));
            let (w, h) = (doc.width() / f64::from(COLUMNS), doc.height() / f64::from(ROWS));
            let centre = Point::new(x0 + w / 2.0, y0 + h / 2.0);
            let (t, _) = gradient_line::project(&LINE, centre, reverse);
            let color = Color32::from(sample(stops, rows, t));
            let base = cells.vertices.len() as u32;
            for p in [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)] {
                cells.colored_vertex(view.to_screen(Point::new(p.0, p.1)), color);
            }
            cells.add_triangle(base, base + 1, base + 2);
            cells.add_triangle(base, base + 2, base + 3);
        }
    }
    ui.painter_at(view.rect).add(cells);
}
