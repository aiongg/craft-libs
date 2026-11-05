//! Stories of Puppet Warp.
//!
//! The layer is `MockImage` on the shared stage of the transform preview stories
//! (`transform_preview::stories::Document`), drawn by the real `TransformPreview` through
//! `PreviewMap::pins` over a fake solver ([`IdwSolver`], an inverse-distance deformation of a
//! grid). Every case keeps its own session and applies the edits the overlay or the bar reports,
//! so pins can be placed, dragged, turned and deleted. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/pin-warp/pins` | Empty mesh, several pins, a selected pin, a fixed rotation ring, several selected, mesh hidden |
//! | `patterns/pin-warp-bar/bar` | The bar with and without a selected pin (with Fixed rotation on request); a refused remesh notice |

use craft_ui::canvas::kurbo::{Point, Rect, Vec2};
use craft_ui::localized::Labels as _;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, Surface};
use craft_ui::typed_story;
use egui::Ui;

use super::{
    Pin, PinDensity, PinEdit, PinMeshView, PinSolver, PinWarp, PinWarpBar, PinWarpBarOutput, PinWarpOptions, PinWarpSession, PinWarpSettings,
};
use crate::ui::transform_preview::PreviewMap;
use crate::ui::transform_preview::stories::{Document, Zoom, draw, grid, sensing};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PinWarp", "PinWarpBar"];

craft_ui::words! {
    /// The captions of the Puppet Warp stories, English by default.
    #[non_exhaustive]
    pub(crate) struct PinWarpStoryLabels {
        empty: "pinWarpStory.empty" = "No pins";
        several: "pinWarpStory.several" = "Several pins";
        selected: "pinWarpStory.selected" = "A selected pin";
        fixed: "pinWarpStory.fixed" = "Fixed rotation";
        multiple: "pinWarpStory.multiple" = "Several selected";
        hidden: "pinWarpStory.hidden" = "Mesh hidden";
        bar_none: "pinWarpStory.barNone" = "No pin selected";
        bar_selected: "pinWarpStory.barSelected" = "A pin selected";
        bar_refused: "pinWarpStory.barRefused" = "A refused remesh";
        refusal: "pinWarpStory.refusal" = "Could not use Puppet Warp: no opaque pixels at this density and expansion";
    }
}

/// The two stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/pin-warp/pins",
            Level::Composites,
            "PinWarp",
            "Pins",
            PinsArgs { width: Document::DEFAULT_SIZE[0], height: Document::DEFAULT_SIZE[1] },
            show_pins
        )
        .on(Surface::Canvas),
        typed_story!("patterns/pin-warp-bar/bar", Level::Patterns, "PinWarpBar", "Bar", BarArgs { fixed: false }, show_bar),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PinsArgs {
    /// The document's width, in pixels.
    #[arg(range = 200.0..=4000.0)]
    width: f64,
    /// The document's height, in pixels.
    #[arg(range = 200.0..=4000.0)]
    height: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    /// The selected pin's rotation is fixed: the bar shows its degrees field.
    fixed: bool,
}

/// The mesh's cells per side.
const GRID: usize = 12;
/// Added to the squared distance of a vertex from a pin, so a vertex on a pin does not divide by
/// zero.
const SOFTENING: f64 = 1e-6;

/// A fake pin solver: a grid over the layer whose vertices move by the inverse-distance weighted
/// mean of the pins' displacements. The pins' angles and depths are not used.
#[derive(Clone, Debug)]
pub(crate) struct IdwSolver {
    rest: Vec<Point>,
    deformed: Vec<Point>,
    triangles: Vec<[u32; 3]>,
    order: Vec<u32>,
    /// The text a remesh is refused with, if it is.
    refusal: Option<String>,
}

