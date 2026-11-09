//! `CanvasTool`, `ToolHost` and `tool_pick` (8.3): a drag is one `Begin`...`Commit`, Escape cancels it once, a press
//! on a canvas control never reaches the tool, and a group's letter picks and cycles its tools.

use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{Axis, CancelReason, CanvasView, GestureId, GesturePhase, PointerButton};
use craft_i18n::Direction;
use craft_keymap::{Chord, Key, Mods};
use craft_ui::canvas::{
    Canvas, CanvasTool, CropOverlay, PickStep, Snap, ToolAccess, ToolAction, ToolAnchor, ToolClaims, ToolContext, ToolHost, ToolInput, ToolKey,
    ToolPath, ToolPointer, tool_pick,
};
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::primitives::Draw;
use craft_ui::rails::{LastUsed, RailTool};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::runtime::env::{Env, Label, UnitWords};
use craft_ui::testing::{Fixture, Frames, ToolFixture, focus, input, trial};
use egui::{Id, Modifiers, Pos2, Rect, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Call {
    Press,
    Drag,
    Release,
    Cancel(CancelReason),
}

/// A tool that claims every press it takes and records what the host called.
#[derive(Default)]
struct Recorder {
    calls: Vec<Call>,
    /// The time of the press and of every drag sample.
    press_time: Option<f64>,
    drag_times: Vec<f64>,
    /// Whether Space was held at each drag.
    space: Vec<bool>,
    /// Shows a readout on every drag.
    readout: bool,
    /// Keeps the plain keys. The tool claims the deletes and takes every key it is offered either way.
    keep: bool,
    got: Vec<ToolKey>,
    /// The gesture id each call read from its context, in the order of `calls`.
    ids: Vec<Option<GestureId>>,
    /// What every pass's context said about the interface: the gesture id, the reading direction and the icon side.
    passes: Vec<(Option<GestureId>, Direction, f32)>,
}

impl Recorder {
    fn count(&self, call: Call) -> usize {
        self.calls.iter().filter(|seen| **seen == call).count()
    }
}

impl CanvasTool for Recorder {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        self.passes.push((cx.gesture_id(), cx.direction(), cx.icon_side()));
    }

    fn claims(&self) -> ToolClaims {
        let deletes = ToolClaims::NONE.deletes();
        if self.keep { deletes.keyboard() } else { deletes }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.calls.push(Call::Press);
        self.ids.push(cx.gesture_id());
        self.press_time = Some(pointer.time);
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.calls.push(Call::Drag);
        self.ids.push(cx.gesture_id());
        self.drag_times.push(pointer.time);
        self.space.push(cx.held(Key::Space));
        if self.readout {
            cx.readout(vec![(Label::Static("Width"), "120 px".to_owned())]);
        }
    }

    fn release(&mut self, _: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.calls.push(Call::Release);
        self.ids.push(cx.gesture_id());
    }

    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>) {
        self.calls.push(Call::Cancel(reason));
        self.ids.push(cx.gesture_id());
    }

    fn key(&mut self, key: &ToolKey, _: &mut ToolContext<'_>) -> bool {
        self.got.push(key.clone());
        true
    }

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    /// The primary button, and the secondary while Alt is held, as a brush takes them.
    fn takes(&self, button: PointerButton, mods: Mods) -> bool {
        button == PointerButton::Primary || (button == PointerButton::Secondary && mods.alt)
    }

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Select Area... command")
    }
}

const BY: Vec2 = Vec2::new(60.0, 40.0);

#[trial]
fn a_drag_reaches_the_tool_as_one_press_drags_and_one_release() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    fixture.drag(BY);
    let tool = fixture.tool();
    assert_eq!(tool.calls.first(), Some(&Call::Press), "a drag starts with the press: {:?}", tool.calls);
    assert_eq!(tool.calls.last(), Some(&Call::Release), "a drag ends with the release: {:?}", tool.calls);
    assert_eq!(tool.count(Call::Press), 1, "one press: {:?}", tool.calls);
    assert_eq!(tool.count(Call::Release), 1, "one release: {:?}", tool.calls);
    assert!(tool.count(Call::Drag) >= 1, "the moves reach the tool: {:?}", tool.calls);
    drop(tool);
    let phases = fixture.gestures();
    assert_eq!(phases.first(), Some(&GesturePhase::Begin), "{phases:?}");
    assert_eq!(phases.last(), Some(&GesturePhase::Commit), "{phases:?}");
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Begin).count(), 1, "one Begin: {phases:?}");
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Commit).count(), 1, "one Commit: {phases:?}");
}

