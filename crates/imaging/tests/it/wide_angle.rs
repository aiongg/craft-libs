//! Behaviour of the Adaptive Wide Angle workspace: the Constraint tool, the constraint lines, the context menu, the
//! Add Constraint form and the preview job.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use craft_behavior::PixelSize;
use craft_behavior::edit_history::Step;
use craft_behavior::kurbo::Point;
use craft_imaging::ui::wide_angle::{
    ConstraintOrientation, PreviewDone, WideAngleConstraint, WideAngleGeometry, WideAngleOpen, WideAngleSession, WideAngleSource, WideAngleWorkspace,
};
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::primitives::Revision;
use craft_ui::runtime::pending::Job;
use craft_ui::testing::relations::equals_token;
use craft_ui::testing::{Fixture, trial};
use egui::{Color32, ColorImage, Event, Key, Modifiers, PointerButton, Pos2, Vec2};

const IMAGE: [u32; 2] = [400, 200];

/// The test picture's colour: data, not a theme colour.
const PICTURE: Color32 = Color32::from_gray(0x80);

/// A step of the arrow keys in document pixels, and the step with Shift.
const NUDGE: f64 = 1.0;
const NUDGE_FAR: f64 = 10.0;

/// A distance along one axis, in points.
fn along(x: f32, y: f32) -> Vec2 {
    Vec2::X * x + Vec2::Y * y
}

/// A grey picture.
struct Grey(u64);

impl RasterSource for Grey {
    fn size(&self) -> PixelSize {
        PixelSize::new(IMAGE[0], IMAGE[1])
    }

    fn revision(&self) -> Revision {
        Revision::new(self.0)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        Some(PixelRect::default())
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(PICTURE);
    }
}

/// Straight lines; the corrected preview moves every point down by `SHIFT`.
struct Lines;

const SHIFT: f64 = 10.0;

impl WideAngleGeometry for Lines {
    fn arc(&self, _: &WideAngleSession, a: [f64; 2], b: [f64; 2], segments: usize) -> Vec<Point> {
        (0..=segments)
            .map(|i| {
                let t = i as f64 / segments as f64;
                Point::new(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
            })
            .collect()
    }

    fn corrected(&self, _: &WideAngleSession, at: Point) -> Option<Point> {
        Some(Point::new(at.x, at.y + SHIFT))
    }
}

struct Sources {
    source: Grey,
    corrected: Grey,
}

impl WideAngleSource for Sources {
    fn source(&self) -> &dyn RasterSource {
        &self.source
    }

    fn corrected(&self) -> Option<&dyn RasterSource> {
        Some(&self.corrected)
    }

    fn preview_job(&self, _: &WideAngleSession) -> Job<PreviewDone, String> {
        Job::new(|_, _| Ok(PreviewDone::new(Duration::from_millis(42))))
    }
}

struct App {
    workspace: WideAngleWorkspace,
    sources: Sources,
    timing: Option<Duration>,
    closed: bool,
}

fn line(a: [f64; 2], b: [f64; 2]) -> WideAngleConstraint {
    WideAngleConstraint::new(a, b, ConstraintOrientation::Free)
}

fn fixture(constraints: Vec<WideAngleConstraint>) -> (Fixture, Rc<RefCell<App>>) {
    let session = WideAngleSession::default().with_constraints(constraints);
    let app = Rc::new(RefCell::new(App {
        workspace: WideAngleWorkspace::new(WideAngleOpen::new(IMAGE, "Layer 1").session(session)),
        sources: Sources { source: Grey(1), corrected: Grey(2) },
        timing: None,
        closed: false,
    }));
    let shared = app.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut app = shared.borrow_mut();
        let App { workspace, sources, timing, closed } = &mut *app;
        let out = workspace.show(ui, &Lines, sources);
        *timing = out.timing.or(*timing);
        *closed |= out.closed.is_some();
    });
    fixture.room(along(900.0, 600.0));
    fixture.settle();
    (fixture, app)
}

