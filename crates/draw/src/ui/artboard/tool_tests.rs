//! Tests of the Artboard tool (task SH-T5). PhotoCraft's `artboard_tool.rs` tests move here. The
//! "+" button press needs `AddBesideHandles::at` (SH-T8): these tests set
//! `ArtboardRules::plus_buttons` off; the "+" test is in `add_beside_tests.rs`.

use egui::{CursorIcon, Event, Key, Modifiers, Sense, vec2};
use egui_kittest::Harness;
use kurbo::{Point, Rect, Vec2};

use super::{ArtboardBackground, ArtboardEvent, ArtboardItem, ArtboardRules, ArtboardTool};
use crate::ui::add_beside::AddSide;
use craft_ui::canvas::{CanvasId, CanvasView, ToolHost, ToolInput};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

/// The app: the boards, the selection and what the tool reported, with every commit applied as
/// the app would.
struct Host {
    tool: ArtboardTool,
    items: Vec<ArtboardItem>,
    selected: Vec<usize>,
    rules: ArtboardRules,
    view: CanvasView,
    canvas: Option<CanvasId>,
    focus: bool,
    cursor: CursorIcon,
    events: Vec<ArtboardEvent>,
}

fn photoshop() -> ArtboardRules {
    ArtboardRules { plus_buttons: false, ..ArtboardRules::PHOTOSHOP }
}

fn board(name: &str, rect: (f64, f64, f64, f64), locked: bool) -> ArtboardItem {
    ArtboardItem { name: name.into(), rect: Rect::new(rect.0, rect.1, rect.2, rect.3), locked, background: ArtboardBackground::White }
}

fn apply(host: &mut Host, event: &ArtboardEvent) {
    match event {
        ArtboardEvent::Draw(EditEvent::Commit { value, .. }) => {
            host.items.push(board("new", (value.x0, value.y0, value.x1, value.y1), false));
            host.selected = vec![host.items.len() - 1];
        }
        ArtboardEvent::Move { indices, event: EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }, .. } => {
            for &i in indices {
                host.items[i].rect = host.items[i].rect + *value;
            }
        }
        ArtboardEvent::Resize { index, event: EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } } => host.items[*index].rect = *value,
        ArtboardEvent::Select { indices, .. } => host.selected.clone_from(indices),
        _ => {}
    }
}

fn harness(items: Vec<ArtboardItem>, selected: &[usize], rules: ArtboardRules) -> Harness<'static, Host> {
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0));
    let host = Host {
        tool: ArtboardTool::new(),
        items,
        selected: selected.to_vec(),
        rules,
        view: CanvasView::new(rect, Point::new(200.0, 150.0), 1.0),
        canvas: None,
        focus: false,
        cursor: CursorIcon::Default,
        events: Vec::new(),
    };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "artboard");
                host.canvas = Some(id);
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                if std::mem::take(&mut host.focus) {
                    canvas.request_focus();
                }
                host.tool.items(&host.items).selected(&host.selected).rules(host.rules);
                let r = ToolHost::new(id).show(ui, &host.view, &canvas, &mut host.tool);
                host.cursor = r.cursor;
                for event in host.tool.take_events() {
                    apply(host, &event);
                    host.events.push(event);
                }
            },
            host,
        );
    h.step();
    h
}

fn input(h: &mut Harness<'_, Host>, input: impl FnOnce(Modifiers) -> ToolInput, m: Modifiers) {
    let Some(id) = h.state().canvas else { panic!("a pass ran") };
    ToolHost::inject(&h.ctx, id, input(m));
    h.step();
}

fn down(h: &mut Harness<'_, Host>, at: (f64, f64), m: Modifiers) {
    input(h, |modifiers| ToolInput::Down { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers }, m);
}

fn to(h: &mut Harness<'_, Host>, at: (f64, f64), m: Modifiers) {
    input(h, |modifiers| ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers }, m);
}

fn up(h: &mut Harness<'_, Host>, at: (f64, f64), m: Modifiers) {
    input(h, |modifiers| ToolInput::Up { doc: Point::new(at.0, at.1), modifiers }, m);
}

