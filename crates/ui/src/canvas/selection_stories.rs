//! Stories of the selection frame, outlines, grips and range helpers (task KS1): not registered.
//! Registering them (task RG2) adds `stories()` to the canvas stories, `COMPONENTS` to the
//! component list and [`SelectionFrameStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! Every story draws its own fake objects on the canvas and keeps the edited boxes in the story's
//! state, so the frame and the grips can be dragged and nudged. The host owns the box and applies
//! `Begin`/`Update` as a preview and `Commit`/`Set` as the value; a `Cancel` restores the value
//! from before the gesture. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/selection/frame` | One object, several (the union box), the key object, a rotated view, a content frame with the item frame's grips off, grow 0–7, a rule that refuses the edges |
//! | `primitives/selection/grips` | Square, midpoint, arrow and triangle grips; cold, warm under the pointer, hot after a ⇧-click |
//! | `primitives/selection/outline` | Solid and dashed outlines over a list of shapes in two layer colours |
//! | `primitives/selection/range-bands` | A range with the active cell cut out; a 20-million-point column scrolled to its middle |

#![expect(dead_code, reason = "registered by task RG2")]

use egui::{Painter, Pos2, Shape, Stroke, Ui};
use kurbo::{Affine, BezPath, Ellipse, Point, Rect, Shape as _, Vec2};

use super::CanvasView;
use super::quad::{BoxAllow, BoxRules, BoxState, Homography, TransformMode};
use super::selection::{Grip, GripSet, GripShape, SelectionFrame, SelectionOutline, range_bands, rect_around};
use super::story::{DOC, ViewArgs, document};
use crate::edit::EditEvent;
use crate::localized::{Labels as _, LocaleNumbers};
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["SelectionFrame", "GripSet", "SelectionOutline"];

crate::words! {
    /// The captions of the selection stories, English by default.
    #[non_exhaustive]
    pub(crate) struct SelectionFrameStoryLabels {
        box_size: "selectionFrameStory.boxSize" = "Box: {x}, {y}, {width} × {height}";
        clicked: "selectionFrameStory.clicked" = "Clicked grip {key}";
        clicked_shift: "selectionFrameStory.clickedShift" = "Shift-clicked grip {key}";
        none: "selectionFrameStory.none" = "No click yet.";
        solid: "selectionFrameStory.solid" = "Solid";
        dashed: "selectionFrameStory.dashed" = "Dashed";
        range: "selectionFrameStory.range" = "Range {width} × {height}";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/selection/frame",
            Level::Primitives,
            "SelectionFrame",
            "Frame",
            FrameArgs { view: ViewArgs::default(), objects: Objects::One, key_object: false, content_frame: false, grow: 0, refuse_edges: false },
            show_frame
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/selection/grips",
            Level::Primitives,
            "GripSet",
            "Grips",
            GripsArgs { view: ViewArgs::default(), arrow: 0.0 },
            show_grips
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/selection/outline",
            Level::Primitives,
            "SelectionOutline",
            "Outline",
            PlainViewArgs { view: ViewArgs::default() },
            show_outline
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/selection/range-bands",
            Level::Primitives,
            "SelectionOutline",
            "Range bands",
            BandsArgs { view: ViewArgs::default(), huge: false, width: 2.0 },
            show_bands
        )
        .on(Surface::Canvas),
    ]
}

/// The fake objects of the frame story, in document units (the story document is 640 × 400).
const OBJECTS: [Rect; 3] = [Rect::new(80.0, 90.0, 220.0, 190.0), Rect::new(260.0, 140.0, 380.0, 280.0), Rect::new(160.0, 240.0, 300.0, 330.0)];
/// Opacity of the story's translucent fills.
const FILL: f32 = 0.25;
/// The largest grow step count `SelectionFrame::grow` is shown with.
const GROW_MAX: u32 = 7;

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// Applies an event as a host would: `Begin`/`Update` write through as a preview, `Commit`/`Set`
/// keep the value, `Cancel` restores the value from before the gesture.
fn fold<T: Copy>(event: &EditEvent<T>, value: &mut T, before: &mut Option<T>) {
    match event {
        EditEvent::Cancel { .. } => {
            if let Some(old) = before.take() {
                *value = old;
            }
        }
        EditEvent::Begin { .. } => {
            *before = Some(*value);
            if let Some(v) = event.value() {
                *value = *v;
            }
        }
        EditEvent::Commit { .. } | EditEvent::Set { .. } => {
            *before = None;
            if let Some(v) = event.value() {
                *value = *v;
            }
        }
        _ => {
            if let Some(v) = event.value() {
                *value = *v;
            }
        }
    }
}

