//! Caller-owned placement: a layer whose overlay is `placed` shows at the caller's rect and reports the user's moves
//! and resizes as `PlacementChange`s, within the overlay's bounds and size limits; a viewport mount hands the move
//! and resize to the OS window.

use super::kit::{Regions, Slab};
use crate::contract::kit::{PlainStyle, Probe, Seen};
use crate::windows_kit::Windows;
use craft_behavior::GesturePhase;
use craft_ui::contract::overlay::{PlacementChange, PlacementKind};
use craft_ui::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, Mount, Overlay, Props as _, Role, Show as _,
    ShowContainer as _, Sizing,
};
use craft_ui::runtime::commands::{self, CommandSet, FireSource, WindowCommand};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::runtime::windows::ResizeEdge;
use craft_ui::style::StyleCx;
use craft_ui::testing::input::key_press;
use craft_ui::testing::layout::{Block, allocate};
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Event, Key, Modifiers, Pos2, Rect, Response, Ui, Vec2, ViewportCommand, viewport::ResizeDirection};
use std::cell::{Cell, RefCell};

const SLACK: f32 = 1.5;
const MOVE: Vec2 = Vec2::new(40.0, 30.0);
const BODY: Vec2 = Vec2::new(120.0, 60.0);
const HEADER: Vec2 = Vec2::new(100.0, 24.0);
/// Where the caller places the layer, and the screen rect it keeps the layer in.
const START: Rect = Rect { min: Pos2::new(60.0, 70.0), max: Pos2::new(280.0, 210.0) };
const BOUNDS: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(400.0, 300.0) };
const LEAST: Vec2 = Vec2::new(150.0, 100.0);
const MOST: Vec2 = Vec2::new(250.0, 170.0);
/// A drag far past every limit.
const FAR: f32 = 500.0;
/// The size of a child window in the viewport trials.
const CHILD: Vec2 = Vec2::new(400.0, 300.0);

/// A container that reports the placement changes it is given.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Pane<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    log: &'a RefCell<Vec<PlacementChange>>,
}

impl Component for Pane<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PaneStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Pane<'_> {
    fn header(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        allocate(ui, HEADER);
    }

    fn placement_changed(&mut self, change: &PlacementChange) {
        self.log.borrow_mut().push(*change);
    }
}

/// The caller of a `Pane`: it places the layer at the rect of the last change it was told of.
struct Caller {
    placed: Cell<Rect>,
    controlled: Cell<bool>,
    rect: Cell<Rect>,
    history: RefCell<Vec<PlacementChange>>,
    probe: RefCell<Seen>,
    /// Cancel the captured press in the next frame, as Escape does.
    escape: Cell<bool>,
}

impl Caller {
    fn new() -> Self {
        Caller {
            placed: Cell::new(START),
            controlled: Cell::new(true),
            rect: Cell::new(Rect::NOTHING),
            history: RefCell::new(Vec::new()),
            probe: RefCell::new(Seen::default()),
            escape: Cell::new(false),
        }
    }

    fn show(&self, overlay: &Overlay) -> impl FnMut(&mut Ui) + '_ {
        let base = overlay.clone();
        move |ui| {
            if self.escape.replace(false) {
                pointer::cancel_captured_presses(ui.ctx());
            }
            let overlay = if self.controlled.get() { base.clone().placed(self.placed.get()) } else { base.clone() };
            let props = ContainerProps::new()
                .mount(Mount::Layer(overlay))
                .sizing(Sizing::Content)
                .header(HeaderRegion::new().band(DragBand::Move))
                .command_scope(<WindowCommand as CommandSet>::SCOPE);
            let log = RefCell::new(Vec::new());
            let out = Pane { common: Common::new(), props, log: &log }.id_salt("pane").show(ui, |ui, _| {
                Probe::new(&self.probe).size(BODY).show(ui);
            });
            self.rect.set(out.output.rect);
            for change in log.into_inner() {
                self.placed.set(change.rect);
                self.history.borrow_mut().push(change);
            }
        }
    }

    fn phases(&self) -> Vec<GesturePhase> {
        self.history.borrow().iter().map(|change| change.phase).collect()
    }

    fn last(&self) -> PlacementChange {
        *self.history.borrow().last().expect("a change was reported")
    }
}

fn near(a: Rect, b: Rect) -> bool {
    (a.min - b.min).abs().max_elem() < SLACK && (a.size() - b.size()).abs().max_elem() < SLACK
}

fn header_of(rect: Rect) -> Pos2 {
    Pos2::new(rect.center().x, rect.min.y + 8.0)
}