/// A press, a move through the middle, a move to `b` and the release.
fn drag(h: &mut Harness<'_, Host>, a: (f64, f64), b: (f64, f64), m: Modifiers) {
    down(h, a, m);
    to(h, ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), m);
    to(h, b, m);
    up(h, b, m);
}

fn none() -> Modifiers {
    Modifiers::NONE
}

/// The phases of the gesture events, per kind, in order.
fn phases(events: &[ArtboardEvent]) -> Vec<&'static str> {
    fn phase<T>(e: &EditEvent<T>) -> &'static str {
        match e {
            EditEvent::Begin { .. } => "begin",
            EditEvent::Update { .. } => "update",
            EditEvent::Commit { .. } => "commit",
            EditEvent::Cancel { .. } => "cancel",
            EditEvent::Set { .. } => "set",
            _ => "other",
        }
    }
    events
        .iter()
        .filter_map(|e| match e {
            ArtboardEvent::Draw(event) => Some(phase(event)),
            ArtboardEvent::Move { event, .. } => Some(phase(event)),
            ArtboardEvent::Resize { event, .. } => Some(phase(event)),
            _ => None,
        })
        .collect()
}

fn committed_rect(events: &[ArtboardEvent]) -> Option<Rect> {
    events.iter().find_map(|e| match e {
        ArtboardEvent::Draw(EditEvent::Commit { value, .. }) | ArtboardEvent::Resize { event: EditEvent::Commit { value, .. }, .. } => Some(*value),
        _ => None,
    })
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect::new(x0, y0, x1, y1)
}

fn key(h: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn focused(h: &mut Harness<'_, Host>) {
    h.state_mut().focus = true;
    h.step();
}

// Drawing, resizing and moving a board each report one gesture (Begin … Commit) with whole-pixel
// values (PhotoCraft `artboard_tool_draws_resizes_and_moves_boards`).
#[test]
fn artboard_draw_resize_move_one_event_each() {
    let mut h = harness(Vec::new(), &[], photoshop());
    drag(&mut h, (20.0, 30.0), (120.0, 110.0), none());
    let events = std::mem::take(&mut h.state_mut().events);
    assert_eq!(phases(&events).first(), Some(&"begin"));
    assert_eq!(phases(&events).last(), Some(&"commit"));
    assert_eq!(phases(&events).iter().filter(|p| **p == "commit").count(), 1);
    assert_eq!(committed_rect(&events), Some(rect(20.0, 30.0, 120.0, 110.0)));
    assert_eq!(h.state().selected, [0], "the new board is the app's selection");

    // The right edge's handle, dragged to a fractional point: whole pixels.
    drag(&mut h, (120.0, 70.0), (160.4, 75.6), none());
    let events = std::mem::take(&mut h.state_mut().events);
    assert!(matches!(events.first(), Some(ArtboardEvent::Resize { index: 0, event: EditEvent::Begin { .. } })), "{events:?}");
    assert_eq!(phases(&events).iter().filter(|p| **p == "commit").count(), 1);
    assert_eq!(committed_rect(&events), Some(rect(20.0, 30.0, 160.0, 110.0)));

    // From inside, the board moves with its contents: one offset.
    drag(&mut h, (60.0, 60.0), (70.0, 50.0), none());
    let events = std::mem::take(&mut h.state_mut().events);
    let commits: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ArtboardEvent::Move { indices, event: EditEvent::Commit { value, changed, .. }, copy } => {
                Some((indices.clone(), *value, *changed, *copy))
            }
            _ => None,
        })
        .collect();
    assert_eq!(commits, [(vec![0], Vec2::new(10.0, -10.0), true, false)], "{events:?}");
    assert_eq!(h.state().items[0].rect, rect(30.0, 20.0, 170.0, 100.0));
}

