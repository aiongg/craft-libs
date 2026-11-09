//! The Pen and the Curvature Pen: anchors, handles, keys, Auto Add/Delete, the close cue and the tool's access
//! node, driven through `ToolFixture`.

use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, CanvasView, EditEvent};
use craft_draw::ui::pen::{BezierTool, EditSubpath, HandleSide, Knot, KnotRef, PathMode, PathTool, PenEvent, PenMode, PenTool};
use craft_keymap::Mods;
use craft_ui::canvas::{CanvasTool, ToolContext};
use craft_ui::testing::{ToolFixture, trial};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, vec2};

/// The document point drawn at the middle of the canvas: the canvas shows (0, 0) to (400, 300) at zoom 1.
const MIDDLE: Point = Point::new(200.0, 150.0);

/// The anchor-hit distance the tests stay well inside: a quarter of the least distance between two anchors.
const NEAR: f64 = 1e-3;

fn view_of(room: Rect) -> CanvasView {
    CanvasView::new(room, MIDDLE, 1.0)
}

/// A pen on a canvas, with the events it reported so far.
struct Pen {
    fixture: ToolFixture<PenTool>,
    view: fn(Rect) -> CanvasView,
    log: Vec<PenEvent>,
}

fn pen() -> Pen {
    pen_with(PenTool::new(), view_of)
}

fn pen_with(tool: PenTool, view: fn(Rect) -> CanvasView) -> Pen {
    Pen { fixture: ToolFixture::with_view("Pen", tool, view, |_, _| {}), view, log: Vec::new() }
}

fn button(at: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers }
}

fn doc(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < NEAR
}

impl Pen {
    fn sync(&mut self) {
        let events = self.fixture.tool_mut().take_events();
        self.log.extend(events);
    }

    /// The screen point of the document point `(x, y)`.
    fn at(&self, x: f64, y: f64) -> Pos2 {
        (self.view)(self.fixture.canvas().rect).to_screen(doc(x, y))
    }

    fn press(&mut self, at: Pos2, modifiers: Modifiers) {
        self.fixture.fixture().hold(modifiers);
        self.fixture.fixture().frame(vec![Event::PointerMoved(at)]);
        self.fixture.fixture().frame(vec![button(at, true, modifiers)]);
        self.sync();
    }

    fn move_to(&mut self, at: Pos2, modifiers: Modifiers) {
        self.fixture.fixture().hold(modifiers);
        self.fixture.fixture().frame(vec![Event::PointerMoved(at)]);
        self.sync();
    }

    fn release(&mut self, at: Pos2, modifiers: Modifiers) {
        self.fixture.fixture().frame(vec![button(at, false, modifiers)]);
        self.fixture.fixture().hold(Modifiers::NONE);
        self.fixture.fixture().frame(Vec::new());
        self.sync();
    }

    fn click(&mut self, at: Pos2, modifiers: Modifiers) {
        self.press(at, modifiers);
        self.release(at, modifiers);
    }

    fn click_doc(&mut self, x: f64, y: f64) {
        let at = self.at(x, y);
        self.click(at, Modifiers::NONE);
    }

    /// A drag over several frames: press, two moves, release.
    fn drag(&mut self, from: Pos2, to: Pos2, modifiers: Modifiers) {
        self.press(from, modifiers);
        self.move_to(from.lerp(to, 0.5), modifiers);
        self.move_to(to, modifiers);
        self.release(to, modifiers);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.fixture.fixture().key(key, modifiers);
        self.sync();
    }

    /// The draft as the host holds it: the last value the pen reported, empty once it finished.
    fn draft(&self) -> EditSubpath {
        let mut now = EditSubpath::default();
        for event in &self.log {
            match event {
                PenEvent::Edit(EditEvent::Update { value, .. }) => now = value.clone(),
                PenEvent::Edit(EditEvent::Commit { after, .. }) => now = after.clone(),
                PenEvent::Finished => now = EditSubpath::default(),
                _ => {}
            }
        }
        now
    }

