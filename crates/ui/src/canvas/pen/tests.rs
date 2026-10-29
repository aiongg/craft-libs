use egui::accesskit::Role;
use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Vec2};

use super::story::PenHost;
use super::{PenDraft, PenNames, PenResponse};
use crate::canvas::{CanvasView, EditPath, EditSubpath, Guide, GuideNames, Guides, Knot, KnotRef, Orientation, RulerUnits};
use crate::edit::{self, CancelReason, EditEvent, Ending, GestureHost, UndoMarks};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1.
struct Host {
    pen: PenHost,
    rotation: f64,
    flip: bool,
    /// The host gives the canvas keyboard focus.
    focus: bool,
    /// The pen is not shown (the host switched tools).
    hidden: bool,
    /// A vertical guide at this x, shown before the pen.
    guide: Option<f64>,
    view: Option<CanvasView>,
    events: Vec<EditEvent<EditSubpath>>,
    guide_events: usize,
    last: Option<PenResponse>,
    resumed: Vec<(u64, KnotRef)>,
}

fn open_path() -> EditPath {
    let knots = vec![Knot::corner(Point::new(300.0, 250.0)), Knot::corner(Point::new(360.0, 200.0))];
    EditPath { key: 5, subpaths: vec![EditSubpath { knots, closed: false }], color: None }
}

fn harness(paths: Vec<EditPath>) -> Harness<'static, Host> {
    let host = Host {
        pen: PenHost::new(paths),
        rotation: 0.0,
        flip: false,
        focus: false,
        hidden: false,
        guide: None,
        view: None,
        events: Vec::new(),
        guide_events: 0,
        last: None,
        resumed: Vec::new(),
    };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            ui.add_space(40.0);
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() | Sense::FOCUSABLE);
            if host.focus {
                canvas.request_focus();
            }
            let view = CanvasView { rotation: host.rotation, flip_x: host.flip, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            if let Some(x) = host.guide {
                let guides = [Guide::new(1, Orientation::Vertical, x)];
                let names = GuideNames::new("Guides", "Vertical guide", "Horizontal guide");
                let g = Guides::new("guides", &guides, &px, &px, names).interactive(true).show(ui, &canvas, &view);
                host.guide_events += usize::from(g.event.is_some());
            }
            if host.hidden {
                return;
            }
            let ends = host.pen.ends();
            let names = PenNames::new("Path being drawn", |i, n| format!("Anchor {i} of {n}"));
            let response = PenDraft::new("pen", host.pen.draft.as_ref(), names).ends(&ends).units(&px, &px).show(ui, &canvas, &view);
            host.events.extend(response.event.clone());
            host.resumed.extend(response.resumed);
            host.pen.apply(&response);
            host.last = Some(response);
        },
        host,
    )
}

fn empty() -> Harness<'static, Host> {
    let mut h = harness(Vec::new());
    h.run();
    h
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    h.state().view.expect("drawn").to_screen(p)
}

fn doc(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

fn press(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.event(button(at, true, modifiers));
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    let modifiers = h.ctx.input(|i| i.modifiers);
    h.event(button(at, false, modifiers));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

fn click(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    press(h, at, modifiers);
    release(h, at);
}

/// A drag over several frames: press, two moves, release.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    press(h, from, modifiers);
    move_to(h, from.lerp(to, 0.5), modifiers);
    move_to(h, to, modifiers);
    release(h, to);
}

fn key(h: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn click_doc(h: &mut Harness<'_, Host>, x: f64, y: f64) {
    let at = screen(h, doc(x, y));
    click(h, at, Modifiers::NONE);
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
        })
        .collect()
}

fn draft(h: &Harness<'_, Host>) -> EditSubpath {
    h.state().pen.draft.clone().unwrap_or_default()
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-3
}

/// A draft of three corner anchors at (60, 60), (160, 60), (160, 160).
fn three() -> Harness<'static, Host> {
    let mut h = empty();
    for (x, y) in [(60.0, 60.0), (160.0, 60.0), (160.0, 160.0)] {
        click_doc(&mut h, x, y);
    }
    h
}