#[trial]
fn a_placed_layer_reports_its_move_and_keeps_no_placement_of_its_own() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    let overlay = Overlay::floating().movable(true);
    let mut show = caller.show(&overlay);
    frames.idle(4, &mut show);
    assert!(near(caller.rect.get(), START), "the layer shows at the placed rect: {:?}", caller.rect.get());
    pointer::drag(&mut frames, header_of(START), MOVE, &mut show);
    frames.idle(2, &mut show);
    let phases = caller.phases();
    assert_eq!(phases.first(), Some(&GesturePhase::Begin), "the drag began: {phases:?}");
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Commit).count(), 1, "one commit: {phases:?}");
    assert_eq!(phases.last(), Some(&GesturePhase::Commit), "the commit ends the gesture: {phases:?}");
    assert!(phases.contains(&GesturePhase::Update), "the drag updated in between: {phases:?}");
    let commit = caller.last();
    assert_eq!(commit.kind, PlacementKind::Move);
    assert!(commit.pointer.is_some(), "a pointer gesture reports the pointer");
    assert!((commit.rect.min - (START.min + MOVE)).abs().max_elem() < SLACK, "the commit carries the moved rect: {:?}", commit.rect);
    assert!(near(caller.rect.get(), commit.rect), "the layer follows the caller's rect: {:?}", caller.rect.get());
    // Without the caller's rect the container has no placement of its own to fall back on.
    caller.controlled.set(false);
    frames.idle(3, &mut show);
    assert!((caller.rect.get().min - commit.rect.min).abs().max_elem() > SLACK, "no placement was stored: {:?}", caller.rect.get());
}

#[trial]
fn a_move_stops_at_the_bounds_and_a_size_drag_at_the_size_limits() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    let overlay = Overlay::floating().movable(true).resizable(true).bounds(BOUNDS).size_limits(LEAST, MOST);
    let mut show = caller.show(&overlay);
    frames.idle(4, &mut show);
    pointer::drag(&mut frames, header_of(START), Vec2::new(FAR, 0.0), &mut show);
    frames.idle(2, &mut show);
    let moved = caller.last().rect;
    assert!(moved.max.x <= BOUNDS.max.x + SLACK && moved.max.x > BOUNDS.max.x - SLACK, "the move stopped at the bounds: {moved:?}");
    assert!((moved.min.y - START.min.y).abs() < SLACK, "the other axis stayed: {moved:?}");
    pointer::drag(&mut frames, moved.min + Vec2::new(0.0, 8.0) + Vec2::new(moved.width() / 2.0, 0.0), Vec2::new(-FAR, 0.0), &mut show);
    frames.idle(2, &mut show);
    let back = caller.last().rect;
    assert!((back.min.x - BOUNDS.min.x).abs() < SLACK, "the move stopped at the other bound: {back:?}");
    let corner = back.max - Vec2::splat(1.0);
    pointer::drag(&mut frames, corner, Vec2::splat(FAR), &mut show);
    frames.idle(2, &mut show);
    let grown = caller.last();
    assert_eq!(grown.kind, PlacementKind::Size(ResizeEdge::BottomRight));
    assert!(grown.rect.width() <= MOST.x + SLACK && grown.rect.height() <= MOST.y + SLACK, "the size stopped at the most: {:?}", grown.rect);
    assert!(grown.rect.max.x <= BOUNDS.max.x + SLACK && grown.rect.max.y <= BOUNDS.max.y + SLACK, "and inside the bounds: {:?}", grown.rect);
    pointer::drag(&mut frames, grown.rect.max - Vec2::splat(1.0), Vec2::splat(-FAR), &mut show);
    frames.idle(2, &mut show);
    let shrunk = caller.last().rect;
    assert!(shrunk.width() >= LEAST.x - SLACK && shrunk.height() >= LEAST.y - SLACK, "the size stopped at the least: {shrunk:?}");
    assert!((shrunk.min - grown.rect.min).abs().max_elem() < SLACK, "the opposite corner stayed: {shrunk:?}");
}

#[trial]
fn escape_during_a_move_drag_reports_one_cancel_with_the_starting_rect() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    let overlay = Overlay::floating().movable(true);
    let mut show = caller.show(&overlay);
    frames.idle(4, &mut show);
    let from = header_of(START);
    frames.frame(vec![pointer::moved(from)], &mut show);
    frames.frame(vec![pointer::press(from)], &mut show);
    frames.frame(vec![pointer::moved(from + MOVE / 2.0)], &mut show);
    frames.frame(vec![pointer::moved(from + MOVE)], &mut show);
    assert!(!near(caller.rect.get(), START), "the layer followed the drag: {:?}", caller.rect.get());
    caller.escape.set(true);
    frames.frame(vec![pointer::moved(from + MOVE * 1.5)], &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![pointer::release(from + MOVE * 1.5)], &mut show);
    frames.idle(2, &mut show);
    let phases = caller.phases();
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Cancel).count(), 1, "one cancel: {phases:?}");
    assert_eq!(phases.last(), Some(&GesturePhase::Cancel), "nothing follows the cancel, not the release: {phases:?}");
    assert!(!phases.contains(&GesturePhase::Commit), "no commit: {phases:?}");
    assert!(near(caller.last().rect, START), "the cancel carries the rect from before the gesture: {:?}", caller.last().rect);
    assert!(near(caller.rect.get(), START), "the layer returned to it: {:?}", caller.rect.get());
}