    /// The path the last `Commit` filed.
    fn filed(&self) -> Option<EditSubpath> {
        self.log.iter().rev().find_map(|event| match event {
            PenEvent::Edit(EditEvent::Commit { after, .. }) => Some(after.clone()),
            _ => None,
        })
    }

    fn kinds(&self) -> Vec<&'static str> {
        self.log
            .iter()
            .map(|event| match event {
                PenEvent::Edit(EditEvent::Begin { .. }) => "begin",
                PenEvent::Edit(EditEvent::Update { .. }) => "update",
                PenEvent::Edit(EditEvent::Commit { .. }) => "commit",
                PenEvent::Edit(EditEvent::Cancel { .. }) => "cancel",
                PenEvent::Edit(_) => "other",
                PenEvent::Finished => "finished",
                PenEvent::Resumed(..) => "resumed",
                _ => "unknown",
            })
            .collect()
    }

    fn finished(&self) -> bool {
        self.log.iter().any(|event| matches!(event, PenEvent::Finished))
    }

    /// A draft of three corner anchors at (60, 60), (160, 60), (160, 160).
    fn three() -> Pen {
        let mut pen = pen();
        for (x, y) in [(60.0, 60.0), (160.0, 60.0), (160.0, 160.0)] {
            pen.click_doc(x, y);
        }
        pen
    }
}

#[trial]
fn a_click_adds_a_corner_anchor_and_a_drag_pulls_out_symmetric_handles() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    assert_eq!(pen.draft().knots, [Knot::corner(doc(60.0, 60.0))]);
    assert_eq!(pen.kinds().first(), Some(&"begin"));
    let from = pen.at(160.0, 60.0);
    pen.drag(from, from + vec2(40.0, 30.0), Modifiers::NONE);
    let knot = pen.draft().knots[1];
    assert_eq!(knot, Knot::smooth(doc(160.0, 60.0), doc(200.0, 90.0)));
    assert!(near(knot.in_ctrl, doc(120.0, 30.0)) && knot.smooth);
}

#[trial]
fn alt_during_the_drag_breaks_the_pair() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    let from = pen.at(160.0, 60.0);
    pen.press(from, Modifiers::NONE);
    pen.move_to(from + vec2(30.0, 0.0), Modifiers::NONE);
    pen.move_to(from + vec2(40.0, 10.0), Modifiers::ALT);
    let knot = pen.draft().knots[1];
    assert!(near(knot.out_ctrl, doc(200.0, 70.0)), "the out handle follows");
    assert!(near(knot.in_ctrl, doc(130.0, 60.0)) && !knot.smooth, "the in handle stays where it was: {knot:?}");
    pen.release(from + vec2(40.0, 10.0), Modifiers::ALT);
    assert_eq!(pen.draft().knots[1].out_ctrl, doc(200.0, 70.0));
}

#[trial]
fn a_press_on_the_last_anchor_retracts_its_out_handle_and_a_drag_sets_it_alone() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    let from = pen.at(160.0, 60.0);
    pen.drag(from, from + vec2(40.0, 0.0), Modifiers::NONE);
    let before = pen.draft().knots[1];
    pen.click(from, Modifiers::NONE);
    let knot = pen.draft().knots[1];
    assert_eq!(pen.draft().knots.len(), 2, "no new anchor");
    assert!(knot.retracted(HandleSide::Out) && knot.in_ctrl == before.in_ctrl, "{knot:?}");
    pen.drag(from, from + vec2(0.0, 40.0), Modifiers::NONE);
    let knot = pen.draft().knots[1];
    assert!(near(knot.out_ctrl, doc(160.0, 100.0)) && knot.in_ctrl == before.in_ctrl && !knot.smooth, "{knot:?}");
}