#[test]
fn a_click_adds_a_corner_anchor_and_a_drag_pulls_out_symmetric_handles() {
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    assert_eq!(draft(&h).knots, [Knot::corner(doc(60.0, 60.0))]);
    assert_eq!(&kinds(&h)[..1], ["begin"]);
    let from = screen(&h, doc(160.0, 60.0));
    drag(&mut h, from, from + vec2(40.0, 30.0), Modifiers::NONE);
    let k = draft(&h).knots[1];
    assert_eq!(k, Knot::smooth(doc(160.0, 60.0), doc(200.0, 90.0)));
    assert!(near(k.in_ctrl, doc(120.0, 30.0)) && k.smooth);
}

#[test]
fn shift_puts_an_anchor_and_a_handle_on_the_nearest_45_degree_ray() {
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    // 100 right and 20 down from the last anchor: the horizontal ray.
    let at = screen(&h, doc(160.0, 80.0));
    click(&mut h, at, Modifiers::SHIFT);
    assert!(near(draft(&h).knots[1].anchor, doc(160.0, 60.0)), "{:?}", draft(&h).knots[1]);
    // A handle pulled 50 right and 40 down turns to the diagonal: its projection, 45 along each axis.
    let from = screen(&h, doc(260.0, 60.0));
    drag(&mut h, from, from + vec2(50.0, 40.0), Modifiers::SHIFT);
    let k = draft(&h).knots[2];
    let d = 90.0 / 2.0;
    assert!(near(k.out_ctrl, doc(260.0 + d, 60.0 + d)) && near(k.in_ctrl, doc(260.0 - d, 60.0 - d)), "{k:?}");
}

#[test]
fn alt_during_the_drag_breaks_the_pair() {
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    let from = screen(&h, doc(160.0, 60.0));
    press(&mut h, from, Modifiers::NONE);
    move_to(&mut h, from + vec2(30.0, 0.0), Modifiers::NONE);
    move_to(&mut h, from + vec2(40.0, 10.0), Modifiers::ALT);
    let k = draft(&h).knots[1];
    assert!(near(k.out_ctrl, doc(200.0, 70.0)), "the out handle follows");
    assert!(near(k.in_ctrl, doc(130.0, 60.0)) && !k.smooth, "the in handle stays where it was: {k:?}");
    release(&mut h, from + vec2(40.0, 10.0));
    assert_eq!(draft(&h).knots[1].out_ctrl, doc(200.0, 70.0));
}

#[test]
fn a_press_on_the_last_anchor_retracts_its_out_handle_and_a_drag_sets_it_alone() {
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    let from = screen(&h, doc(160.0, 60.0));
    drag(&mut h, from, from + vec2(40.0, 0.0), Modifiers::NONE);
    let before = draft(&h).knots[1];
    click(&mut h, from, Modifiers::NONE);
    let k = draft(&h).knots[1];
    assert_eq!(draft(&h).knots.len(), 2, "no new anchor");
    assert!(k.retracted(crate::canvas::HandleSide::Out) && k.in_ctrl == before.in_ctrl, "{k:?}");
    drag(&mut h, from, from + vec2(0.0, 40.0), Modifiers::NONE);
    let k = draft(&h).knots[1];
    assert!(near(k.out_ctrl, doc(160.0, 100.0)) && k.in_ctrl == before.in_ctrl && !k.smooth, "{k:?}");
}

