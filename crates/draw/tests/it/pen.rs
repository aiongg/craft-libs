//! The Pen and the Curvature Pen: anchors, handles, keys, Auto Add/Delete, the close cue and the tool's access
//! node, driven through `ToolFixture`.

use craft_behavior::PointerButton as ToolButton;
use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, CanvasView, EditEvent};
use craft_draw::ui::pen::{
    BezierTool, EditSubpath, HandleSide, Knot, KnotRef, PATH_OPTIONS_COLOR_KEY, PathMode, PathOptionsPopover, PathOptionsSettings, PathTool,
    PenEvent, PenMode, PenTool,
};
use craft_keymap::Mods;
use craft_ui::canvas::{CanvasSpace, CanvasTool, Guide, GuideKey, ToolContext, ToolPointer};
use craft_ui::canvas_controls::Guides;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, ToolFixture, trial};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, vec2};
use std::cell::RefCell;
use std::rc::Rc;

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
fn shift_puts_an_anchor_and_a_handle_on_the_nearest_45_degree_ray() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    // 100 right and 20 down from the last anchor: the horizontal ray.
    let at = pen.at(160.0, 80.0);
    pen.click(at, Modifiers::SHIFT);
    assert!(near(pen.draft().knots[1].anchor, doc(160.0, 60.0)), "{:?}", pen.draft().knots[1]);
    // A handle pulled 50 right and 40 down turns to the diagonal: its projection, 45 along each axis.
    let from = pen.at(260.0, 60.0);
    pen.drag(from, from + vec2(50.0, 40.0), Modifiers::SHIFT);
    let knot = pen.draft().knots[2];
    let along = 90.0 / 2.0;
    assert!(near(knot.out_ctrl, doc(260.0 + along, 60.0 + along)) && near(knot.in_ctrl, doc(260.0 - along, 60.0 - along)), "{knot:?}");
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
fn remove_last_is_the_hosts_undo() {
    let mut pen = Pen::three();
    assert!(pen.fixture.tool_mut().remove_last());
    pen.sync();
    assert_eq!(pen.draft().knots.len(), 2);
    assert!(pen.fixture.tool_mut().remove_last());
    assert!(pen.fixture.tool_mut().remove_last());
    pen.sync();
    assert!(pen.finished() && pen.draft().knots.is_empty());
    assert!(matches!(pen.log.iter().rev().nth(1), Some(PenEvent::Edit(EditEvent::Cancel { .. }))));
    assert!(!pen.fixture.tool_mut().remove_last(), "nothing to remove without a draft");
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

/// A context over the fixture's canvas with the pointer at `at`.
fn context_at<'a>(pen: &Pen, at: Pos2) -> ToolContext<'a> {
    let room = pen.fixture.canvas().rect;
    let space = CanvasSpace::new((pen.view)(room), 1.0);
    ToolContext::new(room, Some(space)).pointer(Some(ToolPointer::new(at, ToolButton::Primary, 0.0)))
}

#[trial]
fn over_the_first_anchor_the_close_cue_shows() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    let first = pen.at(60.0, 60.0);
    assert!(!pen.fixture.tool().closes(&context_at(&pen, first)), "one anchor cannot close");
    pen.click_doc(160.0, 60.0);
    assert!(pen.fixture.tool().closes(&context_at(&pen, first)));
    assert!(!pen.fixture.tool().closes(&context_at(&pen, pen.at(300.0, 250.0))));
    pen.fixture.tool_mut().sensing(false);
    assert!(!pen.fixture.tool().closes(&context_at(&pen, first)), "no cue while the pen takes no press");
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

fn rotated_flipped(room: Rect) -> CanvasView {
    CanvasView::new(room, MIDDLE, 1.0).rotation(30.0).flip_x(true)
}

#[trial]
fn the_first_anchor_takes_the_press_within_the_hit_reach_in_a_rotated_flipped_view() {
    let mut pen = pen_with(PenTool::new(), rotated_flipped);
    for (x, y) in [(60.0, 60.0), (160.0, 60.0), (160.0, 160.0)] {
        pen.click_doc(x, y);
    }
    let first = pen.at(60.0, 60.0);
    pen.click(first + vec2(1.0, 1.0), Modifiers::NONE);
    assert!(pen.finished(), "the press closed the path: {:?}", pen.kinds());
    assert!(pen.filed().is_some_and(|path| path.closed && path.knots.len() == 3));
}

#[trial]
fn a_guide_under_the_press_keeps_it() {
    let mut guides = vec![Guide::new(GuideKey::new(1), craft_behavior::Axis::Y, 100.0)];
    let body = move |ui: &mut egui::Ui, body: &mut craft_ui::contract::BodyCx<'_>| {
        let _ = Guides::new("Guides", Bind::bound(&mut guides)).interactive(true).show_at(ui, body.rect);
    };
    let tool = PenTool::new();
    let mut fixture = ToolFixture::with_view("Pen", tool, view_of, body);
    let guide = view_of(fixture.canvas().rect).to_screen(doc(100.0, 150.0));
    fixture.press_at(guide).release_at(guide);
    assert!(fixture.tool_mut().take_events().is_empty(), "the guide took the press");
    let away = view_of(fixture.canvas().rect).to_screen(doc(200.0, 150.0));
    fixture.press_at(away).release_at(away);
    assert!(!fixture.tool_mut().take_events().is_empty(), "a press away from the guide reaches the pen");
}