#[trial]
fn alt_on_a_handle_of_the_last_anchor_grabs_it_alone() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    let from = pen.at(160.0, 60.0);
    pen.drag(from, from + vec2(40.0, 0.0), Modifiers::NONE);
    let in_end = pen.at(120.0, 60.0);
    pen.drag(in_end, in_end + vec2(0.0, 25.0), Modifiers::ALT);
    let knot = pen.draft().knots[1];
    assert!(near(knot.in_ctrl, doc(120.0, 85.0)) && near(knot.out_ctrl, doc(200.0, 60.0)) && !knot.smooth, "{knot:?}");
    assert_eq!(pen.draft().knots.len(), 2);
    // Without the key the same press adds an anchor.
    pen.drag(in_end, in_end + vec2(0.0, 25.0), Modifiers::NONE);
    assert_eq!(pen.draft().knots.len(), 3);
}

#[trial]
fn a_press_on_the_first_anchor_closes_the_path_on_release_with_or_without_a_drag() {
    let mut pen = Pen::three();
    let first = pen.at(60.0, 60.0);
    pen.press(first, Modifiers::NONE);
    assert!(!pen.draft().closed, "closes on release");
    pen.release(first, Modifiers::NONE);
    let done = pen.filed().expect("commit");
    assert!(pen.kinds().ends_with(&["commit", "finished"]), "{:?}", pen.kinds());
    assert!(done.closed && done.knots.len() == 3 && done.knots[0] == Knot::corner(doc(60.0, 60.0)));

    let mut pen = Pen::three();
    let first = pen.at(60.0, 60.0);
    pen.drag(first, first + vec2(-30.0, 20.0), Modifiers::NONE);
    let done = pen.filed().expect("commit");
    assert!(done.closed);
    assert_eq!(done.knots[0], Knot::smooth(doc(60.0, 60.0), doc(30.0, 80.0)));
    assert_eq!(done.knots.len(), 3, "no anchor added");
}

#[trial]
fn enter_and_escape_end_the_path_open_and_keep_it() {
    for key in [Key::Enter, Key::Escape] {
        let mut pen = Pen::three();
        pen.key(key, Modifiers::NONE);
        let done = pen.filed().unwrap_or_else(|| panic!("{key:?}: {:?}", pen.kinds()));
        assert!(!done.closed && done.knots.len() == 3);
        assert!(pen.finished());
    }
    // One anchor is no path.
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    pen.key(Key::Enter, Modifiers::NONE);
    assert!(matches!(pen.log.iter().rev().nth(1), Some(PenEvent::Edit(EditEvent::Cancel { reason: CancelReason::Escape, .. }))), "{:?}", pen.kinds());
    assert!(pen.filed().is_none());
}

#[trial]
fn backspace_removes_the_last_anchor_and_the_only_one_cancels() {
    let mut pen = Pen::three();
    pen.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(pen.draft().knots.len(), 2);
    pen.key(Key::Delete, Modifiers::NONE);
    assert_eq!(pen.draft().knots.len(), 1);
    pen.key(Key::Backspace, Modifiers::NONE);
    assert!(
        matches!(pen.log.iter().rev().nth(1), Some(PenEvent::Edit(EditEvent::Cancel { reason: CancelReason::UndoDuringGesture, .. }))),
        "{:?}",
        pen.kinds()
    );
    assert!(pen.draft().knots.is_empty());
    // Keys do nothing without a draft.
    let seen = pen.log.len();
    pen.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(pen.log.len(), seen);
}

#[trial]
fn remove_last_waits_for_a_press_that_is_still_down() {
    let mut pen = Pen::three();
    let at = pen.at(260.0, 160.0);
    pen.press(at, Modifiers::NONE);
    assert!(!pen.fixture.tool_mut().remove_last(), "the press is held");
    pen.release(at, Modifiers::NONE);
    assert_eq!(pen.draft().knots.len(), 4);
    assert!(pen.fixture.tool_mut().remove_last());
    pen.sync();
    assert_eq!(pen.draft().knots.len(), 3, "one request removes one anchor");
}