#[trial]
fn the_tool_fixture_checks_pass_for_a_tool_with_a_command_form() {
    ToolFixture::new("Document", Recorder::default()).check();
}

#[trial]
fn escape_during_a_drag_cancels_once_and_a_second_escape_does_nothing() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let from = fixture.canvas().center();
    fixture.drag_and_escape_from(from, BY);
    assert_eq!(fixture.tool().calls.last(), Some(&Call::Cancel(CancelReason::Escape)), "nothing reaches the tool after the cancel");
    assert_eq!(fixture.tool().count(Call::Release), 0, "a cancelled drag has no release: {:?}", fixture.tool().calls);
    fixture.escape();
    assert_eq!(fixture.tool().count(Call::Cancel(CancelReason::Escape)), 1, "a second Escape cancels nothing: {:?}", fixture.tool().calls);
}

#[trial]
fn a_press_on_a_canvas_control_never_reaches_the_tool_and_one_away_from_it_does() {
    let crop = Rc::new(RefCell::new(craft_behavior::geometry::CropBox::upright(craft_behavior::kurbo::Rect::new(20.0, 20.0, 100.0, 80.0))));
    let shown = Rc::clone(&crop);
    let mut fixture = ToolFixture::with_body("Document", Recorder::default(), move |ui, body| {
        // The overlay takes the canvas's top left only: its whole-frame and turn parts cover nothing past it, where the
        // tool owns the press.
        let room = Rect::from_min_size(body.rect.min, Vec2::new(160.0, 120.0));
        let _ = CropOverlay::new("Crop", Bind::bound(&mut *shown.borrow_mut())).show_at(ui, room);
    });
    let corner = fixture.fixture().by_label("Crop corner, top left").expect("the crop shows its corners");
    fixture.fixture().click(&corner);
    assert!(fixture.tool().calls.is_empty(), "a click on a corner is the crop's: {:?}", fixture.tool().calls);

    let away = fixture.canvas().rect.right_bottom() - Vec2::splat(20.0);
    fixture.press_at(away).release_at(away);
    assert_eq!(fixture.tool().count(Call::Press), 1, "a press away from the corners reaches the tool: {:?}", fixture.tool().calls);
}

#[trial]
fn a_button_the_tool_does_not_take_starts_no_gesture_until_its_modifier_is_held() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let from = fixture.canvas().center();
    fixture.secondary_drag_from(from, BY);
    assert!(fixture.tool().calls.is_empty(), "the secondary button alone is refused: {:?}", fixture.tool().calls);
    assert!(fixture.gestures().is_empty(), "no gesture began: {:?}", fixture.gestures());

    fixture.fixture().hold(Modifiers::ALT);
    fixture.secondary_drag_from(from, BY);
    assert_eq!(fixture.tool().count(Call::Press), 1, "the secondary button with Alt is taken: {:?}", fixture.tool().calls);
    assert_eq!(fixture.tool().count(Call::Release), 1, "and the gesture ends with its button: {:?}", fixture.tool().calls);
}

#[trial]
fn drag_samples_carry_the_time_of_their_pass() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    fixture.drag(BY);
    let tool = fixture.tool();
    let press = tool.press_time.expect("the tool was pressed");
    assert!(tool.drag_times.len() >= 2, "the drag has several samples: {:?}", tool.drag_times);
    assert!(tool.drag_times.windows(2).all(|pair| pair[0] < pair[1]), "each sample is from a later pass: {:?}", tool.drag_times);
    assert!(tool.drag_times.iter().all(|time| *time > press), "drag samples are later than the press at {press}: {:?}", tool.drag_times);
}

