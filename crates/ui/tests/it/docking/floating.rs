//! `FloatingGroup`: the caller owns the rect, the title moves the window, a double click on it asks to minimize,
//! and the move and the size stay inside the bounds and the panel limits.

use craft_behavior::GesturePhase;
use craft_ui::contract::overlay::PlacementKind;
use craft_ui::docking::{FloatingGroup, FloatingGroupOutput, PanelLimits};
use craft_ui::testing::layout::allocate;
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const SLACK: f32 = 1.5;
const START: Rect = Rect { min: Pos2::new(60.0, 70.0), max: Pos2::new(300.0, 250.0) };
const BOUNDS: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(400.0, 320.0) };
const LEAST: Vec2 = Vec2::new(150.0, 90.0);
const MOVE: Vec2 = Vec2::new(30.0, 20.0);
const FAR: f32 = 600.0;
/// A point on the title row, clear of the action parts at its end.
const TITLE_AT: Vec2 = Vec2::new(24.0, 10.0);

/// The caller of a group: it keeps the rect and the minimized flag the way the dock model does.
struct Caller {
    rect: Cell<Rect>,
    minimized: Cell<bool>,
    shown: Cell<Rect>,
    toggles: Cell<u32>,
    history: RefCell<Vec<FloatingGroupOutput>>,
}

impl Caller {
    fn new() -> Self {
        Caller {
            rect: Cell::new(START),
            minimized: Cell::new(false),
            shown: Cell::new(Rect::NOTHING),
            toggles: Cell::new(0),
            history: RefCell::new(Vec::new()),
        }
    }

    fn show(&self) -> impl FnMut(&mut Ui) + '_ {
        move |ui| {
            let group =
                FloatingGroup::new("Layers", self.rect.get()).bounds(BOUNDS).limits(PanelLimits::at_least(LEAST)).minimized(self.minimized.get());
            let out = group.show_group(ui, |ui, _| {
                allocate(ui, LEAST);
            });
            self.shown.set(out.output.response.rect);
            if out.output.minimized_toggled {
                self.toggles.set(self.toggles.get() + 1);
                self.minimized.set(!self.minimized.get());
            }
            if let Some(change) = out.output.placed {
                self.rect.set(change.rect);
            }
            self.history.borrow_mut().push(out.output);
        }
    }

    fn last_change(&self) -> craft_ui::contract::overlay::PlacementChange {
        self.history.borrow().iter().rev().find_map(|out| out.placed).expect("a change was reported")
    }
}

fn near(a: Rect, b: Rect) -> bool {
    (a.min - b.min).abs().max_elem() < SLACK && (a.size() - b.size()).abs().max_elem() < SLACK
}

fn title_of(rect: Rect) -> Pos2 {
    rect.min + TITLE_AT
}

#[trial]
fn a_move_by_the_title_reports_the_new_rect_and_where_the_pointer_was_released() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    let mut show = caller.show();
    frames.idle(4, &mut show);
    assert!(near(caller.shown.get(), START), "the group shows at the caller's rect: {:?}", caller.shown.get());
    pointer::drag(&mut frames, title_of(START), MOVE, &mut show);
    frames.idle(2, &mut show);
    let change = caller.last_change();
    assert_eq!(change.kind, PlacementKind::Move);
    assert_eq!(change.phase, GesturePhase::Commit);
    assert!((change.rect.min - (START.min + MOVE)).abs().max_elem() < SLACK, "the rect moved with the pointer: {:?}", change.rect);
    let released: Vec<Pos2> = caller.history.borrow().iter().filter_map(|out| out.released_at).collect();
    assert_eq!(released.len(), 1, "the release is reported once: {released:?}");
    assert!((released[0] - (title_of(START) + MOVE)).abs().max_elem() < SLACK, "at the pointer: {released:?}");
}

#[trial]
fn a_double_click_on_the_title_minimizes_the_group_and_keeps_the_height_it_restores_to() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    let mut show = caller.show();
    frames.idle(4, &mut show);
    let at = title_of(START);
    for events in [vec![pointer::moved(at)], vec![pointer::press(at)], vec![pointer::release(at)], vec![pointer::press(at)]] {
        frames.frame(events, &mut show);
    }
    frames.frame(vec![pointer::release(at)], &mut show);
    frames.idle(4, &mut show);
    assert_eq!(caller.toggles.get(), 1, "the double click is reported once");
    assert!(caller.minimized.get());
    assert!(caller.shown.get().height() < START.height() / 2.0, "only the title row shows: {:?}", caller.shown.get());
    let restored_height = caller.rect.get().height();
    pointer::drag(&mut frames, title_of(caller.shown.get()), MOVE, &mut show);
    frames.idle(2, &mut show);
    assert!(
        (caller.rect.get().height() - restored_height).abs() < SLACK,
        "a move while minimized keeps the restored height: {:?}",
        caller.rect.get()
    );
    assert!((caller.rect.get().min - (START.min + MOVE)).abs().max_elem() < SLACK, "and moves the group: {:?}", caller.rect.get());
}

#[trial]
fn a_move_stops_at_the_bounds_and_a_size_drag_at_the_panel_limits() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    let mut show = caller.show();
    frames.idle(4, &mut show);
    pointer::drag(&mut frames, title_of(START), Vec2::new(FAR, FAR), &mut show);
    frames.idle(2, &mut show);
    let moved = caller.rect.get();
    assert!(BOUNDS.contains_rect(moved.shrink(SLACK)), "the move stayed inside the bounds: {moved:?}");
    pointer::drag(&mut frames, moved.max - Vec2::splat(1.0), Vec2::splat(-FAR), &mut show);
    frames.idle(2, &mut show);
    let shrunk = caller.rect.get();
    assert_eq!(caller.last_change().kind, PlacementKind::Size(craft_ui::runtime::windows::ResizeEdge::BottomRight));
    assert!(shrunk.width() >= LEAST.x - SLACK && shrunk.height() > LEAST.y, "the size stopped at the limits plus the title row: {shrunk:?}");
}
