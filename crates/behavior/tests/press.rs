//! The press machine over passes whose input carries a whole quick gesture.

use craft_behavior::Sense;
use craft_behavior::press::{PassInput, PointerEvent, PressMachine, PressPhase};
use emath::{Pos2, pos2};

const SLOP: f32 = 6.0;

fn pass(machine: &mut PressMachine<u8>, events: &[PointerEvent], held: bool) -> Option<PressPhase> {
    machine.step(PassInput::new(events, SLOP, pos2(0.0, 0.0), held), |_| Some((1, Sense::CLICK | Sense::DRAG))).phase
}

fn origin(phase: Option<PressPhase>) -> Option<Pos2> {
    phase.and_then(|phase| phase.start()).map(|start| start.origin)
}

#[test]
fn a_press_and_moves_past_slop_in_one_pass_start_the_drag_at_the_press() {
    let mut machine = PressMachine::new();
    let events = [PointerEvent::pressed(pos2(10.0, 10.0), 1.0, 1), PointerEvent::Moved(pos2(20.0, 12.0)), PointerEvent::Moved(pos2(50.0, 20.0))];
    let phase = pass(&mut machine, &events, true);
    assert!(matches!(phase, Some(PressPhase::DragStart { pos, .. }) if pos == pos2(50.0, 20.0)), "{phase:?}");
    assert_eq!(origin(phase), Some(pos2(10.0, 10.0)));
}

#[test]
fn moves_and_a_release_in_the_pass_after_the_press_read_drag_start_then_release() {
    let mut machine = PressMachine::new();
    assert!(matches!(pass(&mut machine, &[PointerEvent::pressed(pos2(10.0, 10.0), 1.0, 1)], true), Some(PressPhase::Down { .. })));
    let events = [PointerEvent::Moved(pos2(30.0, 15.0)), PointerEvent::Moved(pos2(50.0, 20.0)), PointerEvent::released(pos2(50.0, 20.0))];
    let phase = pass(&mut machine, &events, false);
    assert!(matches!(phase, Some(PressPhase::DragStart { pos, .. }) if pos == pos2(50.0, 20.0)), "{phase:?}");
    assert!(machine.dragging(), "the drag stays live until its release is reported");
    let phase = pass(&mut machine, &[], false);
    assert!(matches!(phase, Some(PressPhase::Release { pos, click: false, .. }) if pos == pos2(50.0, 20.0)), "{phase:?}");
    assert_eq!(origin(phase), Some(pos2(10.0, 10.0)));
    assert_eq!(pass(&mut machine, &[], false), None, "the press is over");
}