#[test]
fn alt_on_a_handle_of_the_last_anchor_grabs_it_alone() {
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    let from = screen(&h, doc(160.0, 60.0));
    drag(&mut h, from, from + vec2(40.0, 0.0), Modifiers::NONE);
    let in_end = screen(&h, doc(120.0, 60.0));
    drag(&mut h, in_end, in_end + vec2(0.0, 25.0), Modifiers::ALT);
    let k = draft(&h).knots[1];
    assert!(near(k.in_ctrl, doc(120.0, 85.0)) && near(k.out_ctrl, doc(200.0, 60.0)) && !k.smooth, "{k:?}");
    assert_eq!(draft(&h).knots.len(), 2);
    // Without ⌥ the same press adds an anchor.
    drag(&mut h, in_end, in_end + vec2(0.0, 25.0), Modifiers::NONE);
    assert_eq!(draft(&h).knots.len(), 3);
}

#[test]
fn a_press_on_the_first_anchor_closes_the_path_on_release_with_or_without_a_drag() {
    let mut h = three();
    let first = screen(&h, doc(60.0, 60.0));
    press(&mut h, first, Modifiers::NONE);
    assert!(!draft(&h).closed, "closes on release");
    release(&mut h, first);
    let done = h.state().events.last().and_then(|e| e.value().cloned()).expect("commit");
    assert!(matches!(h.state().events.last(), Some(EditEvent::Commit { .. })));
    assert!(done.closed && done.knots.len() == 3 && done.knots[0] == Knot::corner(doc(60.0, 60.0)));
    assert!(h.state().pen.draft.is_none() && h.state().pen.paths[0].subpaths[0].closed);

    let mut h = three();
    let first = screen(&h, doc(60.0, 60.0));
    drag(&mut h, first, first + vec2(-30.0, 20.0), Modifiers::NONE);
    let done = h.state().pen.paths[0].subpaths[0].clone();
    assert!(done.closed);
    assert_eq!(done.knots[0], Knot::smooth(doc(60.0, 60.0), doc(30.0, 80.0)));
    assert_eq!(done.knots.len(), 3, "no anchor added");
}

#[test]
fn enter_and_esc_end_the_path_open_and_keep_it() {
    for k in [Key::Enter, Key::Escape] {
        let mut h = three();
        key(&mut h, Modifiers::NONE, k);
        assert!(matches!(h.state().events.last(), Some(EditEvent::Commit { changed: true, .. })), "{k:?}: {:?}", kinds(&h));
        let path = &h.state().pen.paths[0].subpaths[0];
        assert!(!path.closed && path.knots.len() == 3);
        assert!(h.state().pen.draft.is_none());
    }
    // One anchor is no path.
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", kinds(&h));
    assert!(h.state().pen.paths.is_empty());
}

#[test]
fn backspace_removes_the_last_anchor_and_the_only_one_cancels() {
    let mut h = three();
    key(&mut h, Modifiers::NONE, Key::Backspace);
    assert_eq!(draft(&h).knots.len(), 2);
    key(&mut h, Modifiers::NONE, Key::Delete);
    assert_eq!(draft(&h).knots.len(), 1);
    key(&mut h, Modifiers::NONE, Key::Backspace);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::UndoDuringGesture, .. })), "{:?}", kinds(&h));
    assert!(h.state().pen.draft.is_none());
    // Keys do nothing without a draft.
    let n = h.state().events.len();
    key(&mut h, Modifiers::NONE, Key::Backspace);
    assert_eq!(h.state().events.len(), n);
}

#[test]
fn remove_last_is_the_hosts_undo() {
    let mut h = three();
    let id = h.state().last.as_ref().expect("shown").id;
    PenDraft::remove_last(&h.ctx, id);
    h.run();
    assert_eq!(draft(&h).knots.len(), 2);
    PenDraft::remove_last(&h.ctx, id);
    h.run();
    PenDraft::remove_last(&h.ctx, id);
    h.run();
    assert!(h.state().pen.draft.is_none());
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { .. })));
}

/// The strokes and fills of the last frame, flattened.
fn shapes(h: &Harness<'_, Host>) -> Vec<Shape> {
    fn flat(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| flat(s, out)),
            other => out.push(other.clone()),
        }
    }
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|c| flat(&c.shape, &mut out));
    out
}