/// Where the image pixel `at` is on screen: the fitted image is centred in the canvas.
fn screen(fixture: &Fixture, app: &Rc<RefCell<App>>, at: [f64; 2]) -> Pos2 {
    let canvas = fixture.by_label("Image").map(|target| target.center()).expect("the Image canvas");
    let zoom = (app.borrow().workspace.zoom_percent() / 100.0) as f32;
    canvas + Vec2::new((at[0] - f64::from(IMAGE[0]) / 2.0) as f32 * zoom, (at[1] - f64::from(IMAGE[1]) / 2.0) as f32 * zoom)
}

fn drag(fixture: &mut Fixture, from: Pos2, by: Vec2, modifiers: Modifiers) {
    fixture.hold(modifiers);
    fixture.frame(vec![Event::PointerMoved(from)]);
    fixture.frame(vec![Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers }]);
    fixture.frame(vec![Event::PointerMoved(from + by * 0.5)]);
    fixture.frame(vec![Event::PointerMoved(from + by)]);
    fixture.frame(vec![Event::PointerButton { pos: from + by, button: PointerButton::Primary, pressed: false, modifiers }]);
    fixture.hold(Modifiers::NONE);
    fixture.idle(2);
}

fn right_click(fixture: &mut Fixture, at: Pos2) {
    fixture.frame(vec![Event::PointerMoved(at)]);
    for pressed in [true, false] {
        fixture.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE }]);
    }
    fixture.idle(2);
}

fn constraints(app: &Rc<RefCell<App>>) -> Vec<WideAngleConstraint> {
    app.borrow().workspace.session().constraints.clone()
}

#[trial]
fn a_drag_over_six_points_adds_a_constraint_and_shift_aligns_it() {
    let tie = WideAngleConstraint::from_drag(Point::new(0.0, 0.0), Point::new(5.0, 5.0), true);
    assert_eq!(tie.orientation, ConstraintOrientation::Horizontal, "a tie is Horizontal");
    assert_eq!(WideAngleConstraint::from_drag(Point::new(0.0, 0.0), Point::new(-3.0, 9.0), true).orientation, ConstraintOrientation::Vertical);

    let (mut fixture, app) = fixture(Vec::new());
    let from = screen(&fixture, &app, [200.0, 100.0]);
    drag(&mut fixture, from, along(3.0, 0.0), Modifiers::NONE);
    assert!(constraints(&app).is_empty(), "3 points is a click");

    drag(&mut fixture, from, along(60.0, 4.0), Modifiers::NONE);
    drag(&mut fixture, from, along(40.0, 10.0), Modifiers::SHIFT);
    drag(&mut fixture, from, along(10.0, 40.0), Modifiers::SHIFT);
    let orientations: Vec<_> = constraints(&app).iter().map(|c| c.orientation).collect();
    assert_eq!(orientations, [ConstraintOrientation::Free, ConstraintOrientation::Horizontal, ConstraintOrientation::Vertical]);
}

#[trial]
fn the_context_menu_changes_a_constraints_orientation_or_deletes_it() {
    let (mut fixture, app) = fixture(vec![line([100.0, 100.0], [300.0, 100.0])]);
    // The corrected preview shows: the curve is drawn where the geometry maps it.
    let on_line = screen(&fixture, &app, [200.0, 100.0 + SHIFT]);
    right_click(&mut fixture, on_line);
    let Some(horizontal) = fixture.by_label("Horizontal") else { panic!("the menu offers Horizontal") };
    fixture.click(&horizontal);
    fixture.idle(2);
    assert_eq!(constraints(&app).iter().map(|c| c.orientation).collect::<Vec<_>>(), [ConstraintOrientation::Horizontal]);

    let away = screen(&fixture, &app, [200.0, 40.0]);
    right_click(&mut fixture, away);
    assert!(fixture.by_label("Horizontal").is_none(), "no menu away from a constraint");

    right_click(&mut fixture, on_line);
    let Some(delete) = fixture.by_label("Delete") else { panic!("the menu offers Delete") };
    fixture.click(&delete);
    fixture.idle(2);
    assert!(constraints(&app).is_empty());
}

