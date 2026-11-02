//! Tests of the Adaptive Wide Angle workspace (task WS-WA1).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{ColorImage, Event, Key, Modifiers, PointerButton, Pos2, Vec2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::full_window::PreviewTicket;
use craft_ui::dialog::{Outcome, Step};
use craft_ui::testing::canvas;
use craft_ui::theme::{Base, install};

use super::{
    ConstraintOrientation, WideAngleConstraint, WideAngleConstraintEdit, WideAngleEvent, WideAngleGeometry, WideAngleOpen, WideAngleSession,
    WideAngleSource, WideAngleTool, WideAngleWorkspace,
};

const WINDOW: Vec2 = vec2(900.0, 600.0);
const IMAGE: [u32; 2] = [400, 200];

/// A grey image that counts how often the canvas read it.
struct Grey {
    reads: AtomicUsize,
    /// The canvas re-reads a source only when its revision changes.
    revision: u64,
}

impl RasterSource for Grey {
    fn size(&self) -> [u32; 2] {
        IMAGE
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn read(&self, _region: PixelRect, _factor: u32, out: &mut ColorImage) {
        self.reads.fetch_add(1, Ordering::Relaxed);
        out.pixels.fill(egui::Color32::GRAY);
    }
}

/// Straight lines; the corrected preview moves every point down by `SHIFT` and counts the asks.
struct Lines {
    mapped: AtomicUsize,
}

const SHIFT: f64 = 10.0;

impl WideAngleGeometry for Lines {
    fn arc(&self, _session: &WideAngleSession, a: [f64; 2], b: [f64; 2], segments: usize) -> Vec<Point> {
        (0..=segments)
            .map(|i| {
                let t = i as f64 / segments as f64;
                Point::new(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
            })
            .collect()
    }

    fn corrected(&self, _session: &WideAngleSession, at: Point) -> Option<Point> {
        self.mapped.fetch_add(1, Ordering::Relaxed);
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
}

struct App {
    workspace: WideAngleWorkspace,
    geometry: Lines,
    sources: Sources,
    events: Vec<WideAngleEvent>,
    outcome: Option<Outcome>,
    tickets: usize,
    ticket: Option<PreviewTicket>,
    timing: Option<Duration>,
}

impl App {
    fn new(session: WideAngleSession) -> Self {
        Self {
            workspace: WideAngleWorkspace::new(WideAngleOpen::new(IMAGE, "Layer 1").session(session)),
            geometry: Lines { mapped: AtomicUsize::new(0) },
            sources: Sources {
                source: Grey { reads: AtomicUsize::new(0), revision: 1 },
                corrected: Grey { reads: AtomicUsize::new(0), revision: 2 },
            },
            events: Vec::new(),
            outcome: None,
            tickets: 0,
            ticket: None,
            timing: None,
        }
    }

    fn constraints(&self) -> &[WideAngleConstraint] {
        &self.workspace.session().constraints
    }

    /// How often the source and the corrected preview were read.
    fn reads(&self) -> (usize, usize) {
        (self.sources.source.reads.load(Ordering::Relaxed), self.sources.corrected.reads.load(Ordering::Relaxed))
    }

    fn mapped(&self) -> usize {
        self.geometry.mapped.load(Ordering::Relaxed)
    }
}

fn harness(app: App) -> Harness<'static, App> {
    let mut h = HarnessBuilder::default()
        .with_size(WINDOW)
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let out = app.workspace.show(ui.ctx(), &app.geometry, &app.sources);
                app.events.extend(out.events);
                app.tickets += usize::from(out.preview_wanted.is_some());
                app.ticket = out.preview_wanted.or(app.ticket);
                app.timing = out.timing;
                app.outcome = app.outcome.or(out.outcome);
            },
            app,
        );
    h.run();
    h.run();
    h
}

fn line(a: [f64; 2], b: [f64; 2]) -> WideAngleConstraint {
    WideAngleConstraint { a, b, orientation: ConstraintOrientation::Free }
}

