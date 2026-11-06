//! Tests of the pen draft's Curvature mode.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use kurbo::Point;

use super::bezier::EditSubpath;
use super::story::PenHost;
use super::{PathOptionsSettings, PenDraft, PenMode, PenNames, PenResponse};
use crate::geometry::curvature::curvature_handles;
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, current, install};

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1, the Pen in
/// Curvature mode.
struct Host {
    pen: PenHost,
    options: Option<PathOptionsSettings>,
    view: Option<CanvasView>,
    events: Vec<EditEvent<EditSubpath>>,
    finished: bool,
    sensing: bool,
    remove_last: bool,
    /// [`PenResponse::closes`] of the last pass.
    closes: bool,
}

fn empty() -> Harness<'static, Host> {
    let host = Host {
        pen: PenHost::new(Vec::new()),
        options: None,
        view: None,
        events: Vec::new(),
        finished: false,
        sensing: true,
        remove_last: false,
        closes: false,
    };
    // A small step keeps two presses in one spot within the double-click delay.
    let mut h = Harness::<Host>::builder()
        .with_size([480.0, 400.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() | Sense::FOCUSABLE);
                canvas.request_focus();
                let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
                host.view = Some(view);
                let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
                let names = PenNames::new("Path being drawn", |i, n| format!("Anchor {i} of {n}"));
                let mut pen = PenDraft::new("pen", host.pen.draft.as_ref(), names)
                    .mode(PenMode::Curvature)
                    .units(&px, &px)
                    .sensing(host.sensing)
                    .remove_last(std::mem::take(&mut host.remove_last));
                if let Some(options) = &host.options {
                    pen = pen.path_options(options);
                }
                let response: PenResponse = pen.show(ui, &canvas, &view);
                host.events.extend(response.event.clone());
                host.finished |= response.finished;
                host.closes = response.closes;
                host.pen.apply(&response);
            },
            host,
        );
    h.run();
    h
}

fn screen(h: &Harness<'_, Host>, x: f64, y: f64) -> Pos2 {
    h.state().view.expect("drawn").to_screen(Point::new(x, y))
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn press(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.event(button(at, true));
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(button(at, false));
    h.run();
}

fn click(h: &mut Harness<'_, Host>, x: f64, y: f64) {
    let at = screen(h, x, y);
    press(h, at);
    release(h, at);
}

/// Lets the double-click delay pass.
fn wait(h: &mut Harness<'_, Host>) {
    h.run_steps(20);
}

fn key(h: &mut Harness<'_, Host>, key: Key) {
    h.key_press(key);
    h.run();
}

fn draft(h: &Harness<'_, Host>) -> EditSubpath {
    h.state().pen.draft.clone().unwrap_or_default()
}

fn kinds(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "begin",
            EditEvent::Update { .. } => "update",
            EditEvent::Commit { .. } => "commit",
            EditEvent::Cancel { .. } => "cancel",
            EditEvent::Set { .. } => "set",
            _ => "other",
        })
        .collect()
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-3
}

/// Three anchors at (60, 60), (160, 60), (160, 160).
fn three() -> Harness<'static, Host> {
    let mut h = empty();
    for (x, y) in [(60.0, 60.0), (160.0, 60.0), (160.0, 160.0)] {
        click(&mut h, x, y);
        wait(&mut h);
    }
    h
}

/// Every smooth knot of `draft` has the handles `curvature_handles` gives.
fn flows(draft: &EditSubpath) -> bool {
    let anchors: Vec<Point> = draft.knots.iter().map(|k| k.anchor).collect();
    let corners: Vec<bool> = draft.knots.iter().map(|k| !k.smooth).collect();
    let fresh = curvature_handles(&anchors, &corners, draft.closed);
    draft.knots.iter().zip(fresh).all(|(k, (i, o))| !k.smooth || (near(k.in_ctrl, i) && near(k.out_ctrl, o)))
}