impl IdwSolver {
    /// An undeformed grid over `source`.
    pub(crate) fn new(source: Rect) -> Self {
        let side = GRID + 1;
        let rest: Vec<Point> = (0..side * side)
            .map(|k| {
                let (i, j) = ((k % side) as f64, (k / side) as f64);
                Point::new(source.x0 + source.width() * i / GRID as f64, source.y0 + source.height() * j / GRID as f64)
            })
            .collect();
        let mut triangles = Vec::new();
        for j in 0..GRID {
            for i in 0..GRID {
                let a = (j * side + i) as u32;
                let below = side as u32;
                triangles.push([a, a + 1, a + below + 1]);
                triangles.push([a, a + below + 1, a + below]);
            }
        }
        let order = (0..triangles.len() as u32).collect();
        Self { deformed: rest.clone(), rest, triangles, order, refusal: None }
    }

    /// The grid over `source` deformed by `pins`.
    pub(crate) fn solved(source: Rect, pins: &[Pin]) -> Self {
        let mut solver = Self::new(source);
        solver.deform(pins);
        solver
    }

    /// The mesh as the overlay and the preview read it.
    pub(crate) fn view(&self) -> PinMeshView<'_> {
        PinMeshView { rest: &self.rest, deformed: &self.deformed, triangles: &self.triangles, order: &self.order }
    }

    fn deform(&mut self, pins: &[Pin]) {
        for (vertex, rest) in self.deformed.iter_mut().zip(&self.rest) {
            let (mut sum, mut weight) = (Vec2::ZERO, 0.0);
            for pin in pins {
                let w = 1.0 / ((*rest - pin.rest).hypot2() + SOFTENING);
                sum += (pin.at - pin.rest) * w;
                weight += w;
            }
            *vertex = if weight > 0.0 { *rest + sum / weight } else { *rest };
        }
    }
}

impl PinSolver for IdwSolver {
    fn rebind(&mut self, pins: &[Pin], _: &PinWarpOptions) {
        self.deform(pins);
    }

    fn solve(&mut self, pins: &[Pin], _: &PinWarpOptions) {
        self.deform(pins);
    }

    fn remesh(&mut self, _: &PinWarpOptions) -> Result<(), String> {
        self.refusal.clone().map_or(Ok(()), Err)
    }

    fn mesh(&self) -> PinMeshView<'_> {
        self.view()
    }
}

/// Four pins on `layer`: one held where it attached, two pulled and one that anchors.
pub(crate) fn sample_pins(layer: Rect) -> Vec<Pin> {
    let at = |fx: f64, fy: f64| Point::new(layer.x0 + layer.width() * fx, layer.y0 + layer.height() * fy);
    let pin = |rest: (f64, f64), held: (f64, f64)| Pin { rest: at(rest.0, rest.1), at: at(held.0, held.1), angle: None, depth: 0 };
    vec![pin((0.2, 0.25), (0.2, 0.25)), pin((0.8, 0.25), (0.9, 0.1)), pin((0.5, 0.75), (0.45, 0.85)), pin((0.15, 0.8), (0.15, 0.8))]
}

/// What a case starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Empty,
    Several,
    Selected,
    Fixed,
    Multiple,
    Hidden,
    Refused,
}

/// A case's session and solver, kept between passes.
#[derive(Clone)]
struct PinCell {
    kind: Kind,
    layer: Rect,
    refusal: String,
    session: PinWarpSession,
    solver: IdwSolver,
    notice: Option<String>,
}

impl PinCell {
    fn new(kind: Kind, layer: Rect, refusal: &str) -> Self {
        let mut pins = if kind == Kind::Empty { Vec::new() } else { sample_pins(layer) };
        if kind == Kind::Fixed
            && let Some(pin) = pins.get_mut(1)
        {
            pin.angle = Some(30.0);
        }
        let selected = match kind {
            Kind::Selected | Kind::Fixed | Kind::Refused => vec![1],
            Kind::Multiple => vec![0, 1, 2],
            Kind::Empty | Kind::Several | Kind::Hidden => Vec::new(),
        };
        let settings = PinWarpSettings { show_mesh: kind != Kind::Hidden, ..PinWarpSettings::default() };
        let mut solver = IdwSolver::new(layer);
        let mut session = PinWarpSession::from_pins(pins, &settings);
        solver.deform(session.pins());
        session.apply(PinEdit::Select(selected), &mut solver, 0.0).ok();
        let mut notice = None;
        if kind == Kind::Refused {
            solver.refusal = Some(refusal.to_owned());
            let options = *session.options();
            notice = session.apply(PinEdit::Options(PinWarpOptions { density: PinDensity::More, ..options }), &mut solver, 0.0).err();
        }
        Self { kind, layer, refusal: refusal.to_owned(), session, solver, notice }
    }

