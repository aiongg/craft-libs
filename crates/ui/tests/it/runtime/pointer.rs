//! The sensing core over probe roots: the press lifecycle, arenas, press targets, parts and the shape record.

use craft_behavior::{CanvasTransform, HitLevel, PartKey, PressCancel, PressFocus, PressPhase, Sense};
use craft_ui::testing::pointer::{self, drag, settle};
use craft_ui::testing::sensing::{ProbeMember, ProbePart, ProbeRoot, RootReport, SenseReport, cover, egui_click_target, press_focus, show};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const BACKGROUND: PartKey = PartKey::new("background");
const EDGE: PartKey = PartKey::new("edge");
const PORT: PartKey = PartKey::new("port");
const EYE: PartKey = PartKey::new("eye");
const CORNER: PartKey = PartKey::new("corner");

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

/// The reports of every frame of a scenario.
#[derive(Default)]
struct Log {
    frames: Vec<Vec<RootReport>>,
}

fn press_phases(report: &SenseReport, key: Option<PartKey>) -> Vec<PressPhase> {
    let phase = match key {
        None => report.press.phase,
        Some(key) => report.phase(key),
    };
    phase.is_press().then_some(phase).into_iter().collect()
}

impl Log {
    fn last(&self) -> &RootReport {
        &self.frames[self.frames.len() - 1][0]
    }

    fn root_phases(&self, root: usize, key: Option<PartKey>) -> Vec<PressPhase> {
        self.frames.iter().flat_map(|reports| press_phases(&reports[root].root, key)).collect()
    }

    fn member_phases(&self, root: usize, member: &'static str, key: Option<PartKey>) -> Vec<PressPhase> {
        self.frames.iter().filter_map(|reports| reports[root].member(member)).flat_map(|report| press_phases(report, key)).collect()
    }

    fn member_clicks(&self, root: usize, member: &'static str) -> usize {
        self.frames.iter().filter_map(|reports| reports[root].member(member)).filter(|report| report.clicked).count()
    }

    fn activated_members(&self, root: usize) -> usize {
        self.frames.iter().filter(|reports| reports[root].activated_member.is_some()).count()
    }

    fn root_activations(&self, root: usize) -> usize {
        self.frames.iter().filter(|reports| reports[root].root.activated).count()
    }

    fn clear(&mut self) {
        self.frames.clear();
    }
}

fn drags(phases: &[PressPhase]) -> usize {
    phases.iter().filter(|phase| matches!(phase, PressPhase::DragStart { .. })).count()
}

/// A frame closure that shows `roots` and logs what they read.
fn scene(roots: Vec<ProbeRoot>) -> (impl FnMut(&mut Ui), Rc<RefCell<Log>>) {
    let log = Rc::new(RefCell::new(Log::default()));
    let shared = log.clone();
    let frame = move |ui: &mut Ui| {
        let reports = show(ui, &roots);
        shared.borrow_mut().frames.push(reports);
    };
    (frame, log)
}

fn canvas(transform: Option<CanvasTransform>) -> ProbeRoot {
    let root = ProbeRoot::new("canvas", rect(100.0, 100.0, 500.0, 300.0), Sense::empty())
        .arena()
        .part(ProbePart::new(BACKGROUND, rect(100.0, 100.0, 500.0, 300.0), Sense::DRAG).level(HitLevel::Background, 0))
        .member(ProbeMember::new("tool", rect(160.0, 160.0, 90.0, 30.0), Sense::CLICK | Sense::FOCUS));
    match transform {
        Some(transform) => root.transformed(transform).blocker(Vec2::new(500.0, 300.0)),
        None => root,
    }
}

const ZOOM: CanvasTransform = CanvasTransform::new(2.0, Vec2::new(10.0, 5.0));