/// The end points of the dashes of the rubber band: the dashed segments are 1 pt wide.
fn rubber_end(h: &Harness<'_, Host>) -> Option<Pos2> {
    let tokens = crate::theme::current(&h.ctx);
    let width = tokens.component.bezier.line;
    shapes(h)
        .iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke }
                if stroke.width == width && (points[1] - points[0]).length() <= tokens.component.bezier.rubber_dash + 0.01 =>
            {
                Some(points[1])
            }
            _ => None,
        })
        .max_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)))
}

#[test]
fn the_rubber_band_runs_from_the_last_anchor_to_where_a_click_would_land() {
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    let last = screen(&h, doc(60.0, 60.0));
    let target = screen(&h, doc(200.0, 70.0));
    move_to(&mut h, target, Modifiers::NONE);
    let end = rubber_end(&h).expect("rubber band");
    assert!((end - screen(&h, doc(200.0, 70.0))).length() < 4.01, "within one dash of the pointer: {end:?}");
    // With ⇧ it ends on the horizontal ray.
    move_to(&mut h, target + vec2(0.5, 0.0), Modifiers::SHIFT);
    let end = rubber_end(&h).expect("rubber band");
    assert!((end.y - last.y).abs() < 0.01, "{end:?} against {last:?}");
    // Not while a handle is being pulled.
    let from = screen(&h, doc(260.0, 60.0));
    press(&mut h, from, Modifiers::NONE);
    move_to(&mut h, from + vec2(30.0, 30.0), Modifiers::NONE);
    assert_eq!(rubber_end(&h), None);
}

#[test]
fn over_the_first_anchor_the_close_cue_shows() {
    let mut h = three();
    let circles = |h: &Harness<'_, Host>| {
        shapes(h).iter().filter(|s| matches!(s, Shape::Circle(c) if c.fill == egui::Color32::TRANSPARENT && c.stroke.width > 0.0)).count()
    };
    let away = screen(&h, doc(200.0, 200.0));
    move_to(&mut h, away, Modifiers::NONE);
    assert_eq!(circles(&h), 0);
    let first = screen(&h, doc(60.0, 60.0));
    move_to(&mut h, first + vec2(2.0, 2.0), Modifiers::NONE);
    let tokens = crate::theme::current(&h.ctx);
    let b = tokens.component.bezier;
    let ring = shapes(&h).into_iter().find_map(|s| match s {
        Shape::Circle(c) if c.fill == egui::Color32::TRANSPARENT && c.stroke.width > 0.0 => Some(c),
        _ => None,
    });
    let ring = ring.expect("ring");
    assert_eq!(ring.radius, b.close_ring);
    assert!((ring.center - (first + vec2(2.0 + b.close_offset, 2.0 + b.close_offset))).length() < 0.01);
}

#[test]
fn a_press_on_an_open_paths_end_resumes_it_only_without_a_draft() {
    let mut h = harness(vec![open_path()]);
    h.run();
    let end = screen(&h, doc(360.0, 200.0));
    click(&mut h, end, Modifiers::NONE);
    assert_eq!(h.state().resumed, [(5, KnotRef::new(0, 1))]);
    assert_eq!(kinds(&h), ["begin"], "no anchor added; the draft the host hands in begins");
    // The host's draft is the path, its pressed end last; the next anchor continues it.
    assert_eq!(draft(&h).knots.len(), 2);
    click_doc(&mut h, 380.0, 120.0);
    assert_eq!(draft(&h).knots.len(), 3);
    key(&mut h, Modifiers::NONE, Key::Enter);
    let path = &h.state().pen.paths[0];
    assert_eq!((path.key, path.subpaths[0].knots.len()), (5, 3));

    // The first knot's end: the host reverses the path.
    let mut h = harness(vec![open_path()]);
    h.run();
    let end = screen(&h, doc(300.0, 250.0));
    click(&mut h, end, Modifiers::NONE);
    assert_eq!(h.state().resumed, [(5, KnotRef::new(0, 0))]);
    assert_eq!(draft(&h).knots.last().map(|k| k.anchor), Some(doc(300.0, 250.0)));
    // With a draft open another path's end is just a place for an anchor.
    let mut h = harness(vec![open_path()]);
    h.run();
    click_doc(&mut h, 100.0, 100.0);
    let end = screen(&h, doc(360.0, 200.0));
    click(&mut h, end, Modifiers::NONE);
    assert!(h.state().resumed.is_empty());
    assert_eq!(draft(&h).knots.len(), 2);
}