    /// The case as it was kept for `salt`, or newly built when there is none or the stage changed.
    fn load(ui: &Ui, salt: &'static str, kind: Kind, layer: Rect, refusal: &str) -> Self {
        let id = ui.make_persistent_id(("pin-story", salt));
        ui.data(|d| d.get_temp::<PinCell>(id))
            .filter(|cell| cell.kind == kind && cell.layer == layer)
            .unwrap_or_else(|| Self::new(kind, layer, refusal))
    }

    fn store(self, ui: &Ui, salt: &'static str) {
        let id = ui.make_persistent_id(("pin-story", salt));
        ui.data_mut(|d| d.insert_temp(id, self));
    }

    fn reset(&mut self) {
        *self = Self::new(self.kind, self.layer, &self.refusal.clone());
    }

    /// Applies the edits a widget reported; a refused remesh becomes the notice, any other edit
    /// clears it.
    fn apply(&mut self, edits: Vec<PinEdit>, now: f64) {
        for edit in edits {
            self.notice = self.session.apply(edit, &mut self.solver, now).err();
        }
        self.session.settle(now, false);
    }
}

fn show_pins(ui: &mut Ui, args: &mut PinsArgs) {
    let labels = PinWarpStoryLabels::of(ui);
    let doc = Document::new(args.width, args.height);
    let textures = doc.textures(ui);
    let textures = textures.as_ref();
    let layer = doc.layer();
    let cases = [
        (Kind::Empty, "empty", &labels.empty),
        (Kind::Several, "several", &labels.several),
        (Kind::Selected, "selected", &labels.selected),
        (Kind::Fixed, "fixed", &labels.fixed),
        (Kind::Multiple, "multiple", &labels.multiple),
        (Kind::Hidden, "hidden", &labels.hidden),
    ];
    grid(ui, Vec::from(cases), |column, (kind, salt, text)| {
        column.add(Text::new(text).content(Content::Muted));
        let mut cell = PinCell::load(column, salt, kind, layer, &labels.refusal);
        let canvas = doc.canvas(column, text, Zoom::Fit, None);
        let (response, view) = (sensing(column, &canvas), canvas.view);
        draw(column, &view, &doc, textures, PreviewMap::pins(cell.solver.view(), layer), 1.0, false);
        let now = column.input(|i| i.time);
        let reported = PinWarp::new(salt, &cell.session).show(column, &response, &view, cell.solver.view());
        if reported.discard {
            cell.reset();
        }
        cell.apply(reported.edits, now);
        cell.store(column, salt);
    });
}

fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let labels = PinWarpStoryLabels::of(ui);
    let layer = Document::new(Document::DEFAULT_SIZE[0], Document::DEFAULT_SIZE[1]).layer();
    let selected = if args.fixed { Kind::Fixed } else { Kind::Selected };
    let cases = [
        (Kind::Several, "bar-none", &labels.bar_none),
        (selected, "bar-selected", &labels.bar_selected),
        (Kind::Refused, "bar-refused", &labels.bar_refused),
    ];
    for (kind, salt, text) in cases {
        ui.add(Text::new(text).content(Content::Muted));
        let mut cell = PinCell::load(ui, salt, kind, layer, &labels.refusal);
        let mut out = PinWarpBarOutput::default();
        OptionsBar::new(salt).label(text.as_str()).show(ui, |slots| {
            PinWarpBar::new("bar", &cell.session).notice(cell.notice.as_deref()).output(&mut out).add_to(slots);
        });
        if out.discard {
            cell.reset();
        }
        let now = ui.input(|i| i.time);
        cell.apply(out.edits, now);
        cell.store(ui, salt);
    }
}