#[trial]
fn a_press_on_an_open_paths_end_resumes_it_only_without_a_draft() {
    let end = KnotRef::new(0, 1);
    let mut pen = pen();
    pen.fixture.tool_mut().ends(&[(5, end, doc(360.0, 200.0))]);
    let at = pen.at(360.0, 200.0);
    pen.click(at, Modifiers::NONE);
    assert!(pen.log.contains(&PenEvent::Resumed(5, end)), "{:?}", pen.kinds());
    assert!(pen.draft().knots.is_empty(), "no anchor added");
    // With a draft open the same press adds an anchor.
    pen.click_doc(60.0, 60.0);
    pen.click(at, Modifiers::NONE);
    assert_eq!(pen.draft().knots.len(), 2);
    assert_eq!(pen.log.iter().filter(|event| matches!(event, PenEvent::Resumed(..))).count(), 1);
}

#[trial]
fn a_quick_drag_in_one_frame_pulls_the_handles() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    let (from, to) = (pen.at(160.0, 60.0), pen.at(160.0, 60.0) + vec2(40.0, 30.0));
    pen.fixture.fixture().frame_through(vec![
        Event::PointerMoved(from),
        button(from, true, Modifiers::NONE),
        Event::PointerMoved(to),
        button(to, false, Modifiers::NONE),
    ]);
    pen.sync();
    assert_eq!(pen.draft().knots[1], Knot::smooth(doc(160.0, 60.0), doc(200.0, 90.0)));
}

#[trial]
fn an_interrupted_draft_commits_and_losing_the_focus_leaves_it_open() {
    let mut pen = Pen::three();
    pen.fixture.fixture().frame(vec![Event::WindowFocused(false)]);
    pen.sync();
    assert!(!pen.finished(), "focus loss leaves the draft open: {:?}", pen.kinds());
    let room = pen.fixture.canvas().rect;
    let mut cx = ToolContext::new(room, None);
    pen.fixture.tool_mut().cancel(CancelReason::Removed, &mut cx);
    pen.sync();
    assert!(pen.finished());
    assert_eq!(pen.filed().map(|path| path.knots.len()), Some(3), "switching tools files the path");
}

#[trial]
fn a_draft_ended_with_one_anchor_leaves_no_path() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    pen.key(Key::Escape, Modifiers::NONE);
    assert!(pen.filed().is_none() && pen.finished(), "{:?}", pen.kinds());
    // A resumed path of two anchors that shrinks to one is removed from the host: the empty draft is committed.
    let path = EditSubpath::new(vec![Knot::corner(doc(300.0, 250.0)), Knot::corner(doc(360.0, 200.0))], false);
    let mut pen = pen_with(PenTool::new(), view_of);
    pen.fixture.tool_mut().draft(Some(&path));
    pen.fixture.fixture().idle(1);
    pen.sync();
    assert_eq!(pen.kinds(), ["begin"]);
    assert!(pen.fixture.tool_mut().remove_last());
    assert!(pen.fixture.tool_mut().remove_last());
    pen.sync();
    assert_eq!(pen.filed(), Some(EditSubpath::default()), "the host's path is removed: {:?}", pen.kinds());
}

#[trial]
fn auto_add_delete_edits_the_draft_and_shift_turns_it_off() {
    let mut pen = pen();
    pen.fixture.tool_mut().auto_add_delete(true);
    for (x, y) in [(60.0, 60.0), (160.0, 60.0), (260.0, 60.0), (360.0, 60.0)] {
        pen.click_doc(x, y);
    }
    // A press on a segment inserts an anchor.
    let on_segment = pen.at(110.0, 60.0);
    pen.click(on_segment, Modifiers::NONE);
    assert_eq!(pen.draft().knots.len(), 5);
    assert!(near(pen.draft().knots[1].anchor, doc(110.0, 60.0)), "{:?}", pen.draft().knots);
    // A press on an anchor that is neither the first nor the last deletes it.
    let middle = pen.at(260.0, 60.0);
    pen.click(middle, Modifiers::NONE);
    assert_eq!(pen.draft().knots.len(), 4);
    // With Shift held the press adds an anchor instead.
    let again = pen.at(210.0, 60.0);
    pen.click(again, Modifiers::SHIFT);
    assert_eq!(pen.draft().knots.len(), 5);
    assert!(near(pen.draft().knots[4].anchor, doc(210.0, 60.0)), "{:?}", pen.draft().knots);
}