#[trial]
fn space_pressed_mid_drag_is_held_for_the_tool_and_withheld_from_the_app() {
    let reached = Rc::new(Cell::new(false));
    let seen = Rc::clone(&reached);
    let mut fixture = ToolFixture::with_body("Document", Recorder::default(), move |ui, _| {
        seen.set(seen.get() || input::key_reached(ui.ctx(), egui::Key::Space));
    });
    let from = fixture.canvas().center();
    fixture.press_at(from).move_to(from + Vec2::new(10.0, 0.0));
    assert_eq!(fixture.tool().space.last(), Some(&false), "Space is not down yet");

    reached.set(false);
    fixture.fixture().frame(vec![input::key_press(egui::Key::Space, Modifiers::NONE)]);
    fixture.move_to(from + Vec2::new(20.0, 0.0));
    assert_eq!(fixture.tool().space.last(), Some(&true), "Space pressed mid-drag is held for the tool");
    assert!(!reached.get(), "the app's shortcuts, a momentary Hand among them, never see that press");

    fixture.fixture().frame(vec![focus::key_release(egui::Key::Space, Modifiers::NONE)]);
    fixture.move_to(from + Vec2::new(30.0, 0.0));
    assert_eq!(fixture.tool().space.last(), Some(&false), "Space released mid-drag is no longer held");

    fixture.release_at(from + Vec2::new(30.0, 0.0));
    reached.set(false);
    fixture.fixture().frame(vec![input::key_press(egui::Key::Space, Modifiers::NONE)]);
    assert!(reached.get(), "with no gesture running, Space reaches the app again");
}

#[trial]
fn a_tool_that_asks_for_a_readout_has_it_painted_without_an_app_call() {
    let mut fixture = ToolFixture::new("Document", Recorder { readout: true, ..Recorder::default() });
    let from = fixture.canvas().center();
    fixture.press_at(from).move_to(from + Vec2::new(20.0, 10.0));
    let painted = |fixture: &mut ToolFixture<Recorder>| fixture.fixture().texts().iter().any(|text| text.text == "Width");
    assert!(painted(&mut fixture), "the readout shows while the tool asks for it");
    fixture.release_at(from + Vec2::new(20.0, 10.0));
    assert!(!painted(&mut fixture), "and goes with the gesture");
}

#[trial]
fn a_host_enabled_again_mid_press_starts_no_new_gesture() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let from = fixture.canvas().center();
    fixture.press_at(from);
    fixture.enabled(false).move_to(from + Vec2::new(10.0, 0.0));
    assert_eq!(fixture.tool().calls.last(), Some(&Call::Cancel(CancelReason::Dismissed)), "disabling cancels the gesture");
    let before = fixture.tool().calls.len();

    fixture.enabled(true).move_to(from + Vec2::new(20.0, 0.0)).move_to(from + Vec2::new(30.0, 0.0));
    fixture.release_at(from + Vec2::new(30.0, 0.0));
    assert_eq!(fixture.tool().calls.len(), before, "the press the host was disabled during is spent: {:?}", fixture.tool().calls);

    fixture.drag(BY);
    assert_eq!(fixture.tool().count(Call::Press), 2, "the next press starts a gesture again: {:?}", fixture.tool().calls);
}

#[trial]
fn two_canvases_with_the_same_label_run_separate_gestures() {
    let tools = [Rc::new(RefCell::new(Recorder::default())), Rc::new(RefCell::new(Recorder::default()))];
    let ids = Rc::new(RefCell::new([Id::NULL; 2]));
    let (shown, seen) = (tools.clone(), Rc::clone(&ids));
    let mut fixture = Fixture::new(move |ui| {
        for (at, salt) in ["first", "second"].into_iter().enumerate() {
            let canvas = Canvas::new("Document").id_salt(salt).show(ui, |_, _| {});
            seen.borrow_mut()[at] = canvas.output.response.id;
            let _ = ToolHost::new().run(ui, &canvas.output, &mut *shown[at].borrow_mut());
        }
    });
    fixture.settle();
    let [first, second] = *ids.borrow();
    assert_ne!(first, second, "two canvases have two ids");

    ToolHost::inject(fixture.ctx(), first, ToolInput::down(PointerButton::Primary, Pos2::new(10.0, 10.0)));
    fixture.frame(Vec::new());
    assert!(ToolHost::in_gesture(fixture.ctx(), first), "the first canvas has a gesture");
    assert!(!ToolHost::in_gesture(fixture.ctx(), second), "the second canvas, with the same label, has none");
    assert_eq!(tools[1].borrow().calls, [], "and its tool was not called");
}

