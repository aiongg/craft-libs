//! `CanvasTool`, `ToolHost` and `tool_pick` (8.3): a drag is one `Begin`...`Commit`, Escape cancels it once, a press
//! on a canvas control never reaches the tool, and a group's letter picks and cycles its tools.

use craft_behavior::{CancelReason, GesturePhase, PointerButton};
use craft_keymap::{Chord, Key, Mods};
use craft_ui::canvas::{
    Canvas, CanvasTool, CropOverlay, PickStep, ToolAction, ToolContext, ToolHost, ToolInput, ToolKey, ToolPath, ToolPointer, tool_pick,
};
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::primitives::Draw;
use craft_ui::rails::{LastUsed, RailTool};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::runtime::env::Label;
use craft_ui::testing::{Fixture, ToolFixture, focus, input, trial};
use egui::{Id, Modifiers, Pos2, Rect, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
    /// Keeps the plain keys. The tool takes every key it is offered either way.
    keep: bool,
    got: Vec<ToolKey>,
}

impl Recorder {
    fn count(&self, call: Call) -> usize {
        self.calls.iter().filter(|seen| **seen == call).count()
    }
}

impl CanvasTool for Recorder {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        if self.keep {
            cx.keep_keys();
        }
    }

    fn press(&mut self, pointer: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        self.calls.push(Call::Press);
        self.press_time = Some(pointer.time);
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.calls.push(Call::Drag);
        self.drag_times.push(pointer.time);
        self.space.push(cx.held(Key::Space));
        if self.readout {
            cx.readout(vec![(Label::Static("Width"), "120 px".to_owned())]);
        }
    }

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {
        self.calls.push(Call::Release);
    }

    fn cancel(&mut self, reason: CancelReason, _: &mut ToolContext<'_>) {
        self.calls.push(Call::Cancel(reason));
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
    let crop = Rc::new(RefCell::new(Rect::from_min_max(Pos2::new(20.0, 20.0), Pos2::new(100.0, 80.0))));
    let shown = Rc::clone(&crop);
    let mut fixture = ToolFixture::with_body("Document", Recorder::default(), move |ui, body| {
        let _ = CropOverlay::new("Crop", Bind::bound(&mut *shown.borrow_mut())).show_at(ui, body.rect);
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
            let canvas = Canvas::new("Document", ToolPath::Command("the Select Area... command")).id_salt(salt).show(ui, |_, _| {});
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
    fixture.fixture().key(egui::Key::Delete, Modifiers::NONE);
    fixture.fixture().key(egui::Key::Escape, Modifiers::NONE);
    let got = fixture.tool().got.clone();
    assert!(got.iter().any(|key| matches!(key, ToolKey::Nudge { direction, .. } if *direction == Vec2::X)), "an arrow nudges: {got:?}");
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