#[trial]
fn the_keyboard_move_mode_reports_keyboard_changes_and_enter_commits_while_escape_cancels() {
    let (caller, mut frames) = (Caller::new(), Frames::new());
    let overlay = Overlay::floating().movable(true).resizable(true);
    let mut show = caller.show(&overlay);
    frames.idle(4, &mut show);
    request(&frames.ctx, FocusTarget::Id(caller.probe.borrow().id.expect("the probe ran")));
    frames.idle(2, &mut show);
    assert!(commands::fire(&frames.ctx, WindowCommand::Move, FireSource::Palette), "the command has a live instance");
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::ArrowRight, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(caller.phases(), [GesturePhase::Begin, GesturePhase::Update, GesturePhase::Commit]);
    assert!(caller.history.borrow().iter().all(|change| change.kind == PlacementKind::Keyboard && change.pointer.is_none()));
    let kept = caller.last().rect;
    assert!(kept.min.x > START.min.x, "the arrow moved the rect: {kept:?}");
    caller.history.borrow_mut().clear();
    assert!(commands::fire(&frames.ctx, WindowCommand::Move, FireSource::Palette));
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::ArrowDown, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(caller.phases(), [GesturePhase::Begin, GesturePhase::Update, GesturePhase::Cancel]);
    assert!(near(caller.last().rect, kept), "Escape reports the rect from before the mode: {:?}", caller.last().rect);
    assert!(near(caller.rect.get(), kept), "and the layer is back at it: {:?}", caller.rect.get());
}

/// The props of a viewport mount of `overlay` whose header is the move band.
fn viewport_props(overlay: Overlay) -> ContainerProps {
    ContainerProps::new().mount(Mount::viewport(overlay)).sizing(Sizing::Content).header(HeaderRegion::new().band(DragBand::Move))
}

#[trial]
fn a_viewport_mounts_move_band_and_grips_start_the_os_move_and_resize() {
    let windows = Windows::new();
    let regions = RefCell::new(Regions::default());
    let props = viewport_props(Overlay::floating().movable(true).resizable(true));
    let frame = |child: Vec<Event>| {
        windows.frame(vec![], child, |ui| {
            Slab::new(props.clone()).regions(&regions).label("Tool").id_salt("tool").show(ui, |ui, _| {
                Block::new(BODY).show(ui);
            });
        })
    };
    for _ in 0..4 {
        frame(vec![]);
    }
    let header = regions.borrow().header.expect("the header ran").center();
    let mut sent = Vec::new();
    for events in
        [vec![pointer::moved(header)], vec![pointer::press(header)], vec![pointer::moved(header + MOVE / 2.0)], vec![pointer::moved(header + MOVE)]]
    {
        sent.extend(frame(events).commands);
    }
    frame(vec![pointer::release(header + MOVE)]);
    assert!(sent.iter().any(|(_, command)| *command == ViewportCommand::StartDrag), "the move band started the OS move: {sent:?}");
    let corner = Pos2::new(CHILD.x - 1.0, CHILD.y - 1.0);
    let mut sent = Vec::new();
    for events in
        [vec![pointer::moved(corner)], vec![pointer::press(corner)], vec![pointer::moved(corner - MOVE / 2.0)], vec![pointer::moved(corner - MOVE)]]
    {
        sent.extend(frame(events).commands);
    }
    assert!(
        sent.iter().any(|(_, command)| *command == ViewportCommand::BeginResize(ResizeDirection::SouthEast)),
        "the corner grip started the OS resize: {sent:?}"
    );
}

#[trial]
fn a_viewport_mount_without_viewports_moves_its_fallback_layer_by_its_own_placement() {
    let rect = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let props = viewport_props(Overlay::floating().movable(true));
    let mut show = |ui: &mut Ui| {
        let out = Slab::new(props.clone()).id_salt("tool").show(ui, |ui, _| {
            Block::new(BODY).show(ui);
        });
        rect.set(out.output.rect);
    };
    frames.idle(4, &mut show);
    let before = rect.get();
    pointer::drag(&mut frames, header_of(before), MOVE, &mut show);
    frames.idle(2, &mut show);
    assert!(((rect.get().min - before.min) - MOVE).abs().max_elem() < SLACK, "the fallback layer followed the drag: {:?}", rect.get());
}