// Clicks add smooth anchors (straight until the third); a drag after a press moves the new anchor
// past the click slop and the draft reflows; each change is an Update.
#[test]
fn curvature_draft_adds_smooth_anchors_that_follow_the_pointer() {
    let mut h = empty();
    click(&mut h, 60.0, 60.0);
    wait(&mut h);
    click(&mut h, 160.0, 60.0);
    wait(&mut h);
    let two = draft(&h);
    assert!(two.knots.iter().all(|k| k.smooth && k.retracted(super::HandleSide::In) && k.retracted(super::HandleSide::Out)), "straight: {two:?}");
    click(&mut h, 160.0, 160.0);
    let three = draft(&h);
    assert_eq!(three.knots.len(), 3);
    assert!(three.knots.iter().all(|k| k.smooth));
    assert!(!three.knots[1].retracted(super::HandleSide::Out), "the middle anchor bends: {three:?}");
    assert!(flows(&three));
    assert_eq!(kinds(&h).first(), Some(&"begin"));
    assert!(kinds(&h)[1..].iter().all(|k| *k == "update"), "{:?}", kinds(&h));

    // A press that moves within the slop leaves the anchor where it went down.
    wait(&mut h);
    let at = screen(&h, 260.0, 100.0);
    press(&mut h, at);
    move_to(&mut h, at + vec2(0.5, 0.5));
    assert!(near(draft(&h).knots[3].anchor, Point::new(260.0, 100.0)));
    // Past it the anchor follows the pointer and the handles follow the anchors.
    move_to(&mut h, at + vec2(10.0, 40.0));
    move_to(&mut h, at + vec2(20.0, 60.0));
    let moved = draft(&h);
    assert!(near(moved.knots[3].anchor, Point::new(280.0, 160.0)), "{:?}", moved.knots[3]);
    assert!(flows(&moved));
    release(&mut h, at + vec2(20.0, 60.0));
    assert!(near(draft(&h).knots[3].anchor, Point::new(280.0, 160.0)));
}

// With 3 or more anchors a press on the first closes at once: `closed`, Commit, `finished`; with
// two it grabs that anchor instead (PhotoCraft: no close under 3 anchors).
#[test]
fn curvature_draft_closes_on_the_first_anchor_press() {
    let mut h = empty();
    click(&mut h, 60.0, 60.0);
    wait(&mut h);
    click(&mut h, 160.0, 60.0);
    wait(&mut h);
    click(&mut h, 60.0, 60.0);
    assert_eq!(draft(&h).knots.len(), 2, "two anchors: a press on the first grabs it, as PhotoCraft");
    assert!(!h.state().finished);

    let mut h = three();
    let first = screen(&h, 60.0, 60.0);
    press(&mut h, first);
    assert!(h.state().finished, "closes on the press, before the release");
    assert_eq!(kinds(&h).last(), Some(&"commit"));
    let path = h.state().pen.paths.last().cloned().expect("a path was filed");
    assert!(path.subpaths[0].closed);
    assert_eq!(path.subpaths[0].knots.len(), 3);
    assert!(flows(&path.subpaths[0]));
}

// A double-click on empty canvas leaves a corner anchor; a double-click on a corner makes it
// smooth again.
#[test]
fn curvature_draft_double_click_places_a_corner_point() {
    let mut h = three();
    click(&mut h, 260.0, 160.0);
    click(&mut h, 260.0, 160.0);
    let d = draft(&h);
    assert_eq!(d.knots.len(), 4, "the second click toggles the first click's anchor");
    let last = d.knots[3];
    assert!(!last.smooth && last.retracted(super::HandleSide::In) && last.retracted(super::HandleSide::Out), "{last:?}");
    assert!(flows(&d));
    wait(&mut h);
    click(&mut h, 260.0, 160.0);
    click(&mut h, 260.0, 160.0);
    let again = draft(&h);
    assert_eq!(again.knots.len(), 4);
    assert!(again.knots[3].smooth);
    assert!(flows(&again));
}

// A press on an inner anchor grabs it; the drag moves it and every smooth knot reflows.
#[test]
fn curvature_draft_grabs_an_existing_anchor_and_reflows() {
    let mut h = three();
    click(&mut h, 260.0, 160.0);
    wait(&mut h);
    let before = draft(&h);
    let at = screen(&h, 160.0, 60.0);
    press(&mut h, at);
    assert_eq!(draft(&h).knots.len(), 4, "a press on an anchor adds none");
    move_to(&mut h, at + vec2(0.0, -20.0));
    move_to(&mut h, at + vec2(0.0, -40.0));
    release(&mut h, at + vec2(0.0, -40.0));
    let after = draft(&h);
    assert!(near(after.knots[1].anchor, Point::new(160.0, 20.0)), "{:?}", after.knots[1]);
    assert!(flows(&after));
    assert_ne!(after.knots[2].in_ctrl, before.knots[2].in_ctrl, "the neighbours' handles follow");
}