#[trial]
fn the_pen_is_a_tool_every_canvas_accepts() {
    let mut fixture = ToolFixture::with_view("Pen", PenTool::new(), view_of, |_, _| {});
    fixture.check();
}

// ---- Curvature mode ----

fn curvature() -> Pen {
    let mut tool = PenTool::new();
    tool.mode(PenMode::Curvature);
    pen_with(tool, view_of)
}

fn three_curved() -> Pen {
    let mut pen = curvature();
    for (x, y) in [(60.0, 60.0), (160.0, 60.0), (160.0, 160.0)] {
        pen.click_doc(x, y);
    }
    pen
}

#[trial]
fn curvature_adds_smooth_anchors_that_follow_the_pointer() {
    let mut pen = curvature();
    pen.click_doc(60.0, 60.0);
    pen.click_doc(160.0, 60.0);
    let from = pen.at(160.0, 160.0);
    pen.press(from, Modifiers::NONE);
    let n = pen.draft().knots.len();
    assert_eq!(n, 3);
    pen.move_to(from + vec2(30.0, 20.0), Modifiers::NONE);
    pen.move_to(from + vec2(60.0, 40.0), Modifiers::NONE);
    assert!(near(pen.draft().knots[2].anchor, doc(220.0, 200.0)), "{:?}", pen.draft().knots[2]);
    pen.release(from + vec2(60.0, 40.0), Modifiers::NONE);
    assert!(pen.draft().knots.iter().all(|knot| knot.smooth));
}

#[trial]
fn curvature_closes_on_the_first_anchor_press() {
    let mut pen = three_curved();
    let first = pen.at(60.0, 60.0);
    pen.press(first, Modifiers::NONE);
    assert!(pen.finished(), "the press closes at once: {:?}", pen.kinds());
    assert!(pen.filed().is_some_and(|path| path.closed && path.knots.len() == 3));
    pen.release(first, Modifiers::NONE);
}

#[trial]
fn curvature_double_click_places_a_corner_point() {
    let mut pen = curvature();
    pen.click_doc(60.0, 60.0);
    pen.click_doc(160.0, 60.0);
    let at = pen.at(160.0, 160.0);
    pen.fixture.fixture().frame(vec![Event::PointerMoved(at)]);
    for pressed in [true, false, true, false] {
        pen.fixture.fixture().frame(vec![button(at, pressed, Modifiers::NONE)]);
    }
    pen.sync();
    let knot = pen.draft().knots[2];
    assert_eq!(pen.draft().knots.len(), 3, "the second click is on the anchor the first placed");
    assert!(!knot.smooth && knot.in_ctrl == knot.anchor && knot.out_ctrl == knot.anchor, "{knot:?}");
}

#[trial]
fn curvature_grabs_an_existing_anchor_and_reflows() {
    let mut pen = three_curved();
    let before = pen.draft();
    let middle = pen.at(160.0, 60.0);
    pen.drag(middle, middle + vec2(0.0, -30.0), Modifiers::NONE);
    let after = pen.draft();
    assert_eq!(after.knots.len(), 3, "no anchor added");
    assert!(near(after.knots[1].anchor, doc(160.0, 30.0)), "{:?}", after.knots[1]);
    assert_ne!(after.knots[2].in_ctrl, before.knots[2].in_ctrl, "the curve through the others was reflowed");
}