// --- frame ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum Objects {
    One,
    Several,
}

impl Objects {
    fn picked(self) -> &'static [Rect] {
        match self {
            Objects::One => &OBJECTS[..1],
            Objects::Several => &OBJECTS,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FrameArgs {
    #[arg(flatten)]
    view: ViewArgs,
    objects: Objects,
    #[arg(name = "key object")]
    key_object: bool,
    #[arg(name = "content frame")]
    content_frame: bool,
    #[arg(range = 0.0..=7.0)]
    grow: u32,
    #[arg(name = "refuse edges")]
    refuse_edges: bool,
}

/// The frame story's state: the base box of the picked objects, the box now and the one a
/// cancelled gesture returns to.
#[derive(Clone, Copy, Debug)]
struct FrameState {
    objects: Objects,
    base: Rect,
    state: BoxState,
    before: Option<BoxState>,
}

fn union(rects: &[Rect]) -> Rect {
    rects.iter().skip(1).fold(rects[0], |acc, r| acc.union(*r))
}

/// The objects follow the box: each corner goes where the box maps the unit square.
fn mapped(map: &Homography, base: Rect, r: Rect, view: &CanvasView) -> Vec<Pos2> {
    let unit = |p: Point| Point::new((p.x - base.x0) / base.width(), (p.y - base.y0) / base.height());
    [Point::new(r.x0, r.y0), Point::new(r.x1, r.y0), Point::new(r.x1, r.y1), Point::new(r.x0, r.y1)]
        .map(|p| view.to_screen(map.apply(unit(p))))
        .to_vec()
}

/// The selected objects, a frame round them (the union box for several) and its grips, which
/// resize it, and its inside, which moves it. The objects follow the box.
fn show_frame(ui: &mut Ui, args: &mut FrameArgs) {
    let words = StoryLabels::of(ui);
    let own = SelectionFrameStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let key = ui.make_persistent_id("selection-frame-story");
    let ctx = ui.ctx().clone();
    let mut st: FrameState = ctx.data(|d| d.get_temp(key)).filter(|s: &FrameState| s.objects == args.objects).unwrap_or_else(|| {
        let base = union(args.objects.picked());
        FrameState { objects: args.objects, base, state: BoxState::from_rect(base), before: None }
    });
    let painter = ui.painter_at(canvas.view.rect);
    let object_fill = tokens.color.accent.solid.to_egui().gamma_multiply(FILL);
    if let Some(map) = Homography::unit_to_quad(st.state.quad) {
        for r in args.objects.picked() {
            painter.add(Shape::convex_polygon(mapped(&map, st.base, *r, &canvas.view), object_fill, Stroke::NONE));
        }
    }
    let mut rules = BoxRules::resize_only();
    if args.refuse_edges {
        // Distort refuses the edges: only corners, the inside and the reference point act.
        rules = BoxRules { mode: TransformMode::Distort, allow: BoxAllow { rotate: false, skew: false, distort: true }, ..rules };
    }
    let state = st.state;
    let mut frame = SelectionFrame::new("selection-frame-story", &state)
        .rules(rules)
        .key_object(args.key_object)
        .centre_mark(args.objects == Objects::Several)
        .grow(args.grow.min(GROW_MAX) as u8);
    if args.content_frame {
        frame = frame.grips(false).colour(tokens.color.status.warn.solid.to_egui());
    }
    let out = frame.show(ui, &canvas.response, &canvas.view);
    if let Some(event) = &out.event {
        fold(event, &mut st.state, &mut st.before);
    }
    let numbers = LocaleNumbers::of(ui);
    let q = st.state.quad;
    let (w, h) = ((q[1] - q[0]).hypot(), (q[3] - q[0]).hypot());
    status(
        ui,
        own.box_size
            .replace("{x}", &numbers.format(q[0].x, 0..=1))
            .replace("{y}", &numbers.format(q[0].y, 0..=1))
            .replace("{width}", &numbers.format(w, 0..=1))
            .replace("{height}", &numbers.format(h, 0..=1)),
    );
    ctx.data_mut(|d| d.insert_temp(key, st));
}

// --- grips ---

#[derive(StoryArgs, Debug, PartialEq)]
struct GripsArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// The direction of the arrow grips, in degrees.
    #[arg(range = -180.0..=180.0)]
    arrow: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PlainViewArgs {
    #[arg(flatten)]
    view: ViewArgs,
}

/// The shapes in the order of the grips' keys, in two rows: cold at the top, hot below.
const GRIP_SHAPES: [fn(Vec2) -> GripShape; 4] =
    [|_| GripShape::Square, |_| GripShape::Midpoint, |direction| GripShape::Arrow { direction }, |_| GripShape::Triangle];
const GRIP_COLUMNS: [f64; 4] = [120.0, 240.0, 360.0, 480.0];
const GRIP_ROWS: [f64; 2] = [140.0, 260.0];

#[derive(Clone, Debug)]
struct GripsState {
    grips: Vec<Grip>,
    before: Option<(u64, Point)>,
    last: Option<(u64, bool)>,
}

fn fresh_grips(arrow: Vec2) -> Vec<Grip> {
    let mut grips = Vec::new();
    for (row, y) in GRIP_ROWS.iter().enumerate() {
        for (column, x) in GRIP_COLUMNS.iter().enumerate() {
            let key = (row * GRIP_COLUMNS.len() + column) as u64 + 1;
            grips.push(Grip::new(key, Point::new(*x, *y), GRIP_SHAPES[column](arrow)).hot(row == 1));
        }
    }
    grips
}

/// Eight grips: cold ones above, hot ones below. The grip under the pointer is warm; a ⇧-click
/// toggles hot; a drag moves a grip.
fn show_grips(ui: &mut Ui, args: &mut GripsArgs) {
    let words = StoryLabels::of(ui);
    let own = SelectionFrameStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let ctx = ui.ctx().clone();
    let key = ui.make_persistent_id("selection-grips-story");
    let arrow = Vec2::from_angle(args.arrow.to_radians());
    let mut st: GripsState = ctx.data(|d| d.get_temp(key)).unwrap_or_else(|| GripsState { grips: fresh_grips(arrow), before: None, last: None });
    for grip in &mut st.grips {
        if let GripShape::Arrow { direction } = &mut grip.shape {
            *direction = arrow;
        }
    }
    let out = GripSet::new("selection-grips-story", &st.grips).show(ui, &canvas.response, &canvas.view);
    if let Some(click) = out.clicked {
        st.last = Some((click.key, click.shift));
        if click.shift
            && let Some(grip) = st.grips.iter_mut().find(|g| g.key == click.key)
        {
            grip.hot = !grip.hot;
        }
    }
    if let Some(event) = &out.drag {
        match event {
            EditEvent::Cancel { .. } => {
                if let Some((key, at)) = st.before.take()
                    && let Some(grip) = st.grips.iter_mut().find(|g| g.key == key)
                {
                    grip.at = at;
                }
            }
            _ => {
                if matches!(event, EditEvent::Begin { .. })
                    && let Some(drag) = event.value()
                {
                    st.before = Some((drag.key, drag.from));
                }
                if matches!(event, EditEvent::Commit { .. } | EditEvent::Set { .. }) {
                    st.before = None;
                }
                if let Some(drag) = event.value()
                    && let Some(grip) = st.grips.iter_mut().find(|g| g.key == drag.key)
                {
                    grip.at = drag.to;
                }
            }
        }
    }
    let line = match st.last {
        Some((key, true)) => own.clicked_shift.replace("{key}", &key.to_string()),
        Some((key, false)) => own.clicked.replace("{key}", &key.to_string()),
        None => own.none.clone(),
    };
    status(ui, line);
    ctx.data_mut(|d| d.insert_temp(key, st));
}

// --- outline ---

/// The shapes of the outline story, in the left half of the document: an ellipse and a rectangle
/// in the first layer's colour, a star and a blob with a curve in the second's.
fn outline_shapes() -> [BezPath; 4] {
    let ellipse = Ellipse::new(Point::new(90.0, 110.0), (60.0, 45.0), 0.0).to_path(0.1);
    let rect = Rect::new(170.0, 70.0, 290.0, 160.0).to_path(0.1);
    let mut star = BezPath::new();
    let (centre, outer, inner) = (Point::new(90.0, 290.0), 70.0, 28.0);
    for i in 0..10 {
        let radius = if i % 2 == 0 { outer } else { inner };
        let angle = std::f64::consts::TAU * f64::from(i) / 10.0 - std::f64::consts::FRAC_PI_2;
        let p = centre + Vec2::from_angle(angle) * radius;
        if i == 0 { star.move_to(p) } else { star.line_to(p) }
    }
    star.close_path();
    let mut blob = BezPath::new();
    blob.move_to((180.0, 300.0));
    blob.curve_to((200.0, 220.0), (270.0, 220.0), (290.0, 290.0));
    blob.curve_to((300.0, 340.0), (200.0, 360.0), (180.0, 300.0));
    blob.close_path();
    [ellipse, rect, star, blob]
}

/// Solid outlines on the left and the same shapes dashed on the right, in two layer colours.
fn show_outline(ui: &mut Ui, args: &mut PlainViewArgs) {
    let words = StoryLabels::of(ui);
    let own = SelectionFrameStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let layers = [tokens.color.accent.solid.to_egui(), tokens.color.status.good.solid.to_egui()];
    let shapes = outline_shapes();
    let painter: Painter = ui.painter_at(canvas.view.rect);
    let shift = Affine::translate((DOC.width() / 2.0, 0.0));
    for (dashed, offset) in [(false, Affine::IDENTITY), (true, shift)] {
        for (layer, pair) in shapes.chunks(2).enumerate() {
            let moved: Vec<BezPath> = pair.iter().map(|p| offset * p.clone()).collect();
            let outline = if dashed { SelectionOutline::dashed(&moved) } else { SelectionOutline::solid(&moved) };
            outline.colour(layers[layer]).paint(&painter, &canvas.view);
        }
    }
    status(ui, format!("{} · {}", own.solid, own.dashed));
}

// --- range bands ---

#[derive(StoryArgs, Debug, PartialEq)]
struct BandsArgs {
    #[arg(flatten)]
    view: ViewArgs,
    #[arg(name = "20-million column")]
    huge: bool,
    #[arg(range = 1.0..=8.0)]
    width: f64,
}

/// The size of the range selection in the ordinary case, in screen points: it sits at 60, 60 from
/// the canvas corner, with an active cell of the given size at its corner.
const RANGE_AT: [f32; 2] = [60.0, 60.0];
const RANGE_SIZE: [f32; 2] = [260.0, 140.0];
const CELL_SIZE: [f32; 2] = [90.0, 30.0];
/// How far the huge column reaches above and below the view, in screen points.
const HUGE_REACH: f32 = 10_000_000.0;
/// The huge column's width.
const HUGE_WIDTH: f32 = 120.0;

/// A range (an ordinary one, or a column 20 million points tall scrolled to its middle) with its
/// outline as four bands and its fill as the area round the active cell.
fn show_bands(ui: &mut Ui, args: &mut BandsArgs) {
    let words = StoryLabels::of(ui);
    let own = SelectionFrameStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let view = canvas.view.rect;
    let accent = tokens.color.accent.solid.to_egui();
    let (range, cell) = if args.huge {
        let x = view.center().x - HUGE_WIDTH / 2.0;
        let range = egui::Rect::from_min_max(egui::pos2(x, view.top() - HUGE_REACH), egui::pos2(x + HUGE_WIDTH, view.bottom() + HUGE_REACH));
        let cell = egui::Rect::from_min_size(egui::pos2(x, view.center().y), egui::vec2(CELL_SIZE[0], CELL_SIZE[1]));
        (range, cell)
    } else {
        let range = egui::Rect::from_min_size(view.min + egui::vec2(RANGE_AT[0], RANGE_AT[1]), egui::vec2(RANGE_SIZE[0], RANGE_SIZE[1]));
        (range, egui::Rect::from_min_size(range.min, egui::vec2(CELL_SIZE[0], CELL_SIZE[1])))
    };
    let painter = ui.painter_at(view);
    for part in rect_around(range.intersect(view), cell) {
        painter.rect_filled(part, 0.0, accent.gamma_multiply(FILL));
    }
    for band in range_bands(range, args.width as f32, view) {
        painter.rect_filled(band, 0.0, accent);
    }
    painter.rect_stroke(cell, 0.0, Stroke::new(args.width as f32, accent), egui::StrokeKind::Inside);
    let numbers = LocaleNumbers::of(ui);
    status(
        ui,
        own.range
            .replace("{width}", &numbers.format(f64::from(range.width()), 0..=0))
            .replace("{height}", &numbers.format(f64::from(range.height()), 0..=0)),
    );
}