#[trial]
fn preview_off_shows_the_source_and_the_curves_as_arcs() {
    let (mut fixture, app) = fixture(vec![line([100.0, 100.0], [300.0, 100.0])]);
    let mapped = screen(&fixture, &app, [200.0, 100.0 + SHIFT]);
    let unmapped = screen(&fixture, &app, [200.0, 100.0]);
    assert!(!equals_token(mapped.y, unmapped.y), "the two curves differ on screen");
    let Some(preview) = fixture.by_label("Preview") else { panic!("the panel has the Preview check") };
    fixture.click(&preview);
    fixture.idle(2);
    // With Preview off the curve is the source's arc: a right-click on it opens the menu.
    right_click(&mut fixture, unmapped);
    assert!(fixture.by_label("Horizontal").is_some(), "the arc is where the source shows it");
}

#[trial]
fn constraints_are_keyboard_parts_that_nudge_and_delete() {
    let (mut fixture, app) = fixture(vec![line([100.0, 60.0], [300.0, 60.0]), line([100.0, 140.0], [300.0, 140.0])]);
    let on_line = screen(&fixture, &app, [200.0, 140.0 + SHIFT]);
    fixture.click_at(on_line);
    let before = constraints(&app)[1].clone();
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(constraints(&app)[1].a[0], before.a[0] + NUDGE, "an arrow moves the focused constraint one pixel");
    fixture.key(Key::ArrowDown, Modifiers::SHIFT);
    fixture.idle(2);
    assert_eq!(constraints(&app)[1].b, [before.b[0] + NUDGE, before.b[1] + NUDGE_FAR], "Shift moves it ten");
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(constraints(&app).len(), 1);
}

#[trial]
fn undo_during_a_drag_cancels_the_drag() {
    let (mut fixture, app) = fixture(Vec::new());
    let from = screen(&fixture, &app, [100.0, 100.0]);
    fixture.frame(vec![Event::PointerMoved(from)]);
    fixture.frame(vec![Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    fixture.frame(vec![Event::PointerMoved(from + along(80.0, 0.0))]);
    app.borrow_mut().workspace.step(Step::Undo);
    fixture.idle(2);
    fixture.frame(vec![Event::PointerButton {
        pos: from + along(80.0, 0.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    fixture.idle(2);
    assert!(constraints(&app).is_empty(), "no constraint from the cancelled drag");
}

#[trial]
fn a_tool_switch_during_a_drag_cancels_the_drag() {
    let (mut fixture, app) = fixture(Vec::new());
    let from = screen(&fixture, &app, [100.0, 100.0]);
    fixture.frame(vec![Event::PointerMoved(from)]);
    fixture.frame(vec![Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    fixture.frame(vec![Event::PointerMoved(from + along(80.0, 0.0))]);
    fixture.key(Key::H, Modifiers::NONE);
    fixture.frame(vec![Event::PointerButton {
        pos: from + along(80.0, 0.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    fixture.idle(2);
    assert!(constraints(&app).is_empty(), "no constraint from the cancelled drag");
}

#[trial]
fn the_output_reports_the_previews_timing() {
    let (mut fixture, app) = fixture(Vec::new());
    assert_eq!(app.borrow().timing, None, "nothing reported before the job runs");
    fixture.idle(10);
    assert_eq!(app.borrow().timing, Some(Duration::from_millis(42)), "the finished preview job reports its time");
}

#[trial]
fn the_add_constraint_form_adds_a_constraint() {
    let (mut fixture, app) = fixture(Vec::new());
    let Some(add) = fixture.by_label("Add Constraint…") else { panic!("the panel has the Add button") };
    fixture.click(&add);
    fixture.idle(3);
    let Some(ok) = fixture.by_label("OK").filter(|_| fixture.by_label("Start").is_some()) else { panic!("the form opens with its OK button") };
    fixture.click(&ok);
    fixture.idle(3);
    let all = constraints(&app);
    let [added] = all.as_slice() else { panic!("one constraint: {all:?}") };
    assert_ne!(added.a, added.b, "the form's default end points differ");
}