#[trial]
fn injected_input_runs_a_gesture_on_the_canvas_it_names() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let (ctx, id) = (fixture.fixture().ctx().clone(), fixture.canvas_id());
    ToolHost::inject(&ctx, id, ToolInput::down(PointerButton::Primary, Pos2::new(40.0, 40.0)));
    fixture.fixture().frame(Vec::new());
    assert!(ToolHost::in_gesture(&ctx, id), "an injected press starts the gesture");
    ToolHost::inject(&ctx, id, ToolInput::moved(Pos2::new(60.0, 50.0)));
    fixture.fixture().frame(Vec::new());
    ToolHost::inject(&ctx, id, ToolInput::up(Pos2::new(60.0, 50.0)));
    fixture.fixture().frame(Vec::new());
    assert!(!ToolHost::in_gesture(&ctx, id), "an injected release ends it");
    assert_eq!(fixture.tool().calls, [Call::Press, Call::Drag, Call::Release]);
    assert_eq!(fixture.gestures(), [GesturePhase::Begin, GesturePhase::Update, GesturePhase::Commit]);
}

#[trial]
fn cancelling_by_canvas_id_ends_the_gesture_without_a_commit() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let from = fixture.canvas().center();
    fixture.press_at(from);
    let (ctx, id) = (fixture.fixture().ctx().clone(), fixture.canvas_id());
    assert!(ToolHost::cancel(&ctx, id, &mut *fixture.tool_mut(), CancelReason::Removed), "a gesture was running");
    assert_eq!(fixture.tool().calls.last(), Some(&Call::Cancel(CancelReason::Removed)));
    assert!(!ToolHost::in_gesture(&ctx, id));
    assert!(!ToolHost::cancel(&ctx, id, &mut *fixture.tool_mut(), CancelReason::Removed), "a second cancel finds nothing");
}

#[trial]
fn the_keys_a_canvas_holds_reach_the_tool_while_it_has_focus() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let at = fixture.canvas().center();
    fixture.press_at(at).release_at(at);
    fixture.fixture().key(egui::Key::ArrowRight, Modifiers::NONE);
    fixture.fixture().key(egui::Key::ArrowDown, Modifiers::SHIFT);
    fixture.fixture().key(egui::Key::Delete, Modifiers::NONE);
    fixture.fixture().key(egui::Key::Enter, Modifiers::NONE);
    fixture.fixture().key(egui::Key::Escape, Modifiers::NONE);
    let got = fixture.tool().got.clone();
    let nudged = |direction: Vec2, shift: bool| {
        got.iter().any(|key| matches!(key, ToolKey::Nudge { direction: d, mods, .. } if *d == direction && mods.shift == shift))
    };
    assert!(nudged(Vec2::X, false), "an arrow nudges by its direction: {got:?}");
    assert!(nudged(Vec2::Y, true), "Shift with an arrow is the far nudge: {got:?}");
    assert!(got.contains(&ToolKey::Submit), "Enter submits: {got:?}");
    assert!(got.iter().any(|key| matches!(key, ToolKey::Delete { forward: true, .. })), "Delete deletes forward: {got:?}");
    assert!(got.contains(&ToolKey::Cancel), "Escape with no gesture reaches the tool as Cancel: {got:?}");
}

#[trial]
fn a_tool_that_keeps_the_keys_gets_plain_presses_as_kept() {
    let mut fixture = ToolFixture::new("Document", Recorder { keep: true, ..Recorder::default() });
    let at = fixture.canvas().center();
    fixture.press_at(at).release_at(at);
    fixture.fixture().key(egui::Key::B, Modifiers::NONE);
    let b = Chord::new(Mods::NONE, Key::B);
    assert!(fixture.tool().got.contains(&ToolKey::Kept(b)), "a plain B is the tool's: {:?}", fixture.tool().got);
}

const UNDO: u8 = 1;
const REDO: u8 = 2;
const CUT: u8 = 3;
const OUTSIDE: u8 = 9;