// A press and release 2 screen points apart on the pasteboard reports nothing.
#[test]
fn artboard_click_without_drag_draws_nothing() {
    let mut h = harness(vec![board("a", (20.0, 30.0, 120.0, 110.0), false)], &[0], photoshop());
    drag(&mut h, (300.0, 250.0), (302.0, 250.0), none());
    assert!(h.state().events.is_empty(), "{:?}", h.state().events);
    // A drag past the slop that is a line, not a board, ends without one.
    drag(&mut h, (300.0, 250.0), (340.0, 250.0), none());
    assert_eq!(h.state().items.len(), 1);
    assert_eq!(phases(&h.state().events).iter().filter(|p| **p == "commit").count(), 0, "{:?}", h.state().events);
}

// The right handle's cursor is the horizontal resize arrow at 0° and the vertical one with the view
// turned 90° (PhotoCraft `resize_cursors_turn_with_the_view`).
#[test]
fn artboard_resize_cursors_turn_with_the_view() {
    use CursorIcon::{ResizeHorizontal as H, ResizeNeSw as NeSw, ResizeNwSe as NwSe, ResizeVertical as V};
    let mut h = harness(vec![board("a", (150.0, 100.0, 250.0, 200.0), false)], &[0], photoshop());
    // The right edge, the top edge, the top-left and the top-right corner.
    let handles = [(250.0, 150.0), (200.0, 100.0), (150.0, 100.0), (250.0, 100.0)];
    for (rotation, want) in [(0.0, [H, V, NwSe, NeSw]), (90.0, [V, H, NeSw, NwSe])] {
        h.state_mut().view.rotation = rotation;
        let mut got = Vec::new();
        for at in handles {
            let screen = h.state().view.to_screen(Point::new(at.0, at.1));
            h.event(Event::PointerMoved(screen));
            h.step();
            h.step();
            got.push(h.state().cursor);
        }
        assert_eq!(got, want, "view turned {rotation}°");
    }
    // Inside the board the move arrow, away from it the arrow.
    h.state_mut().view.rotation = 0.0;
    for (at, want) in [((200.0, 150.0), CursorIcon::Move), ((20.0, 20.0), CursorIcon::Default)] {
        let screen = h.state().view.to_screen(Point::new(at.0, at.1));
        h.event(Event::PointerMoved(screen));
        h.step();
        h.step();
        assert_eq!(h.state().cursor, want, "{at:?}");
    }
}

// ⇧ pressed during a corner drag keeps the board's proportions from that pass on (PhotoCraft
// `plus_buttons_add_boards_and_shift_keeps_proportions`).
#[test]
fn artboard_shift_corner_keeps_proportions_mid_drag() {
    let mut h = harness(vec![board("a", (20.0, 30.0, 120.0, 80.0), false)], &[0], photoshop());
    let latest = |h: &Harness<'_, Host>| {
        h.state().events.iter().rev().find_map(|e| match e {
            ArtboardEvent::Resize { event: EditEvent::Update { value, .. } | EditEvent::Commit { value, .. }, .. } => Some(*value),
            _ => None,
        })
    };
    // The 100 × 50 board's bottom-right corner, dragged 40 right and 5 down.
    down(&mut h, (120.0, 80.0), none());
    to(&mut h, (160.0, 85.0), none());
    assert_eq!(latest(&h), Some(rect(20.0, 30.0, 160.0, 85.0)));
    // ⇧ goes down: the same pointer now gives 140 × 70.
    to(&mut h, (160.0, 85.0), Modifiers::SHIFT);
    assert_eq!(latest(&h), Some(rect(20.0, 30.0, 160.0, 100.0)));
    up(&mut h, (160.0, 85.0), Modifiers::SHIFT);
    assert_eq!(committed_rect(&h.state().events), Some(rect(20.0, 30.0, 160.0, 100.0)));
    assert_eq!(h.state().items[0].rect, rect(20.0, 30.0, 160.0, 100.0));
}