fn session_with(constraints: Vec<WideAngleConstraint>) -> WideAngleSession {
    WideAngleSession { constraints, ..WideAngleSession::default() }
}

/// Where the image pixel `at` is on screen: the fitted image is centred in the canvas.
fn screen(h: &Harness<'_, App>, at: [f64; 2]) -> Pos2 {
    let centre = h.get_by_role_and_label(Role::Canvas, "Image").rect().center();
    let zoom = h.state().workspace.view.zoom() as f32;
    centre + vec2((at[0] - f64::from(IMAGE[0]) / 2.0) as f32 * zoom, (at[1] - f64::from(IMAGE[1]) / 2.0) as f32 * zoom)
}

fn press(h: &mut Harness<'_, App>, button: PointerButton, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event_modifiers(Event::PointerButton { pos, button, pressed, modifiers }, modifiers);
    h.run();
}

fn drag(h: &mut Harness<'_, App>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    h.event(Event::PointerMoved(from));
    h.run();
    press(h, PointerButton::Primary, from, true, modifiers);
    h.event(Event::PointerMoved(from + (to - from) * 0.5));
    h.run();
    h.event(Event::PointerMoved(to));
    h.run();
    press(h, PointerButton::Primary, to, false, modifiers);
    h.run();
}

fn right_click(h: &mut Harness<'_, App>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
    // Press and release in one pass: egui counts a click only within a short time.
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
    }
    h.run();
}

// A drag over 6 points adds a constraint, shorter adds none; Shift at the release makes it
// Horizontal or Vertical by the larger delta.
#[test]
fn wide_angle_drag_adds_a_constraint_over_six_points_and_shift_aligns() {
    let tie = WideAngleConstraint::from_drag(Point::new(0.0, 0.0), Point::new(5.0, 5.0), true);
    assert_eq!(tie.orientation, ConstraintOrientation::Horizontal, "a tie is Horizontal");
    assert_eq!(WideAngleConstraint::from_drag(Point::new(0.0, 0.0), Point::new(-3.0, 9.0), true).orientation, ConstraintOrientation::Vertical);
    assert_eq!(WideAngleConstraint::from_drag(Point::new(0.0, 0.0), Point::new(9.0, 3.0), false).orientation, ConstraintOrientation::Free);

    let mut h = harness(App::new(WideAngleSession::default()));
    let from = screen(&h, [200.0, 100.0]);
    drag(&mut h, from, from + vec2(3.0, 0.0), Modifiers::NONE);
    assert!(h.state().constraints().is_empty(), "3 points is a click");

    drag(&mut h, from, from + vec2(60.0, 4.0), Modifiers::NONE);
    let zoom = f64::from(h.state().workspace.view.zoom() as f32);
    let [added] = h.state().constraints() else { panic!("one constraint: {:?}", h.state().constraints()) };
    assert_eq!(added.orientation, ConstraintOrientation::Free);
    assert!((added.a[0] - 200.0).abs() < 1.0 && (added.b[0] - (200.0 + 60.0 / zoom)).abs() < 1.0, "document pixels: {added:?}");
    assert!(matches!(h.state().events.last(), Some(WideAngleEvent::Constraints(_))));

    drag(&mut h, from, from + vec2(40.0, 10.0), Modifiers::SHIFT);
    drag(&mut h, from, from + vec2(10.0, 40.0), Modifiers::SHIFT);
    let orientations: Vec<_> = h.state().constraints().iter().map(|c| c.orientation).collect();
    assert_eq!(orientations, [ConstraintOrientation::Free, ConstraintOrientation::Horizontal, ConstraintOrientation::Vertical]);
}