fn group() -> Vec<RailTool<u8>> {
    vec![RailTool::new(UNDO, EditCommands::Undo), RailTool::new(REDO, EditCommands::Redo), RailTool::new(CUT, EditCommands::Cut)]
}

fn pick(group: &[RailTool<u8>], current: u8, last: &LastUsed<u8>, step: PickStep, disabled: &[u8]) -> Option<u8> {
    let tools: Vec<&RailTool<u8>> = group.iter().collect();
    tool_pick(&tools, 0, &current, last, step, |tool| !disabled.contains(&tool.id)).map(|tool| tool.id)
}

#[trial]
fn tool_pick_selects_the_last_used_tool_and_cycles_through_the_enabled_ones() {
    let tools = group();
    let mut last = LastUsed::default();
    last.set(0, REDO);
    assert_eq!(pick(&tools, OUTSIDE, &last, PickStep::Last, &[]), Some(REDO), "a plain letter picks the group's last-used tool");
    assert_eq!(pick(&tools, CUT, &last, PickStep::Last, &[]), Some(CUT), "a plain letter keeps the current tool of the group");
    assert_eq!(pick(&tools, OUTSIDE, &last, PickStep::Next, &[]), Some(REDO), "the cycle chord enters the group at its last-used tool");

    assert_eq!(pick(&tools, UNDO, &last, PickStep::Next, &[]), Some(REDO));
    assert_eq!(pick(&tools, REDO, &last, PickStep::Next, &[]), Some(CUT));
    assert_eq!(pick(&tools, CUT, &last, PickStep::Next, &[]), Some(UNDO), "the cycle wraps");

    assert_eq!(pick(&tools, UNDO, &last, PickStep::Next, &[REDO]), Some(CUT), "a disabled tool is skipped");
    assert_eq!(pick(&tools, UNDO, &last, PickStep::Next, &[REDO, CUT]), Some(UNDO), "a group of one enabled tool stays on it");
    assert_eq!(pick(&tools, OUTSIDE, &last, PickStep::Last, &[UNDO, REDO, CUT]), None, "a group with no enabled tool picks nothing");
}

/// A rectangle tool that describes the size it is dragging.
#[derive(Default)]
struct Sizer {
    from: Option<Pos2>,
    to: Option<Pos2>,
}

impl CanvasTool for Sizer {
    fn press(&mut self, pointer: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        self.from = Some(pointer.pos);
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, _: &mut ToolContext<'_>) {
        self.to = Some(pointer.pos);
    }

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {
        self.from = None;
    }

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {
        self.from = None;
    }

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn access(&self, cx: &ToolContext<'_>) -> Option<ToolAccess> {
        let (from, to) = (self.from?, self.to?);
        let units = cx.units();
        let size = to - from;
        let value = format!("{} x {}", units.pixels(f64::from(size.x)), units.pixels(f64::from(size.y)));
        let view = cx.space()?.view;
        let (top_left, bottom_right) = (view.to_doc(from), view.to_doc(to));
        Some(
            ToolAccess::new("Rectangle")
                .value(value)
                .bounds(DocRect::from_points(top_left, bottom_right))
                .anchor(ToolAnchor::new("Start", top_left))
                .anchor(ToolAnchor::new("End", bottom_right)),
        )
    }

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Rectangle command")
    }
}

#[trial]
fn a_drag_tool_node_values_its_live_size_and_goes_with_the_gesture() {
    let view = |room: Rect| CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut fixture = ToolFixture::with_view("Document", Sizer::default(), view, |_, _| {});
    let from = fixture.canvas().center();
    fixture.press_at(from).move_to(from + BY);
    let value = |fixture: &mut ToolFixture<Sizer>| {
        let node = fixture.fixture().by_label("Rectangle")?;
        fixture.fixture().tree().value_of(node.node).map(str::to_owned)
    };
    let node = fixture.fixture().by_label("Rectangle").expect("the tool has a node");
    assert!(node.rect.width() > 0.0, "the node covers the drawing: {:?}", node.rect);
    assert_eq!(value(&mut fixture).as_deref(), Some("60 px x 40 px"), "the node's value is the size it is dragging");
    assert!(fixture.fixture().by_label("Start").is_some() && fixture.fixture().by_label("End").is_some(), "each anchor has a node");
    fixture.move_to(from + BY * 2.0);
    assert_eq!(value(&mut fixture).as_deref(), Some("120 px x 80 px"), "the value follows the drag");
    fixture.release_at(from + BY * 2.0);
    assert!(fixture.fixture().by_label("Rectangle").is_none(), "a tool with nothing to describe has no node");
}

