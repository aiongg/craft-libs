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
fn drag_start_and_release_across_passes() {
    // a press and moves past slop in one pass start the drag at the press
    {
        let mut machine = PressMachine::new();
        let events = [PointerEvent::pressed(pos2(10.0, 10.0), 1.0, 1), PointerEvent::Moved(pos2(20.0, 12.0)), PointerEvent::Moved(pos2(50.0, 20.0))];
        let phase = pass(&mut machine, &events, true);
        assert!(matches!(phase, Some(PressPhase::DragStart { pos, .. }) if pos == pos2(50.0, 20.0)), "{phase:?}");
        assert_eq!(origin(phase), Some(pos2(10.0, 10.0)));
    }
    // moves and a release in the pass after the press read drag start then release
    {
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
}
