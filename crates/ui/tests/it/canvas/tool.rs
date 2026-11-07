//! `CanvasTool`, `ToolHost` and `tool_pick` (8.3): a drag is one `Begin`...`Commit`, Escape cancels it once, a press
//! on a canvas control never reaches the tool, and a group's letter picks and cycles its tools.

use craft_behavior::{CancelReason, GesturePhase, PointerButton};
use craft_keymap::Mods;
use craft_ui::canvas::{CanvasTool, CropOverlay, PickStep, ToolAction, ToolContext, ToolPath, ToolPointer, tool_pick};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::primitives::Draw;
use craft_ui::rails::{LastUsed, RailTool};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::testing::{ToolFixture, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::RefCell;
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
}

impl Recorder {
    fn count(&self, call: Call) -> usize {
        self.calls.iter().filter(|seen| **seen == call).count()
    }
}

impl CanvasTool for Recorder {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        self.calls.push(Call::Press);
        ToolAction::Claimed
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {
        self.calls.push(Call::Drag);
    }

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {
        self.calls.push(Call::Release);
    }

    fn cancel(&mut self, reason: CancelReason, _: &mut ToolContext<'_>) {
        self.calls.push(Call::Cancel(reason));
    }

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn takes(&self, button: PointerButton, _: Mods) -> bool {
        button == PointerButton::Primary
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
fn a_middle_button_press_does_not_reach_a_tool_that_refuses_it() {
    let mut fixture = ToolFixture::new("Document", Recorder::default());
    let from = fixture.canvas().center();
    fixture.middle_drag_from(from, BY);
    assert!(fixture.tool().calls.is_empty(), "the middle button belongs to the canvas: {:?}", fixture.tool().calls);
    assert!(fixture.gestures().is_empty(), "no gesture began: {:?}", fixture.gestures());
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