#[trial]
fn a_quick_drag_in_one_frame_pulls_the_handles() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    let (from, to) = (pen.at(160.0, 60.0), pen.at(160.0, 60.0) + vec2(40.0, 30.0));
    pen.fixture.fixture().frame(vec![
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
    let path = EditSubpath { knots: vec![Knot::corner(doc(300.0, 250.0)), Knot::corner(doc(360.0, 200.0))], closed: false };
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
fn enter_after_an_options_bar_change_ends_the_draft_and_every_anchor_stays() {
    let mut pen = Pen::three();
    pen.fixture.tool_mut().path_options(options(true, Some([200, 0, 0]), Some(2.0)));
    pen.click_doc(260.0, 160.0);
    assert_eq!(pen.draft().knots.len(), 4, "the anchors before and after the change are one draft");
    pen.key(Key::Enter, Modifiers::NONE);
    assert!(pen.finished());
    assert_eq!(pen.filed().map(|path| path.knots.len()), Some(4));
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
fn a_pen_that_does_not_sense_takes_no_press() {
    let mut pen = pen();
    pen.click_doc(60.0, 60.0);
    pen.fixture.tool_mut().sensing(false);
    pen.click_doc(160.0, 60.0);
    assert_eq!(pen.draft().knots.len(), 1);
    pen.key(Key::Enter, Modifiers::NONE);
    assert!(!pen.finished(), "one anchor ends as no path, but the keys still reach the tool");
    pen.fixture.tool_mut().sensing(true);
    pen.click_doc(160.0, 60.0);
    assert_eq!(pen.draft().knots.len(), 1, "the draft ended; this click starts another");
}

#[trial]
fn the_rubber_band_follows_the_path_options() {
    let shapes = |rubber_band: bool| {
        let mut pen = pen();
        pen.fixture.tool_mut().path_options(options(rubber_band, None, None));
        pen.click_doc(60.0, 60.0);
        let far = pen.at(300.0, 200.0);
        pen.move_to(far, Modifiers::NONE);
        pen.fixture.fixture().idle(1);
        pen.fixture.fixture().shape_bounds().len()
    };
    assert!(shapes(true) > shapes(false), "the dashed band is drawn only when the option is on");
}

#[trial]
fn the_draft_is_a_group_of_named_anchors_for_assistive_technology() {
    let pen = Pen::three();
    let cx = context_at(&pen, pen.at(0.0, 0.0));
    let access = pen.fixture.tool().access(&cx).expect("a draft has a node");
    assert_eq!(access.anchors.len(), 3);
    assert_eq!(access.anchors[2].name, "Anchor 3 of 3");
    assert_eq!(access.anchors[2].at, doc(160.0, 160.0));
    assert!(access.anchors[2].description.as_deref().is_some_and(|text| text.contains("160")));
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

#[trial]
fn curvature_remove_last_removes_the_last_anchor_and_reflows() {
    let mut pen = three_curved();
    assert!(pen.fixture.tool_mut().remove_last());
    pen.sync();
    let draft = pen.draft();
    assert_eq!(draft.knots.len(), 2);
    let mut reflowed = draft.clone();
    reflowed.reflow();
    assert_eq!(draft, reflowed);
}

#[trial]
fn curvature_hover_cues_follow_sensing() {
    let mut pen = three_curved();
    let first = pen.at(60.0, 60.0);
    assert!(pen.fixture.tool().closes(&context_at(&pen, first)), "three anchors can close");
    pen.fixture.tool_mut().sensing(false);
    assert!(!pen.fixture.tool().closes(&context_at(&pen, first)));
}

#[trial]
fn curvature_previews_the_next_curve_whatever_the_rubber_band_setting() {
    let shapes = |rubber_band: bool| {
        let mut pen = curvature();
        pen.fixture.tool_mut().path_options(options(rubber_band, None, None));
        pen.click_doc(60.0, 60.0);
        pen.click_doc(160.0, 60.0);
        let far = pen.at(300.0, 200.0);
        pen.move_to(far, Modifiers::NONE);
        pen.fixture.fixture().idle(1);
        pen.fixture.fixture().shape_bounds().len()
    };
    assert_eq!(shapes(true), shapes(false));
    assert!(shapes(false) > 0);
}

#[trial]
fn curvature_ignores_shift_and_alt() {
    let mut plain = curvature();
    let mut held = curvature();
    for (pen, modifiers) in [(&mut plain, Modifiers::NONE), (&mut held, Modifiers::SHIFT | Modifiers::ALT)] {
        pen.click_doc(60.0, 60.0);
        let at = pen.at(160.0, 100.0);
        pen.click(at, modifiers);
    }
    assert_eq!(plain.draft(), held.draft());
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

fn options(rubber_band: bool, color: Option<[u8; 3]>, thickness: Option<f32>) -> PathOptionsSettings {
    serde_json::from_value(serde_json::json!({ "rubberBand": rubber_band, "color": color, "thickness": thickness })).expect("settings")
}

/// `PathMode::resolve` over every row of its table: each tool, drawing or not, Auto Add/Delete on and off, with no
/// key, Shift, Alt and Command.
#[trial]
fn path_mode_resolves_the_pen_tools_and_modifiers() {
    use BezierTool::{AddAnchor, AutoAddDelete, Convert, Curvature, DeleteAnchor, Direct, Show};
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
    assert_eq!(r(T::Pen, none, true, true), mode(Show, pen, true));
    assert_eq!(r(T::Pen, none, false, true), mode(AutoAddDelete, pen, true));
    assert_eq!(r(T::Pen, none, false, false), mode(Show, pen, true));
    assert_eq!(r(T::Pen, alt, false, true), mode(Convert, pen, true));
    for held in [shift, alt] {
        assert_eq!(r(T::Pen, held, true, true), mode(Show, pen, true));
    }
    assert_eq!(r(T::Pen, shift, false, true), mode(Show, pen, true));

    // The Curvature Pen.
    for held in [none, shift, alt] {
        assert_eq!(r(T::Curvature, held, false, true), mode(Curvature, curve, true));
        assert_eq!(r(T::Curvature, held, true, true), mode(Show, curve, true));
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

/// The settings default to Photoshop's, round-trip through JSON and read `{}` as the default.
#[trial]
fn path_options_settings_default_to_photoshops() {
    let default = PathOptionsSettings::default();
    assert!(!default.rubber_band && default.color.is_none() && default.thickness.is_none());
    let set = options(true, Some([1, 2, 3]), Some(2.5));
    let json = serde_json::to_string(&set).expect("serialises");
    assert!(json.contains("rubberBand"));
    assert_eq!(serde_json::from_str::<PathOptionsSettings>(&json).ok(), Some(set));
    assert_eq!(serde_json::from_str::<PathOptionsSettings>("{}").ok(), Some(default));
}

struct Options {
    settings: PathOptionsSettings,
    open: bool,
    asked: Vec<String>,
}

/// The checkbox changes the bound settings; the swatch asks the host for the colour under the popover's key.
#[trial]
fn the_path_options_popover_changes_the_settings_and_asks_for_the_colour() {
    let state = Rc::new(RefCell::new(Options { settings: PathOptionsSettings::default(), open: true, asked: Vec::new() }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut options = shown.borrow_mut();
        let Options { settings, open, asked } = &mut *options;
        let output = PathOptionsPopover::new("Path Options", craft_ui::contract::Bind::bound(settings)).show(ui, open);
        asked.extend(output.pick_color.iter().map(|request| request.key.to_string()));
    });
    fixture.settle();
    let rubber_band = fixture.by_label("Rubber Band").expect("the checkbox is in the popover");
    fixture.click(&rubber_band);
    assert!(state.borrow().settings.rubber_band);
    let color = fixture.by_label("Color").expect("the swatch is in the popover");
    fixture.click(&color);
    assert_eq!(state.borrow().asked, [PATH_OPTIONS_COLOR_KEY]);
}