/// Skeleton: `frames_pointer::button_on_canvas_click_and_slop_drag` and `..._on_zoomed_canvas_...`.
fn member_on_canvas(transform: Option<CanvasTransform>) {
    let (mut show_frame, log) = scene(vec![canvas(transform)]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    let button = log.borrow().last().to_screen(Pos2::new(205.0, 175.0));
    let root_rect = rect(100.0, 100.0, 500.0, 300.0);
    assert!(root_rect.contains(button), "the button lies on the canvas: {button:?}");
    log.borrow_mut().clear();

    // A click on the member activates the member, not the canvas tool.
    pointer::click(&mut frames, button, &mut show_frame);
    {
        let log = log.borrow();
        assert_eq!(log.member_clicks(0, "tool"), 1, "the member was clicked once");
        assert!(log.root_phases(0, Some(BACKGROUND)).is_empty(), "the tool saw nothing: {:?}", log.root_phases(0, Some(BACKGROUND)));
        assert_eq!(log.root_activations(0), 0, "the root's own activation stays empty");
        assert_eq!(log.activated_members(0), 1, "the root's activated member named the member once");
    }
    if transform.is_some() {
        assert_eq!(
            egui_click_target(&frames.ctx),
            Some(ProbeRoot::transform_widget("canvas")),
            "egui's click target is the forwarding widget on the body layer"
        );
    }

    // A click has no duration limit: a press held for two seconds still activates once.
    frames.frame(vec![pointer::press(button)], &mut show_frame);
    frames.advance(2.0);
    frames.frame(vec![pointer::release(button)], &mut show_frame);
    frames.idle(1, &mut show_frame);
    assert_eq!(log.borrow().member_clicks(0, "tool"), 2, "a long press released within slop is a click");
    log.borrow_mut().clear();

    // A press on the member that moves past the slop starts no canvas drag and no click.
    drag(&mut frames, button, Vec2::new(40.0, 30.0), &mut show_frame);
    assert_eq!(log.borrow().member_clicks(0, "tool"), 0, "a dragged press is no click");
    assert!(log.borrow().root_phases(0, Some(BACKGROUND)).is_empty(), "the canvas tool saw nothing");
    assert_eq!(log.borrow().activated_members(0), 0);

    // Control: a drag that starts on the canvas background starts the tool's drag.
    let free = Pos2::new(160.0, 360.0);
    log.borrow_mut().clear();
    drag(&mut frames, free, Vec2::new(40.0, 30.0), &mut show_frame);
    let phases = log.borrow().root_phases(0, Some(BACKGROUND));
    assert_eq!(drags(&phases), 1, "a drag from the background starts one tool drag: {phases:?}");
    // The root's own parts live in the root's space (the parent layer), so the release lands where the pointer is.
    match phases.last() {
        Some(PressPhase::Release { pos, .. }) => assert!((*pos - (free + Vec2::new(40.0, 30.0))).length() < 0.5, "release at {pos:?}"),
        other => panic!("the drag ended with {other:?}"),
    }
}

#[trial]
fn member_on_zoomed_canvas_click_and_slop_drag() {
    member_on_canvas(Some(ZOOM));
}

/// Skeleton: `frames_pointer::interactive_container_activates_on_its_background_only`.
fn card(arena: bool) {
    let root = ProbeRoot::new("card", rect(100.0, 100.0, 400.0, 300.0), Sense::CLICK | Sense::FOCUS).member(ProbeMember::new(
        "child",
        rect(120.0, 120.0, 100.0, 30.0),
        Sense::CLICK | Sense::FOCUS,
    ));
    let root = if arena { root.arena() } else { root };
    let (mut show_frame, log) = scene(vec![root]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);

    pointer::click(&mut frames, Pos2::new(170.0, 135.0), &mut show_frame);
    assert_eq!(log.borrow().member_clicks(0, "child"), 1, "the child was clicked once (arena: {arena})");
    assert_eq!(log.borrow().root_activations(0), 0, "a press on the body child does not activate the card (arena: {arena})");

    pointer::click(&mut frames, Pos2::new(400.0, 300.0), &mut show_frame);
    assert_eq!(log.borrow().root_activations(0), 1, "a press on the card background activates it once (arena: {arena})");
}

#[trial]
fn interactive_container_activates_on_its_background_only() {
    card(false);
    card(true);
}

/// Skeleton: `frames_pointer::touchpad_tap_activates_one_component`.
fn tap_on_canvas(transform: Option<CanvasTransform>, gone: bool) {
    let (mut show_frame, log) = scene(vec![canvas(transform)]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    let button = log.borrow().last().to_screen(Pos2::new(205.0, 175.0));
    log.borrow_mut().clear();

    // A tap on the member activates it once and nothing else.
    pointer::tap(&mut frames, button, gone, &mut show_frame);
    frames.idle(1, &mut show_frame);
    assert_eq!(log.borrow().member_clicks(0, "tool"), 1, "one tap, one member activation");
    assert_eq!(log.borrow().activated_members(0), 1, "the root names the member once");
    assert!(log.borrow().root_phases(0, Some(BACKGROUND)).is_empty(), "the tool starts no press lifecycle");

    // A tap on the background starts exactly one press lifecycle, on the tool, and activates no member.
    let free = Pos2::new(160.0, 360.0);
    pointer::tap(&mut frames, free, gone, &mut show_frame);
    frames.idle(1, &mut show_frame);
    let phases = log.borrow().root_phases(0, Some(BACKGROUND));
    // The input hook splits a one-frame tap over two passes, so the one lifecycle shows Down then Release.
    assert_eq!(
        phases.iter().filter(|phase| matches!(phase, PressPhase::Down { .. })).count(),
        1,
        "one tap, one press lifecycle on the tool: {phases:?}"
    );
    assert!(matches!(phases.last(), Some(PressPhase::Release { .. })), "{phases:?}");
    assert_eq!(log.borrow().member_clicks(0, "tool"), 1, "the member saw nothing more");
    assert_eq!(log.borrow().activated_members(0), 1);
}

#[trial]
fn touchpad_tap_activates_one_component() {
    tap_on_canvas(None, false);
    tap_on_canvas(Some(ZOOM), false);
    // A touch screen reports the pointer gone in the frame of the lift.
    tap_on_canvas(None, true);
    tap_on_canvas(Some(ZOOM), true);
}

/// Skeleton: `frames_pointer::drag_from_item_part_reaches_the_right_owner`.
#[trial]
fn drag_from_a_member_part_reaches_the_part_and_not_the_member() {
    let node = rect(140.0, 140.0, 140.0, 50.0);
    let graph = ProbeRoot::new("graph", rect(100.0, 100.0, 600.0, 400.0), Sense::CLICK | Sense::DRAG | Sense::FOCUS).arena().member(
        ProbeMember::new("node", node, Sense::CLICK | Sense::DRAG)
            .part(ProbePart::new(PORT, rect(134.0, 144.0, 12.0, 12.0), Sense::DRAG).level(HitLevel::Handle, 0)),
    );
    let (mut show_frame, log) = scene(vec![graph]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    log.borrow_mut().clear();

    // A drag from the port starts one part drag and no move of the node.
    drag(&mut frames, Pos2::new(140.0, 150.0), Vec2::new(60.0, 80.0), &mut show_frame);
    assert_eq!(drags(&log.borrow().member_phases(0, "node", Some(PORT))), 1, "one drag starts from the port");
    assert!(log.borrow().member_phases(0, "node", None).is_empty(), "the port, not the node body, owns the drag");

    // A drag from the node body moves the node: one drag, none from the port.
    log.borrow_mut().clear();
    drag(&mut frames, Pos2::new(210.0, 170.0), Vec2::new(30.0, 20.0), &mut show_frame);
    assert_eq!(drags(&log.borrow().member_phases(0, "node", None)), 1, "the body starts one drag");
    assert!(log.borrow().member_phases(0, "node", Some(PORT)).is_empty());
}

/// Skeleton: `frames_pointer::drag_from_list_row_part_toggles_nothing`.
#[trial]
fn a_click_only_part_clicks_and_a_dragged_press_on_it_does_nothing() {
    let list = ProbeRoot::new("list", rect(100.0, 100.0, 400.0, 200.0), Sense::CLICK | Sense::DRAG | Sense::FOCUS | Sense::TAKE_FOCUS)
        .arena()
        .member(ProbeMember::new("row", rect(100.0, 100.0, 400.0, 24.0), Sense::CLICK | Sense::DRAG).part(ProbePart::new(
            EYE,
            rect(480.0, 102.0, 20.0, 20.0),
            Sense::CLICK,
        )));
    let (mut show_frame, log) = scene(vec![list]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    log.borrow_mut().clear();
    let eye = Pos2::new(490.0, 112.0);

    pointer::click(&mut frames, eye, &mut show_frame);
    let clicks = log.borrow().member_phases(0, "row", Some(EYE)).iter().filter(|phase| phase.is_click()).count();
    assert_eq!(clicks, 1, "a click on the eye part clicks once");

    log.borrow_mut().clear();
    drag(&mut frames, eye, Vec2::new(0.0, 40.0), &mut show_frame);
    let phases = log.borrow().member_phases(0, "row", Some(EYE));
    assert!(
        phases.iter().all(|phase| !phase.is_click() && !phase.is_drag()),
        "a press on a click-only part that moves away neither clicks nor drags: {phases:?}"
    );
    assert!(log.borrow().member_phases(0, "row", None).is_empty(), "the row starts no drag from its part");
}

/// Skeleton: `frames_pointer::press_outside_root_next_to_edge_part_reaches_widget_below`.
#[trial]
fn a_press_beside_an_edge_part_outside_the_root_reaches_the_widget_below() {
    let below = ProbeRoot::new("below", rect(100.0, 100.0, 400.0, 60.0), Sense::CLICK | Sense::FOCUS);
    let root_rect = rect(200.0, 100.0, 200.0, 60.0);
    let root = ProbeRoot::new("root", root_rect, Sense::CLICK | Sense::FOCUS).arena().part(
        ProbePart::new(EDGE, Rect::from_center_size(Pos2::new(root_rect.right(), root_rect.center().y), Vec2::new(6.0, 20.0)), Sense::DRAG)
            .level(HitLevel::Edge, 0),
    );
    let (mut show_frame, log) = scene(vec![below, root]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    log.borrow_mut().clear();

    // Control: a press on the edge part belongs to the root.
    drag(&mut frames, Pos2::new(root_rect.right(), root_rect.center().y), Vec2::new(30.0, 0.0), &mut show_frame);
    assert_eq!(drags(&log.borrow().root_phases(1, Some(EDGE))), 1, "the edge part took its drag");
    assert!(!log.borrow().frames.iter().any(|reports| reports[0].root.clicked), "the widget below saw no click");

    // Just outside the root rect, beside the part: the press reaches the widget below.
    log.borrow_mut().clear();
    pointer::click(&mut frames, Pos2::new(root_rect.right() + 2.0, root_rect.top() + 5.0), &mut show_frame);
    let clicks = log.borrow().frames.iter().filter(|reports| reports[0].root.clicked).count();
    assert_eq!(clicks, 1, "the widget below was clicked");
    assert_eq!(log.borrow().root_activations(1), 0, "the root took no activation");
}

/// Skeleton: `frames_pointer::canvas_control_over_the_canvas_leaves_the_tool_its_press`, with a control that
/// senses nothing but its handle part.
#[trial]
fn a_canvas_control_with_an_empty_root_sense_leaves_the_tool_its_press() {
    let area = rect(100.0, 100.0, 400.0, 300.0);
    let canvas = ProbeRoot::new("canvas", area, Sense::empty())
        .arena()
        .part(ProbePart::new(BACKGROUND, area, Sense::DRAG).level(HitLevel::Background, 0))
        .member(
            ProbeMember::new("crop", area, Sense::empty())
                .part(ProbePart::new(CORNER, rect(154.0, 154.0, 12.0, 12.0), Sense::DRAG).level(HitLevel::Handle, 0)),
        );
    let (mut show_frame, log) = scene(vec![canvas]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    log.borrow_mut().clear();

    // A drag away from the handle goes to the tool: the control's root senses nothing.
    drag(&mut frames, Pos2::new(400.0, 320.0), Vec2::new(30.0, 20.0), &mut show_frame);
    assert_eq!(drags(&log.borrow().root_phases(0, Some(BACKGROUND))), 1, "the tool got its drag");
    assert!(log.borrow().member_phases(0, "crop", Some(CORNER)).is_empty(), "the control saw nothing");

    // Control: a drag from the handle belongs to the control.
    drag(&mut frames, Pos2::new(160.0, 160.0), Vec2::new(-20.0, -10.0), &mut show_frame);
    assert_eq!(drags(&log.borrow().root_phases(0, Some(BACKGROUND))), 1, "the handle press did not reach the tool");
    assert_eq!(drags(&log.borrow().member_phases(0, "crop", Some(CORNER))), 1, "the handle took its drag");
}

/// 8.2: a member deleted mid-drag, with its root still shown, ends the press instead of leaving the pointer captured.
#[trial]
fn a_member_removed_mid_drag_ends_the_press() {
    let node = rect(140.0, 140.0, 140.0, 50.0);
    let with_member = ProbeRoot::new("graph", rect(100.0, 100.0, 600.0, 400.0), Sense::empty()).arena().member(ProbeMember::new(
        "node",
        node,
        Sense::CLICK | Sense::DRAG,
    ));
    let without = ProbeRoot::new("graph", rect(100.0, 100.0, 600.0, 400.0), Sense::empty()).arena();
    let (mut show_with, _) = scene(vec![with_member]);
    let (mut show_without, _) = scene(vec![without]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_with);
    let from = Pos2::new(210.0, 165.0);
    frames.frame(vec![pointer::moved(from)], &mut show_with);
    frames.frame(vec![pointer::press(from)], &mut show_with);
    frames.frame(vec![pointer::moved(from + Vec2::new(30.0, 0.0))], &mut show_with);
    frames.frame(vec![pointer::moved(from + Vec2::new(60.0, 0.0))], &mut show_with);
    assert!(craft_ui::testing::pointer::press_captured(&frames.ctx), "control: the drag holds the pointer");
    frames.idle(1, &mut show_without);
    assert!(!craft_ui::testing::pointer::press_captured(&frames.ctx), "the removed member's press ended");
}

/// An egui layer over the pointer takes the hover: the arena's part under it reads idle.
#[trial]
fn a_layer_over_the_pointer_takes_the_hover_from_the_arena() {
    let area = rect(100.0, 100.0, 400.0, 300.0);
    let canvas = ProbeRoot::new("canvas", area, Sense::empty()).arena().part(ProbePart::new(BACKGROUND, area, Sense::DRAG));
    let (mut show_frame, log) = scene(vec![canvas]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    frames.frame(vec![pointer::moved(Pos2::new(200.0, 200.0))], &mut show_frame);
    frames.idle(1, &mut show_frame);
    assert!(log.borrow().last().root.parts[0].hovered, "control: the pointer hovers the part");
    let mut with_cover = |ui: &mut Ui| {
        show_frame(ui);
        cover(ui.ctx(), Vec2::splat(600.0));
    };
    frames.idle(3, &mut with_cover);
    assert!(!log.borrow().last().root.parts[0].hovered, "a layer over the pointer takes the hover");
}

/// The focus service classifies a press by the shape that wins it in last pass's shape record.
#[trial]
fn the_shape_record_classifies_presses_by_the_winning_shape() {
    let area = rect(100.0, 100.0, 400.0, 300.0);
    let canvas = ProbeRoot::new("canvas", area, Sense::empty())
        .arena()
        .part(ProbePart::new(BACKGROUND, area, Sense::DRAG | Sense::TAKE_FOCUS).level(HitLevel::Background, 0))
        .member(ProbeMember::new("palette", rect(120.0, 120.0, 60.0, 60.0), Sense::CLICK | Sense::KEEP_FOCUS))
        .member(ProbeMember::new("button", rect(200.0, 120.0, 60.0, 60.0), Sense::CLICK));
    let (mut show_frame, _log) = scene(vec![canvas]);
    let mut frames = Frames::new();
    settle(&mut frames, &mut show_frame);
    let mut at = |x: f32, y: f32| {
        frames.frame(vec![pointer::moved(Pos2::new(x, y))], &mut show_frame);
        press_focus(&frames.ctx, Pos2::new(x, y))
    };
    assert_eq!(at(300.0, 300.0), Some(PressFocus::Take), "the tool's background takes focus");
    assert_eq!(at(150.0, 150.0), Some(PressFocus::Keep), "a keep-focus member keeps it");
    assert_eq!(at(230.0, 150.0), Some(PressFocus::Release), "another control acts without moving it");
    assert_eq!(at(50.0, 50.0), None, "no shape takes a press outside the canvas");
}

/// A middle press over a part that senses a drag captures the pointer for a pan: it drags, and its release is no click.
/// A part that senses only a click takes no middle press.
#[trial]
fn a_middle_press_pans_a_drag_part_and_never_clicks() {
    let area = rect(100.0, 100.0, 400.0, 300.0);
    let from = Pos2::new(200.0, 200.0);
    let to = from + Vec2::new(30.0, 0.0);
    let run = |sense: Sense| {
        let canvas = ProbeRoot::new("canvas", area, Sense::empty()).arena().part(ProbePart::new(BACKGROUND, area, sense));
        let (mut show_frame, log) = scene(vec![canvas]);
        let mut frames = Frames::new();
        settle(&mut frames, &mut show_frame);
        log.borrow_mut().clear();
        frames.frame(vec![pointer::moved(from)], &mut show_frame);
        frames.frame(vec![pointer::middle(from, true)], &mut show_frame);
        frames.frame(vec![pointer::moved(to)], &mut show_frame);
        frames.frame(vec![pointer::moved(to + Vec2::new(5.0, 0.0))], &mut show_frame);
        frames.frame(vec![pointer::middle(to, false)], &mut show_frame);
        frames.idle(1, &mut show_frame);
        log.borrow().root_phases(0, Some(BACKGROUND))
    };
    let phases = run(Sense::CLICK | Sense::DRAG);
    let by_middle = |start: &craft_behavior::PressStart| start.button == craft_behavior::PointerButton::Middle;
    assert!(phases.iter().any(|phase| matches!(phase, PressPhase::DragStart { start, .. } if by_middle(start))), "the pan starts: {phases:?}");
    assert!(phases.iter().any(|phase| matches!(phase, PressPhase::Drag { start, .. } if by_middle(start))), "the pan drags: {phases:?}");
    assert!(phases.iter().any(|phase| matches!(phase, PressPhase::Release { click: false, .. })), "the release is no click: {phases:?}");
    assert_eq!(run(Sense::CLICK), Vec::new(), "a click-only part takes no middle press");
}