#[test]
fn the_first_anchor_takes_the_press_within_6_points_in_a_rotated_flipped_view() {
    for (offset, closes) in [(5.9, true), (6.1, false)] {
        let mut h = harness(Vec::new());
        (h.state_mut().rotation, h.state_mut().flip) = (30.0, true);
        h.run();
        for (x, y) in [(60.0, 60.0), (160.0, 60.0), (160.0, 160.0)] {
            click_doc(&mut h, x, y);
        }
        let first = screen(&h, doc(60.0, 60.0));
        click(&mut h, first + vec2(offset, 0.0), Modifiers::NONE);
        let closed = matches!(h.state().events.last(), Some(EditEvent::Commit { .. }));
        assert_eq!(closed, closes, "offset {offset}");
        if !closes {
            assert_eq!(draft(&h).knots.len(), 4);
        }
    }
}

#[test]
fn a_guide_under_the_press_keeps_it() {
    let mut h = empty();
    h.state_mut().guide = Some(160.0);
    h.run();
    let (from, to) = (screen(&h, doc(160.0, 100.0)), screen(&h, doc(180.0, 100.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    assert!(h.state().pen.draft.is_none(), "{:?}", kinds(&h));
    assert!(h.state().guide_events > 0, "the guide moved");
}

#[test]
fn a_quick_drag_in_one_frame_pulls_the_handles() {
    let mut h = empty();
    click_doc(&mut h, 60.0, 60.0);
    let from = screen(&h, doc(160.0, 60.0));
    let to = from + vec2(40.0, 30.0);
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false, Modifiers::NONE));
    h.run();
    assert_eq!(draft(&h).knots[1], Knot::smooth(doc(160.0, 60.0), doc(200.0, 90.0)));
}

/// An interrupted draft commits, whatever had the keyboard focus, and losing the focus leaves it
/// open.
#[test]
fn an_interrupted_draft_commits_and_focus_loss_leaves_it_open() {
    for focus in [true, false] {
        let mut h = harness(Vec::new());
        h.state_mut().focus = focus;
        h.run();
        for (x, y) in [(60.0, 60.0), (160.0, 60.0)] {
            click_doc(&mut h, x, y);
        }
        let id = h.state().events.iter().find_map(|e| e.id()).expect("begun");
        assert_eq!(edit::ending(&h.ctx, id), Ending::Commit { changed: true }, "focused canvas: {focus}");
        // The focus goes to an options-bar field.
        h.state_mut().focus = false;
        h.ctx.memory_mut(|m| m.request_focus(Id::unique("options-bar field")));
        h.run();
        assert!(!kinds(&h).iter().any(|k| ["commit", "cancel", "set"].contains(k)), "focused canvas: {focus}: {:?}", kinds(&h));
        click_doc(&mut h, 160.0, 160.0);
        assert_eq!(draft(&h).knots.len(), 3, "focused canvas: {focus}");
        // The host stops showing the pen: its gesture is no longer live.
        h.state_mut().hidden = true;
        h.run();
        h.run();
        assert!(!edit::is_live(&h.ctx, id));
        assert_eq!(edit::ending(&h.ctx, id), Ending::Commit { changed: true });
    }
}

/// A document with the path being drawn and the options bar's stroke width; its undo history
/// holds whole snapshots.
#[derive(Clone, Debug, Default)]
struct Drawing {
    path: EditSubpath,
    width: f32,
    undo: Vec<(EditSubpath, f32)>,
}

impl Drawing {
    /// Changes the document as one undo step.
    fn edit(&mut self, f: impl FnOnce(&mut Self)) {
        self.undo.push((self.path.clone(), self.width));
        f(self);
    }
}

impl UndoMarks for Drawing {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some((path, width)) = self.undo.get(mark).cloned() {
            (self.path, self.width) = (path, width);
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

/// The pen under a [`GestureHost`], beside an options bar whose stroke width is set before or
/// after the pen in a frame.
struct Hosted {
    doc: Drawing,
    host: GestureHost<Drawing>,
    /// The width the options bar sets in the next frame, and whether it comes before the pen.
    set_width: Option<(f32, bool)>,
    /// The draft was finished: the host no longer hands it in.
    finished: bool,
    view: Option<CanvasView>,
}

fn hosted() -> Harness<'static, Hosted> {
    let state = Hosted { doc: Drawing { width: 1.0, ..Drawing::default() }, host: GestureHost::new(), set_width: None, finished: false, view: None };
    Harness::<Hosted>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, s: &mut Hosted| {
            let ctx = ui.ctx().clone();
            let options_bar = |s: &mut Hosted, before: bool| {
                if let Some((width, _)) = s.set_width.take_if(|(_, first)| *first == before) {
                    s.host.handle(&ctx, &mut s.doc, EditEvent::set(width, Modifiers::NONE), |doc: &mut Drawing, w| doc.edit(|d| d.width = w));
                }
            };
            options_bar(s, true);
            ui.add_space(40.0);
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag());
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            s.view = Some(view);
            let draft = (!s.finished && !s.doc.path.knots.is_empty()).then(|| s.doc.path.clone());
            let names = PenNames::new("Path being drawn", |i, n| format!("Anchor {i} of {n}"));
            let r = PenDraft::new("pen", draft.as_ref(), names).show(ui, &canvas, &view);
            if let Some(event) = r.event {
                s.host.handle(&ctx, &mut s.doc, event, |doc: &mut Drawing, path| doc.edit(|d| d.path = path));
            }
            s.finished |= r.finished;
            options_bar(s, false);
            s.host.end_frame(&ctx, &mut s.doc);
        },
        state,
    )
}