// Illustrator rules: ⇧-click toggles a board, ⇧-drag on the pasteboard selects the boards it
// touches, arrows move by `arrow_step` and ⇧ ten times, Esc reports `Leave`.
#[test]
fn artboard_ai_rules_multi_select_marquee_and_arrow_step() {
    let items = vec![
        board("a", (0.0, 0.0, 100.0, 100.0), false),
        board("b", (200.0, 0.0, 300.0, 100.0), false),
        board("c", (400.0, 0.0, 500.0, 100.0), false),
    ];
    let mut h = harness(items, &[0], ArtboardRules::ILLUSTRATOR.arrow_step(2.0));
    let selects = |h: &mut Harness<'_, Host>| -> Vec<Vec<usize>> {
        std::mem::take(&mut h.state_mut().events)
            .into_iter()
            .filter_map(|e| match e {
                ArtboardEvent::Select { indices, .. } => Some(indices),
                _ => None,
            })
            .collect()
    };
    // ⇧-click adds board b as the active one, and again takes it out.
    drag(&mut h, (250.0, 50.0), (250.0, 50.0), Modifiers::SHIFT);
    assert_eq!(selects(&mut h), [vec![1, 0]]);
    drag(&mut h, (250.0, 50.0), (250.0, 50.0), Modifiers::SHIFT);
    assert_eq!(selects(&mut h), [vec![0]]);
    // ⇧-drag on the pasteboard selects what it touches.
    drag(&mut h, (150.0, 150.0), (250.0, 50.0), Modifiers::SHIFT);
    assert_eq!(selects(&mut h), [vec![1]]);
    // ⇧-drag inside a board draws a new one.
    drag(&mut h, (50.0, 50.0), (150.0, 150.0), Modifiers::SHIFT);
    assert_eq!(h.state().items.len(), 4);
    assert_eq!(h.state().items[3].rect, rect(50.0, 50.0, 150.0, 150.0));
    h.state_mut().events.clear();
    // A click on the pasteboard asks the app to fit a board to the object there.
    drag(&mut h, (350.0, 200.0), (350.0, 200.0), none());
    assert_eq!(h.state().events, [ArtboardEvent::FitObject { at: Point::new(350.0, 200.0) }]);
    // ⌥-drag inside a board moves a copy.
    h.state_mut().selected = vec![1];
    h.state_mut().events.clear();
    drag(&mut h, (250.0, 50.0), (260.0, 50.0), Modifiers::ALT);
    assert!(
        h.state().events.iter().any(|e| matches!(e, ArtboardEvent::Move { copy: true, event: EditEvent::Commit { .. }, .. })),
        "{:?}",
        h.state().events
    );

    h.state_mut().events.clear();
    h.state_mut().selected = vec![2];
    focused(&mut h);
    key(&mut h, none(), Key::ArrowRight);
    key(&mut h, Modifiers::SHIFT, Key::ArrowDown);
    key(&mut h, none(), Key::Escape);
    let moves: Vec<Vec2> = h
        .state()
        .events
        .iter()
        .filter_map(|e| match e {
            ArtboardEvent::Move { indices, event: EditEvent::Set { value, .. }, .. } if indices == &[2] => Some(*value),
            _ => None,
        })
        .collect();
    assert_eq!(moves, [Vec2::new(2.0, 0.0), Vec2::new(0.0, 20.0)]);
    assert_eq!(h.state().events.last(), Some(&ArtboardEvent::Leave));
}

// A locked board selects on a press but reports no `Move` or `Resize`, by pointer or arrows.
#[test]
fn artboard_locked_board_selects_but_never_moves() {
    let mut h = harness(vec![board("a", (0.0, 0.0, 100.0, 100.0), true)], &[], ArtboardRules::ILLUSTRATOR);
    drag(&mut h, (50.0, 50.0), (80.0, 80.0), none());
    assert_eq!(h.state().events.len(), 1, "{:?}", h.state().events);
    assert!(matches!(&h.state().events[0], ArtboardEvent::Select { indices, .. } if indices == &[0]));
    h.state_mut().events.clear();
    // Selected, its edge is no handle and a drag moves nothing.
    drag(&mut h, (98.0, 50.0), (150.0, 90.0), none());
    focused(&mut h);
    key(&mut h, none(), Key::ArrowRight);
    key(&mut h, Modifiers::ALT, Key::ArrowRight);
    assert!(h.state().events.is_empty(), "{:?}", h.state().events);
    assert_eq!(h.state().items[0].rect, rect(0.0, 0.0, 100.0, 100.0));
}

