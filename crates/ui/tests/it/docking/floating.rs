//! `FloatingGroup`: the caller owns the rect, the title moves the window, a double click on it asks to minimize,
//! and the move and the size stay inside the bounds and the panel limits.

use craft_behavior::GesturePhase;
use craft_ui::contract::overlay::PlacementKind;
use craft_ui::contract::{Common, Component, Cx, DropMark, DropOperation, DropTarget, MeasureCx, Pointer as _, Props as _, Role, Show as _, Sizing};
use craft_ui::docking::{DragUnit, FloatingGroup, FloatingGroupOutput, GroupDrag, PanelLimits};
use craft_ui::runtime::drag;
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::layout::allocate;
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Response, Ui, Vec2};
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
    sizing: Sizing,
    rect: Cell<Rect>,
    minimized: Cell<bool>,
    shown: Cell<Rect>,
    toggles: Cell<u32>,
    body: Cell<Vec2>,
    anchor: Cell<Option<&'static str>>,
    history: RefCell<Vec<FloatingGroupOutput>>,
}

impl Caller {
    fn new() -> Self {
        Self::sized(Sizing::Fill)
    }

    fn sized(sizing: Sizing) -> Self {
        Caller {
            sizing,
            rect: Cell::new(START),
            minimized: Cell::new(false),
            shown: Cell::new(Rect::NOTHING),
            toggles: Cell::new(0),
            body: Cell::new(LEAST),
            anchor: Cell::new(None),
            history: RefCell::new(Vec::new()),
        }
    }

    fn show(&self) -> impl FnMut(&mut Ui) + '_ {
        move |ui| {
            let mut group = FloatingGroup::new("Layers", self.rect.get())
                .sized(self.sizing)
                .bounds(BOUNDS)
                .limits(PanelLimits::at_least(LEAST))
                .minimized(self.minimized.get());
            if let Some(anchor) = self.anchor.get() {
                group = group.window_of(anchor);
            }
            let out = group.show_group(ui, |ui, _| {
                allocate(ui, self.body.get());
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
fn a_move_by_the_title_reports_the_new_rect() {
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
}

#[trial]
fn a_move_by_the_title_of_a_window_group_runs_a_window_unit_drag_until_the_release() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    caller.anchor.set(Some("Layers"));
    let mut show = caller.show();
    frames.idle(4, &mut show);
    let from = title_of(START);
    for events in [vec![pointer::moved(from)], vec![pointer::press(from)], vec![pointer::moved(from + MOVE / 2.0)], vec![pointer::moved(from + MOVE)]]
    {
        frames.frame(events, &mut show);
    }
    let carried = drag::active::<GroupDrag<&'static str>>(&frames.ctx).expect("the move started a drag session");
    assert_eq!(carried.unit, DragUnit::Window("Layers"));
    frames.frame(vec![pointer::release(from + MOVE)], &mut show);
    frames.idle(2, &mut show);
    assert!(drag::active::<GroupDrag<&'static str>>(&frames.ctx).is_none(), "the release ended it");
}

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
struct PadStyle {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// The rect of the pad, under the window group's start rect and the pointer of the window drag.
const PAD: Rect = Rect { min: Pos2::new(40.0, 60.0), max: Pos2::new(200.0, 200.0) };

struct TakesGroups;

impl DropTarget<GroupDrag<&'static str>> for TakesGroups {
    type Drop = ();

    fn accept(&self, _: &GroupDrag<&'static str>, _: Pos2, _: DropOperation) -> Option<()> {
        Some(())
    }

    fn mark(&self, _: &()) -> DropMark {
        DropMark::Into(PAD)
    }
}

/// A target for dragged groups that lies under the floating group.
#[derive(Component)]
struct Pad {
    #[common]
    common: Common,
}

impl Component for Pad {
    const ROLE: Role = Role::GROUP;
    type Style = PadStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PadStyleKey) -> PadStyle {
        PadStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        PAD.size()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.drop_target(&TakesGroups);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_window_unit_drag_sees_the_target_under_its_own_group() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    caller.anchor.set(Some("Layers"));
    let mut group = caller.show();
    let mut show = |ui: &mut Ui| {
        Pad { common: Common::new() }.id_salt("pad").show_at(ui, PAD);
        group(ui);
    };
    frames.idle(4, &mut show);
    let from = title_of(START);
    for events in [vec![pointer::moved(from)], vec![pointer::press(from)], vec![pointer::moved(from + MOVE / 2.0)], vec![pointer::moved(from + MOVE)]]
    {
        frames.frame(events, &mut show);
    }
    frames.idle(2, &mut show);
    assert!(drag::active::<GroupDrag<&'static str>>(&frames.ctx).is_some(), "the move started a window drag");
    assert!(drag::mark(&frames.ctx).is_some(), "the target under the moving group takes the drop");
}

#[trial]
fn a_content_sized_group_keeps_its_position_and_grows_with_its_body() {
    let (caller, mut frames) = (Caller::sized(Sizing::Content), Frames::new());
    let mut show = caller.show();
    frames.idle(4, &mut show);
    let before = caller.shown.get();
    assert!((before.min - START.min).abs().max_elem() < SLACK, "the rect gives the position: {before:?}");
    caller.body.set(LEAST + Vec2::new(0.0, 60.0));
    frames.idle(4, &mut show);
    let after = caller.shown.get();
    assert!(after.height() > before.height(), "the height follows the body: {before:?} then {after:?}");
    assert!((after.min - before.min).abs().max_elem() < SLACK, "and the position stays: {after:?}");
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
    assert!(caller.shown.get().height() < caller.rect.get().height(), "only the title row shows: {:?}", caller.shown.get());
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
    assert!(shrunk.width() < moved.width() && shrunk.height() < moved.height(), "the size drag shrank the group: {shrunk:?}");
    pointer::drag(&mut frames, shrunk.max - Vec2::splat(1.0), Vec2::splat(-FAR), &mut show);
    frames.idle(2, &mut show);
    assert!(near(caller.rect.get(), shrunk), "the limits stopped the second drag: {:?} then {:?}", shrunk, caller.rect.get());
}