// Backspace and `remove_last` remove the last anchor and reflow; Enter ends and keeps the draft.
#[test]
fn curvature_draft_backspace_removes_the_last_anchor_and_reflows() {
    let mut h = three();
    click(&mut h, 260.0, 160.0);
    key(&mut h, Key::Backspace);
    let d = draft(&h);
    assert_eq!(d.knots.len(), 3);
    assert!(flows(&d));
    let fresh = {
        let mut copy = d.clone();
        copy.reflow();
        copy
    };
    assert_eq!(d, fresh);

    key(&mut h, Key::Enter);
    assert!(h.state().finished);
    let path = h.state().pen.paths.last().cloned().expect("filed");
    assert_eq!(path.subpaths[0].knots.len(), 3);
    assert!(!path.subpaths[0].closed);
}

// The host's Undo removes the last anchor and reflows, in its own pass.
#[test]
fn curvature_remove_last_removes_the_last_anchor_and_reflows() {
    let mut h = three();
    click(&mut h, 260.0, 160.0);
    h.state_mut().remove_last = true;
    h.run();
    let d = draft(&h);
    assert_eq!(d.knots.len(), 3);
    let mut fresh = d.clone();
    fresh.reflow();
    assert_eq!(d, fresh);
}

// `sensing(false)` hides the hover cues (the dashed preview and the close cue) and `closes`, as
// the Pen does; sensing again brings them back.
#[test]
fn curvature_hover_cues_follow_sensing() {
    let mut h = three();
    let (middle, first) = (screen(&h, 100.0, 100.0), screen(&h, 60.0, 60.0));
    move_to(&mut h, middle);
    assert!(!dashes(&h).is_empty(), "the preview shows while sensing");
    move_to(&mut h, first);
    assert!(h.state().closes);
    h.state_mut().sensing = false;
    move_to(&mut h, middle);
    assert!(dashes(&h).is_empty(), "no preview without sensing");
    move_to(&mut h, first);
    assert!(!h.state().closes);
}

/// The two ends of every dash of the preview: the dashed segments are `line` wide and no
/// longer than.
fn dashes(h: &Harness<'_, Host>) -> Vec<Pos2> {
    fn flat(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| flat(s, out)),
            other => out.push(other.clone()),
        }
    }
    let tokens = current(&h.ctx);
    let mut all = Vec::new();
    h.output().shapes.iter().for_each(|c| flat(&c.shape, &mut all));
    all.iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke }
                if stroke.width == tokens.stroke.thin && (points[1] - points[0]).length() <= tokens.stroke.dash + 0.01 =>
            {
                Some(points.to_vec())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

/// Largest distance of `points` from the line through `a` and `b`.
fn bend(points: &[Pos2], a: Pos2, b: Pos2) -> f32 {
    let v = b - a;
    points.iter().map(|p| (v.x * (p.y - a.y) - v.y * (p.x - a.x)).abs() / v.length()).fold(0.0, f32::max)
}

// The hover draws the dashed preview segment through the pointer with 2 or more anchors and a
// straight dashed line with one, with the rubber band setting off and on.
#[test]
fn curvature_rubber_band_previews_the_next_curve() {
    for rubber_band in [false, true] {
        let mut h = empty();
        h.state_mut().options = Some(PathOptionsSettings { rubber_band, ..PathOptionsSettings::default() });
        h.run();
        click(&mut h, 60.0, 60.0);
        let last = screen(&h, 60.0, 60.0);
        let target = screen(&h, 200.0, 140.0);
        move_to(&mut h, target);
        let one = dashes(&h);
        assert!(!one.is_empty(), "a dashed line with one anchor (rubber band {rubber_band})");
        assert!(bend(&one, last, target) < 0.01, "straight");
        assert!((one.iter().map(|p| (*p - target).length()).fold(f32::MAX, f32::min)) < 4.01, "ends at the pointer");

        click(&mut h, 160.0, 60.0);
        let b = screen(&h, 160.0, 60.0);
        let target = screen(&h, 260.0, 160.0);
        move_to(&mut h, target);
        let two = dashes(&h);
        assert!(bend(&two, b, target) > 2.0, "a curve through the pointer (rubber band {rubber_band}): {two:?}");
        // Not while a press is down.
        press(&mut h, target);
        move_to(&mut h, target + vec2(0.0, 20.0));
        assert!(dashes(&h).is_empty());
    }
}