// Page Down picks the next board (`Select`), arrows move it, Delete reports `Delete`, Enter
// `OpenOptions`, Insert `AddBeside` on the right.
#[test]
fn artboard_keyboard_picks_moves_and_deletes() {
    let mut h = harness(vec![board("a", (0.0, 0.0, 100.0, 100.0), false), board("b", (200.0, 0.0, 300.0, 100.0), false)], &[0], photoshop());
    // The canvas is the first Tab stop, the boards the second.
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().selected, [1]);
    let m = none();
    key(&mut h, m, Key::ArrowRight);
    key(&mut h, Modifiers::ALT, Key::ArrowDown);
    key(&mut h, m, Key::Delete);
    key(&mut h, m, Key::Enter);
    key(&mut h, m, Key::Insert);
    let events: Vec<_> = h.state().events.iter().skip(1).cloned().collect();
    let by = |e: &ArtboardEvent| match e {
        ArtboardEvent::Move { event: EditEvent::Set { value, .. }, .. } => Some(*value),
        _ => None,
    };
    assert_eq!(events.first().and_then(by), Some(Vec2::new(1.0, 0.0)), "{events:?}");
    assert!(
        matches!(&events[1], ArtboardEvent::Resize { index: 1, event: EditEvent::Set { value, .. } } if *value == rect(201.0, 0.0, 301.0, 101.0)),
        "{events:?}"
    );
    assert_eq!(events[2], ArtboardEvent::Delete { indices: vec![1] });
    assert_eq!(events[3], ArtboardEvent::OpenOptions { index: 1 });
    assert_eq!(events[4], ArtboardEvent::AddBeside { index: 1, side: AddSide::Right, copy: false });
}

// Esc and `ToolHost::cancel` mid-drag report `Cancel` for `Draw`, `Move` and `Resize`.
#[test]
fn artboard_cancels_on_escape_and_on_tool_switch() {
    let mut h = harness(vec![board("a", (20.0, 30.0, 120.0, 110.0), false)], &[0], photoshop());
    let cancels = |h: &mut Harness<'_, Host>| -> Vec<&'static str> {
        std::mem::take(&mut h.state_mut().events)
            .iter()
            .filter_map(|e| match e {
                ArtboardEvent::Draw(EditEvent::Cancel { .. }) => Some("draw"),
                ArtboardEvent::Move { event: EditEvent::Cancel { .. }, .. } => Some("move"),
                ArtboardEvent::Resize { event: EditEvent::Cancel { .. }, .. } => Some("resize"),
                _ => None,
            })
            .collect()
    };
    for (from, to_, want) in
        [((250.0, 150.0), (300.0, 200.0), "draw"), ((60.0, 60.0), (80.0, 70.0), "move"), ((120.0, 70.0), (150.0, 70.0), "resize")]
    {
        down(&mut h, from, none());
        self::to(&mut h, to_, none());
        h.key_press(Key::Escape);
        h.run();
        assert_eq!(cancels(&mut h), [want]);
        up(&mut h, to_, none());
        assert!(h.state().events.is_empty(), "{want}: the release after Esc reports nothing");
    }
    // The app switches tools mid-drag.
    down(&mut h, (60.0, 60.0), none());
    to(&mut h, (80.0, 70.0), none());
    let (ctx, id) = (h.ctx.clone(), h.state().canvas.expect("a pass ran"));
    ToolHost::cancel(&ctx, id, &mut h.state_mut().tool);
    h.step();
    assert_eq!(cancels(&mut h), ["move"]);
    assert_eq!(h.state().items[0].rect, rect(20.0, 30.0, 120.0, 110.0));
}