/// A tool that reads the snapping and the words through its context on a press.
#[derive(Default)]
struct Reader {
    snapped: Option<Point>,
    unit_words: Option<Arc<UnitWords>>,
}

impl CanvasTool for Reader {
    fn press(&mut self, _: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.snapped = Some(cx.snap(Point::new(10.3, 7.9)));
        self.unit_words = Some(cx.words::<UnitWords>());
        ToolAction::Done
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Reader command")
    }
}

/// Presses `Reader` once on a canvas that snaps to a guide at x = 10 when `snapping`; returns what the tool read
/// and the unit words of the env the tool was shown in.
fn press_reader(snapping: bool) -> (Reader, Arc<UnitWords>) {
    let tool = Rc::new(RefCell::new(Reader::default()));
    let words = Rc::new(RefCell::new(None));
    let canvas = Rc::new(Cell::new(Id::NULL));
    let press = Rc::new(Cell::new(false));
    let (shown, seen, id, queue) = (Rc::clone(&tool), Rc::clone(&words), Rc::clone(&canvas), Rc::clone(&press));
    let mut fixture = Fixture::new(move |ui| {
        let guide = Snap::new(|along, position, _| if along == Axis::Y && (position - 10.0).abs() < 1.0 { 10.0 } else { position });
        let canvas = Canvas::new("Document");
        let out = if snapping { canvas.snap(guide) } else { canvas }.show(ui, |_, _| {});
        id.set(out.output.response.id);
        let _ = ToolHost::new().run(ui, &out.output, &mut *shown.borrow_mut());
        *seen.borrow_mut() = Some(Env::of(ui).words::<UnitWords>());
        if queue.take() {
            let at = out.output.response.rect.center();
            ToolHost::inject(ui.ctx(), id.get(), ToolInput::down(PointerButton::Primary, at));
        }
    });
    fixture.settle();
    press.set(true);
    fixture.idle(3);
    let reader = std::mem::take(&mut *tool.borrow_mut());
    let words = words.borrow_mut().take().expect("the env of the pass");
    (reader, words)
}

#[trial]
fn a_tool_snaps_a_point_through_its_context_and_reads_the_words_of_the_env() {
    let (reader, env_words) = press_reader(true);
    assert_eq!(reader.snapped, Some(Point::new(10.0, 7.9)), "the point snaps to the guide the canvas was given, on the axis it governs");
    let words = reader.unit_words.expect("the press reached the tool");
    assert!(Arc::ptr_eq(&words, &env_words), "the tool reads the words of the env it is shown in");
    let (reader, _) = press_reader(false);
    assert_eq!(reader.snapped, Some(Point::new(10.3, 7.9)), "a canvas given no snapping leaves the point");
}

/// A tool that shows one outline of marching ants.
struct Outline;

impl CanvasTool for Outline {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn paint(&self, _: &Draw<'_>, cx: &ToolContext<'_>) {
        let corners = [Pos2::new(40.0, 40.0), Pos2::new(160.0, 40.0), Pos2::new(160.0, 120.0), Pos2::new(40.0, 120.0)];
        cx.marching_ants(&corners, true);
    }

    fn command_form(&self) -> ToolPath {
        ToolPath::Freehand("the outline is fixed; no command changes it")
    }
}

#[trial]
fn a_tools_marching_ants_step_with_the_clock_and_ask_for_the_next_repaint() {
    let mut outline = Outline;
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        let shown = Canvas::new("Document").show(ui, |_, _| ());
        let _ = ToolHost::new().run(ui, &shown.output, &mut outline);
    };
    frames.idle(3, &mut show);
    let before = frames.frame_shapes(&mut show);
    frames.advance(MARCH_STEP);
    let after = frames.frame_shapes(&mut show);
    assert_ne!(format!("{before:?}"), format!("{after:?}"), "the ants moved one step in a march step of the clock");
    assert!(frames.repaint_delay().is_some_and(|wait| wait <= Duration::from_secs_f64(MARCH_STEP)), "shown ants ask for the next step");
}

