//! Stories of Puppet Warp: the pins on a plain canvas over a fake solver ([`IdwSolver`], an inverse-distance
//! deformation of a grid), and the options bar.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect, Vec2};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::Ui;

use super::{Pin, PinDensity, PinEdit, PinMeshView, PinSolver, PinWarp, PinWarpBar, PinWarpOptions, PinWarpOutput, PinWarpSession, PinWarpSettings};

/// The most pins the invalid story accepts.
const MOST_PINS: usize = 3;

/// The layer the pins deform.
fn layer() -> DocRect {
    DocRect::new(20.0, 20.0, 260.0, 200.0)
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
    pub(crate) fn new(source: DocRect) -> Self {
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
    pub(crate) fn solved(source: DocRect, pins: &[Pin]) -> Self {
        let mut solver = Self::new(source);
        solver.deform(pins);
        solver
    }

    /// The mesh as the overlay and the preview read it.
    pub(crate) fn view(&self) -> PinMeshView<'_> {
        PinMeshView::new(&self.rest, &self.deformed, &self.triangles, &self.order)
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
pub(crate) fn sample_pins(layer: DocRect) -> Vec<Pin> {
    let at = |fx: f64, fy: f64| Point::new(layer.x0 + layer.width() * fx, layer.y0 + layer.height() * fy);
    let pin = |rest: (f64, f64), held: (f64, f64)| Pin::new(at(rest.0, rest.1), at(held.0, held.1));
    vec![pin((0.2, 0.25), (0.2, 0.25)), pin((0.8, 0.25), (0.9, 0.1)), pin((0.5, 0.75), (0.45, 0.85)), pin((0.15, 0.8), (0.15, 0.8))]
}

/// Shows `pins` on a canvas whose view has the document origin at its top left; the selection the pins report is kept.
fn on_canvas(ui: &mut Ui, data: &mut StoryData, bind: Bind<'_, Vec<Pin>>, show_mesh: bool) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let selected = data.value("selected", vec![1usize]).clone();
    let solver = IdwSolver::new(layer());
    let mut control = Some(PinWarp::new("Pins", bind, solver.view()).selected_pins(&selected).show_mesh(show_mesh));
    let shown = Canvas::new("Document")
        .view(view)
        .show(ui, |ui, body| -> Option<PinWarpOutput> { control.take().map(|control| control.show_at(ui, body.rect)) });
    let Some(output) = shown.inner.flatten() else { return StoryOutput::default() };
    if let Some(select) = output.select.clone() {
        *data.value("selected", vec![1usize]) = select;
    }
    StoryOutput::from(output)
}

/// Drags the second pin by twice the drag threshold.
fn drag_second(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(pin) = fixture.part(id, super::overlay::PIN.at(1)) else { return };
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&pin, egui::Vec2::new(travel, 0.0));
}

fn pins_with_a_fixed_turn() -> Vec<Pin> {
    let mut pins = sample_pins(layer());
    pins[1].angle = Some(30.0);
    pins
}

#[story(PinWarp<'static>, covers = [Default], drive = Drive::Custom(drag_second))]
fn pins_with_a_fixed_rotation(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut pins = data.value("pins", pins_with_a_fixed_turn()).clone();
    let output = on_canvas(ui, data, Bind::bound(&mut pins), true);
    *data.value("pins", pins_with_a_fixed_turn()) = pins;
    output
}

#[story(PinWarp<'static>, covers = [Invalid], drive = Drive::Never)]
fn too_many_pins(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut pins = data.value("invalid", sample_pins(layer())).clone();
    let few = |pins: &Vec<Pin>| if pins.len() > MOST_PINS { Validity::Refuse(Message::new("There are too many pins")) } else { Validity::Ok };
    let output = on_canvas(ui, data, Bind::bound(&mut pins).check(few), true);
    *data.value("invalid", sample_pins(layer())) = pins;
    output
}

/// A session on the sample pins with pin 1 selected and, when `fixed`, its rotation fixed.
fn session(fixed: bool) -> (PinWarpSession, IdwSolver) {
    let pins = if fixed { pins_with_a_fixed_turn() } else { sample_pins(layer()) };
    let mut solver = IdwSolver::solved(layer(), &pins);
    let mut session = PinWarpSession::from_pins(pins, &PinWarpSettings::default());
    session.apply(PinEdit::Select(vec![1]), &mut solver).ok();
    (session, solver)
}

#[story(PinWarpBar<'static>, covers = [Default], drive = Drive::Never)]
fn bar_with_a_fixed_pin_selected(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let (session, _) = session(true);
    PinWarpBar::new(&session).show(ui);
    StoryOutput::default()
}

#[story(PinWarpBar<'static>, covers = [], drive = Drive::Never)]
fn bar_with_a_refused_remesh(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let (mut session, mut solver) = session(false);
    solver.refusal = Some("Could not use Puppet Warp: no opaque pixels at this density and expansion".to_owned());
    let options = *session.options();
    let refused = session.apply(PinEdit::Options(PinWarpOptions { density: PinDensity::More, ..options }), &mut solver).err();
    PinWarpBar::new(&session).notice(refused.as_deref()).show(ui);
    StoryOutput::default()
}