#[trial]
fn curvature_backspace_removes_the_last_anchor_and_reflows() {
    let mut pen = three_curved();
    pen.key(Key::Backspace, Modifiers::NONE);
    let draft = pen.draft();
    assert_eq!(draft.knots.len(), 2);
    let mut reflowed = draft.clone();
    reflowed.reflow();
    assert_eq!(draft, reflowed, "the handles flow through the anchors left");
}

// ---- Path modes and Path Options ----

/// A resolved mode as its three parts.
fn mode(bezier: BezierTool, draft: Option<PenMode>, draft_senses: bool) -> (BezierTool, Option<PenMode>, bool) {
    (bezier, draft, draft_senses)
}

fn resolve(tool: PathTool, mods: Mods, drawing: bool, auto_add_delete: bool) -> (BezierTool, Option<PenMode>, bool) {
    let resolved = PathMode::resolve(tool, mods, drawing, auto_add_delete);
    (resolved.bezier, resolved.draft, resolved.draft_senses)
}

/// `PathMode::resolve` over every row of its table: each tool, drawing or not, Auto Add/Delete on and off, with no
/// key, Shift, Alt and Command.
#[trial]
fn path_mode_resolves_the_pen_tools_and_modifiers() {
    use BezierTool::{AddAnchor, Anchors, AutoAddDelete, Convert, Curvature, DeleteAnchor, Direct};
    use PathTool as T;
    let (none, shift, alt, cmd) = (Mods::NONE, Mods::SHIFT, Mods::ALT, Mods::CMD);
    let (pen, curve) = (Some(PenMode::Pen), Some(PenMode::Curvature));
    let r = resolve;

    // Command makes any pen or anchor tool Direct Selection; the draft shows only while drawing.
    for tool in [T::Pen, T::AddAnchor, T::DeleteAnchor, T::ConvertPoint] {
        assert_eq!(r(tool, cmd, false, true), mode(Direct, None, false), "{tool:?}");
        assert_eq!(r(tool, cmd, true, true), mode(Direct, pen, false), "{tool:?}");
    }
    assert_eq!(r(T::Curvature, cmd, false, false), mode(Direct, None, false));
    assert_eq!(r(T::Curvature, cmd, true, false), mode(Direct, curve, false));

    // The Pen.
    assert_eq!(r(T::Pen, none, true, true), mode(Anchors, pen, true));
    assert_eq!(r(T::Pen, none, false, true), mode(AutoAddDelete, pen, true));
    assert_eq!(r(T::Pen, none, false, false), mode(Anchors, pen, true));
    assert_eq!(r(T::Pen, alt, false, true), mode(Convert, pen, true));
    for held in [shift, alt] {
        assert_eq!(r(T::Pen, held, true, true), mode(Anchors, pen, true));
    }
    assert_eq!(r(T::Pen, shift, false, true), mode(Anchors, pen, true));

    // The Curvature Pen.
    for held in [none, shift, alt] {
        assert_eq!(r(T::Curvature, held, false, true), mode(Curvature, curve, true));
        assert_eq!(r(T::Curvature, held, true, true), mode(Anchors, curve, true));
    }

    // The anchor tools: the swap with Alt is the overlay's; the draft shows without sensing.
    for held in [none, alt] {
        assert_eq!(r(T::AddAnchor, held, false, true), mode(AddAnchor, None, false));
        assert_eq!(r(T::AddAnchor, held, true, true), mode(AddAnchor, pen, false));
        assert_eq!(r(T::DeleteAnchor, held, false, true), mode(DeleteAnchor, None, false));
        assert_eq!(r(T::ConvertPoint, held, false, true), mode(Convert, None, false));
        assert_eq!(r(T::ConvertPoint, held, true, true), mode(Convert, pen, false));
    }
    for held in [none, shift, alt, cmd] {
        assert_eq!(r(T::DirectSelection, held, false, true), mode(Direct, None, false));
        assert_eq!(r(T::DirectSelection, held, true, true), mode(Direct, pen, false));
    }
}