fn hosted_click(h: &mut Harness<'_, Hosted>, x: f64, y: f64) {
    let at = h.state().view.expect("drawn").to_screen(doc(x, y));
    h.event(Event::PointerMoved(at));
    h.event(button(at, true, Modifiers::NONE));
    h.run();
    h.event(button(at, false, Modifiers::NONE));
    h.run();
}

/// A draft ended with one anchor is no path, whatever the host already holds of it: with nothing
/// filed it is cancelled; after an options-bar change kept the first two anchors, its end
/// removes them as one more undo step instead of bringing back the anchor the user removed.
#[test]
fn a_draft_ended_with_one_anchor_leaves_no_path() {
    for bar in [false, true] {
        for k in [Key::Enter, Key::Escape] {
            let mut h = hosted();
            h.run();
            hosted_click(&mut h, 60.0, 60.0);
            hosted_click(&mut h, 160.0, 60.0);
            if bar {
                h.state_mut().set_width = Some((4.0, true));
                h.run();
            }
            h.key_press(Key::Backspace);
            h.run();
            h.key_press(k);
            h.run();
            h.run();

            let s = h.state();
            assert!(s.finished, "bar: {bar}, {k:?}");
            assert!(s.doc.path.knots.is_empty(), "bar: {bar}, {k:?}: {:?}", s.doc.path);
            if bar {
                // The first two anchors, the width, their removal.
                let mut d = s.doc.clone();
                assert_eq!((d.undo.len(), d.width), (3, 4.0), "{k:?}");
                d.revert_to(2);
                assert_eq!(d.path.knots.len(), 2, "{k:?}");
            } else {
                assert!(s.doc.undo.is_empty(), "{k:?}: {:?}", s.doc.undo);
            }
        }
    }
}