// A right-click within 10 points of a curve opens its menu; Horizontal changes it, Delete removes
// it; a right-click elsewhere opens nothing.
#[test]
fn wide_angle_right_click_menu_changes_orientation_or_deletes() {
    let mut h = harness(App::new(session_with(vec![line([100.0, 100.0], [300.0, 100.0])])));
    // The corrected preview shows: the curve is drawn where the geometry maps it.
    let on_line = screen(&h, [200.0, 100.0 + SHIFT]);

    right_click(&mut h, on_line);
    h.run();
    h.get_by_label("Horizontal").click();
    h.run();
    h.run();
    assert_eq!(h.state().constraints().len(), 1, "choosing an orientation keeps the constraint");
    assert_eq!(h.state().constraints()[0].orientation, ConstraintOrientation::Horizontal);

    let away = screen(&h, [200.0, 40.0]);
    right_click(&mut h, away);
    h.run();
    assert!(h.query_by_label("Horizontal").is_none(), "no menu away from a constraint");
    assert_eq!(h.state().constraints().len(), 1);

    right_click(&mut h, on_line);
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    h.run();
    assert!(h.state().constraints().is_empty());
}

// Preview on shows the corrected source with the curves mapped; off, or while dragging, the
// source with the curves as arcs.
#[test]
fn wide_angle_preview_toggle_shows_source_or_corrected() {
    let mut h = harness(App::new(session_with(vec![line([100.0, 100.0], [300.0, 100.0])])));
    h.run();
    let (source_reads, corrected_reads) = h.state().reads();
    assert!(corrected_reads > 0 && source_reads == 0, "the corrected preview shows: {:?}", h.state().reads());
    assert!(h.state().mapped() > 0, "the curves go through the solved mesh");

    h.get_by_label("Preview").click();
    h.run();
    h.run();
    assert!(!h.state().workspace.preview_on);
    assert!(h.state().reads().0 > 0, "the source shows with Preview off");
    let before = h.state().mapped();
    h.run();
    assert_eq!(h.state().mapped(), before, "the curves are the source's arcs");

    // A drag shows the source even with Preview on.
    h.get_by_label("Preview").click();
    h.run();
    h.run();
    assert!(h.state().workspace.preview_on);
    let from = screen(&h, [200.0, 150.0]);
    h.event(Event::PointerMoved(from));
    h.run();
    press(&mut h, PointerButton::Primary, from, true, Modifiers::NONE);
    h.event(Event::PointerMoved(from + vec2(50.0, 0.0)));
    h.run();
    h.run();
    assert!(h.state().workspace.constraint.dragging());
    let mapped = h.state().mapped();
    h.run();
    assert_eq!(h.state().mapped(), mapped, "the curves are arcs while the drag runs");
    press(&mut h, PointerButton::Primary, from + vec2(50.0, 0.0), false, Modifiers::NONE);
}

// ⌘Z steps the session's history, not the document's; Enter is OK.
#[test]
fn wide_angle_command_z_steps_the_workspace_and_enter_is_ok() {
    let mut h = harness(App::new(WideAngleSession::default()));
    let tickets = h.state().tickets;
    assert_eq!(tickets, 1, "opening asks for the first preview");
    for y in [60.0, 140.0] {
        let from = screen(&h, [100.0, y]);
        drag(&mut h, from, from + vec2(120.0, 0.0), Modifiers::NONE);
        h.run();
    }
    assert_eq!(h.state().constraints().len(), 2);
    assert!(h.state().tickets > tickets, "a change asks for a ticket");

    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(h.state().constraints().len(), 1, "⌘Z removes the last constraint");
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    h.run();
    assert_eq!(h.state().constraints().len(), 2, "⇧⌘Z puts it back");
    assert!(h.state_mut().workspace.step(Step::Undo), "the app's Edit menu steps too");
    assert_eq!(h.state().constraints().len(), 1);
    assert!(h.state().outcome.is_none());

    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().outcome, Some(Outcome::Ok));
}