/// Seconds of one march step.
const MARCH_STEP: f64 = 0.1;

#[trial]
fn a_gesture_reads_one_id_from_press_to_release_and_a_cancel_reads_it_too() {
    {
        let mut fixture = ToolFixture::new("Document", Recorder::default());
        fixture.drag(BY);
        fixture.drag(BY);
        let tool = fixture.tool();
        assert_eq!(tool.passes.first().map(|pass| pass.0), Some(None), "no gesture runs before the first press");
        let ids: Vec<GestureId> = tool.ids.iter().map(|id| id.expect("every gesture call reads the id")).collect();
        let (first, second) = ids.split_at(ids.len() / 2);
        assert!(first.len() >= 3 && first.len() == second.len(), "two drags of the same calls: {:?}", tool.calls);
        assert!(first.iter().all(|id| *id == first[0]), "press, drags and release share one id: {first:?}");
        assert!(second.iter().all(|id| *id == second[0]), "and so does the next gesture: {second:?}");
        assert!(second[0] > first[0], "ids grow in the order gestures begin: {first:?} then {second:?}");
    }
    {
        let mut fixture = ToolFixture::new("Document", Recorder::default());
        let from = fixture.canvas().center();
        fixture.drag_and_escape_from(from, BY);
        let tool = fixture.tool();
        let press = tool.ids.first().copied().flatten();
        assert!(press.is_some(), "the press reads an id");
        assert_eq!(tool.ids.last().copied().flatten(), press, "the cancel reads the id of the gesture it ends: {:?}", tool.ids);
    }
}

#[trial]
fn the_tool_reads_the_reading_direction_and_a_theme_icon_side_of_the_pass() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let (direction, compact) = fixture.tool().passes.last().map(|pass| (pass.1, pass.2)).expect("the tool ran a pass");
    assert_eq!(direction, Direction::Ltr, "an interface reads left to right unless told otherwise");
    assert!(compact > 0.0, "the icon side is the theme's");

    fixture.fixture().direction(Direction::Rtl).density(craft_theme::Size::Comfortable).settle();
    let (direction, comfortable) = fixture.tool().passes.last().map(|pass| (pass.1, pass.2)).expect("the tool ran a pass");
    assert_eq!(direction, Direction::Rtl, "the tool reads the interface's direction");
    assert!(comfortable > compact, "the icon side follows the theme's density: {compact} then {comfortable}");
}

/// A tool that moves a point with the keys it is offered.
struct Mover {
    at: Vec2,
}

impl CanvasTool for Mover {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn key(&mut self, key: &ToolKey, _: &mut ToolContext<'_>) -> bool {
        match key {
            ToolKey::Nudge { direction, mods, .. } => {
                self.at += *direction * if mods.shift { 10.0 } else { 1.0 };
                true
            }
            ToolKey::Delete { forward, .. } => {
                self.at = if *forward { Vec2::ZERO } else { Vec2::splat(-1.0) };
                true
            }
            _ => false,
        }
    }

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Move command")
    }
}

#[trial]
fn a_gesture_cancelled_by_the_app_starts_no_new_one_while_the_button_stays_down() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let from = fixture.canvas().center();
    fixture.press_at(from);
    let (ctx, id) = (fixture.fixture().ctx().clone(), fixture.canvas_id());
    assert!(ToolHost::cancel(&ctx, id, &mut *fixture.tool_mut(), CancelReason::Dismissed), "a gesture was running");

    fixture.move_to(from + BY).move_to(from + BY * 2.0).release_at(from + BY * 2.0);
    assert_eq!(fixture.tool().calls, [Call::Press, Call::Cancel(CancelReason::Dismissed)], "the held button is not the tool's any more");

    fixture.press_at(from).release_at(from);
    assert_eq!(fixture.tool().count(Call::Press), 2, "the next press starts a gesture again: {:?}", fixture.tool().calls);
}