/// Enter and Esc right after an options-bar change end the draft, though its gesture has no
/// change to report: the host already holds every anchor.
#[test]
fn enter_or_esc_after_an_options_bar_change_finishes_the_draft() {
    for k in [Key::Enter, Key::Escape] {
        let mut h = hosted();
        h.run();
        hosted_click(&mut h, 60.0, 60.0);
        hosted_click(&mut h, 160.0, 60.0);
        h.state_mut().set_width = Some((4.0, true));
        h.run();
        h.key_press(k);
        h.run();
        h.run();

        let s = h.state();
        assert!(s.finished, "{k:?}");
        assert_eq!(s.doc.path.knots.iter().map(|k| k.anchor).collect::<Vec<_>>(), [doc(60.0, 60.0), doc(160.0, 60.0)], "{k:?}");
        // The anchors and the width; finishing adds no step.
        assert_eq!((s.doc.undo.len(), s.doc.width), (2, 4.0), "{k:?}");
    }
}

/// An options-bar change during a draft keeps the anchors so far as one undo step, and the
/// anchors after it reach the document too, whether the bar comes before or after the pen.
#[test]
fn a_change_in_the_options_bar_during_a_draft_keeps_every_anchor() {
    for before in [true, false] {
        let mut h = hosted();
        h.run();
        let click_at = |h: &mut Harness<'_, Hosted>, x: f64, y: f64| {
            let at = h.state().view.expect("drawn").to_screen(doc(x, y));
            h.event(Event::PointerMoved(at));
            h.event(button(at, true, Modifiers::NONE));
            h.run();
            h.event(button(at, false, Modifiers::NONE));
            h.run();
        };
        click_at(&mut h, 60.0, 60.0);
        click_at(&mut h, 160.0, 60.0);
        h.state_mut().set_width = Some((4.0, before));
        h.run();
        click_at(&mut h, 160.0, 160.0);
        h.key_press(Key::Enter);
        h.run();

        let anchors = |d: &Drawing| d.path.knots.iter().map(|k| k.anchor).collect::<Vec<_>>();
        let s = h.state();
        assert!(s.finished, "bar before the pen: {before}");
        assert_eq!(anchors(&s.doc), [doc(60.0, 60.0), doc(160.0, 60.0), doc(160.0, 160.0)], "bar before the pen: {before}");
        assert_eq!(s.doc.width, 4.0);
        // Three steps: the first two anchors, the width, the last anchor.
        let mut d = s.doc.clone();
        assert_eq!(d.undo.len(), 3, "bar before the pen: {before}");
        d.revert_to(2);
        assert_eq!((anchors(&d).len(), d.width), (2, 4.0), "bar before the pen: {before}");
        d.revert_to(1);
        assert_eq!((anchors(&d).len(), d.width), (2, 1.0), "bar before the pen: {before}");
    }
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

#[test]
fn anchors_are_graphics_objects_in_a_group_that_is_not_a_tab_stop() {
    let mut h = three();
    let group = h.get_by_label("Path being drawn");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    let objects: Vec<(String, String, Option<bool>)> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect();
    let o = |name: &str, value: &str, selected: bool| (name.to_owned(), value.to_owned(), Some(selected));
    assert_eq!(
        objects,
        [
            o("Anchor 1 of 3", "60.0 px, 60.0 px", false),
            o("Anchor 2 of 3", "160.0 px, 60.0 px", false),
            o("Anchor 3 of 3", "160.0 px, 160.0 px", true)
        ]
    );
    assert_eq!(h.get_by_label("Path being drawn").accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Anchor 3 of 3"));
    // The group never takes keyboard focus, whatever the keys do.
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let focused = h.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label());
    assert_ne!(focused.as_deref(), Some("Path being drawn"));
    let _ = Vec2::ZERO;
}