// Constraints are keyboard parts: Page picks, arrows move by a pixel (Shift ten), Delete removes.
#[test]
fn wide_angle_constraints_are_keyboard_parts() {
    let mut h = harness(App::new(session_with(vec![line([100.0, 60.0], [300.0, 60.0]), line([100.0, 140.0], [300.0, 140.0])])));
    // A click on a constraint's middle picks it and gives the constraints the keyboard (the source shows
    // while the press runs, so the curve is where the arc is).
    let at = screen(&h, [200.0, 60.0]);
    drag(&mut h, at, at, Modifiers::NONE);
    assert_eq!(h.state().workspace.constraint.picked(), Some(0));
    assert!(h.state().constraints().len() == 2, "a click adds nothing");
    // The release on the part gave the constraints group the keyboard, whose Page keys pick and
    // whose arrows move the picked part.
    h.key_press(Key::PageDown);
    h.run();
    let Some(picked) = h.state().workspace.constraint.picked() else { panic!("Page Down picks a constraint") };
    assert_eq!(picked, 1);
    let before = h.state().constraints()[picked].clone();

    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().constraints()[picked].a[0], before.a[0] + 1.0);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(h.state().constraints()[picked].b, [before.b[0] + 1.0, before.b[1] + 10.0]);
    assert!(
        h.state()
            .events
            .iter()
            .any(|e| matches!(e, WideAngleEvent::Constraints(edit) if matches!(edit.value(), Some(WideAngleConstraintEdit::Move { .. })))),
        "the move is reported"
    );

    h.key_press(Key::Delete);
    h.run();
    assert_eq!(h.state().constraints().len(), 1);
    assert_eq!(h.state().constraints()[0].orientation, ConstraintOrientation::Free);
}

// With the canvas, not the constraints group, holding the keyboard, arrows do nothing: a click on
// the constraint away from its part picks it without giving the group the keyboard (a release on
// the part does).
#[test]
fn wide_angle_arrows_do_nothing_while_the_canvas_has_the_keyboard() {
    let mut h = harness(App::new(session_with(vec![line([100.0, 60.0], [300.0, 60.0])])));
    let at = screen(&h, [130.0, 60.0]);
    drag(&mut h, at, at, Modifiers::NONE);
    assert_eq!(h.state().workspace.constraint.picked(), Some(0));
    let before = h.state().constraints().to_vec();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().constraints(), before, "the arrow moved nothing");
}

// A letter that switches the tool mid-drag ends the drag through the host: the tool is not busy,
// and the release adds no constraint.
#[test]
fn wide_angle_tool_switch_mid_drag_cancels_it() {
    let mut h = harness(App::new(WideAngleSession::default()));
    let from = screen(&h, [100.0, 100.0]);
    canvas::press(&mut h, from, Modifiers::NONE);
    canvas::move_to(&mut h, from + vec2(80.0, 0.0));
    assert!(h.state().workspace.constraint.dragging());
    canvas::key(&mut h, Key::H, Modifiers::NONE);
    assert_eq!(h.state().workspace.tool, WideAngleTool::Hand);
    assert!(!h.state().workspace.constraint.dragging(), "the drag ended with the tool");
    canvas::release(&mut h, from + vec2(80.0, 0.0), Modifiers::NONE);
    assert!(h.state().constraints().is_empty(), "no constraint from the cancelled drag");
}

// ⌘Z mid-drag ends the drag before it steps.
#[test]
fn wide_angle_undo_mid_drag_cancels_it() {
    let mut h = harness(App::new(WideAngleSession::default()));
    let from = screen(&h, [100.0, 100.0]);
    canvas::press(&mut h, from, Modifiers::NONE);
    canvas::move_to(&mut h, from + vec2(80.0, 0.0));
    assert!(h.state().workspace.constraint.dragging());
    canvas::key(&mut h, Key::Z, Modifiers::COMMAND);
    assert!(!h.state().workspace.constraint.dragging(), "the drag ended before the step");
    canvas::release(&mut h, from + vec2(80.0, 0.0), Modifiers::NONE);
    assert!(h.state().constraints().is_empty());
}

// The output carries the time the app reported for the preview now shown.
#[test]
fn wide_angle_output_reports_the_previews_timing() {
    let mut h = harness(App::new(WideAngleSession::default()));
    assert_eq!(h.state().timing, None);
    let Some(ticket) = h.state().ticket else { panic!("opening asked for a preview") };
    h.state_mut().workspace.preview_ready(ticket, Some(Duration::from_millis(42)));
    h.run();
    assert_eq!(h.state().timing, Some(Duration::from_millis(42)));
}
