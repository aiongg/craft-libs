//! Tests of the marquees and the selection drag (task SE-F8). Each drives a `MarqueeTool` through
//! a `ToolHost` with injected pointer input (`ToolHost::inject`) and a fake `SelectionProbe` (a
//! rectangle selection, optionally floating), then reads `take_events`.

use craft_ui::canvas::kurbo::{Point, Rect, Size};
use craft_ui::canvas::{CanvasId, CanvasView, Constraint, SelectionOperation, ToolHost, ToolInput, ToolKey};
use craft_ui::edit::CancelReason;
use craft_ui::edit::EditEvent;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
use craft_ui::testing::canvas;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{DragKind, MarqueeKind, MarqueeTool, SelectEvent, SelectPhase, SelectToolPass, SelectionProbe, SelectionShape};

const SIZE: [u32; 2] = [400, 300];

/// A document with an optional rectangle selection and floating piece.
#[derive(Default)]
struct Fake {
    selection: Option<Rect>,
    piece: Option<Rect>,
}

impl SelectionProbe for Fake {
    fn document(&self) -> u64 {
        1
    }

    fn size(&self) -> [u32; 2] {
        SIZE
    }

    fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    fn inside_selection(&self, at: Point) -> bool {
        self.selection.is_some_and(|r| r.contains(at))
    }

    fn floating(&self) -> bool {
        self.piece.is_some()
    }

    fn on_floating(&self, at: Point) -> bool {
        self.piece.is_some_and(|r| r.contains(at))
    }
}

struct Rig {
    tool: MarqueeTool,
    probe: Fake,
    id: Option<CanvasId>,
    focus: bool,
    cancel: bool,
    ignored: bool,
    took_key: Option<ToolKey>,
}

/// Document point and screen point coincide: the view's centre is the canvas's.
fn rig(kind: MarqueeKind, probe: Fake) -> Harness<'static, Rig> {
    let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let state = Rig { tool: MarqueeTool::new(kind), probe, id: None, focus: false, cancel: false, ignored: false, took_key: None };
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, rig: &mut Rig| {
            let canvas_id = CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            if std::mem::take(&mut rig.focus) {
                canvas.request_focus();
            }
            let mut pass = SelectToolPass::new(&mut rig.tool, &rig.probe);
            if std::mem::take(&mut rig.cancel) {
                let _ = ToolHost::cancel(ui.ctx(), canvas_id, &mut pass, CancelReason::Removed);
            }
            let r = ToolHost::new(canvas_id).document(Size::new(400.0, 300.0)).show(ui, &view, &canvas, &mut pass);
            rig.id = Some(r.id);
            rig.ignored |= r.ignored_press.is_some();
            rig.took_key = rig.took_key.or(r.took_key);
        },
        state,
    );
    h.step();
    h
}

fn input(h: &mut Harness<'_, Rig>, input: ToolInput) {
    let id = h.state().id.expect("a pass ran");
    ToolHost::inject(&h.ctx, id, input);
    h.step();
}

fn down(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(
        h,
        ToolInput::Down {
            button: egui::PointerButton::Primary,
            doc: Point::new(at.0, at.1),
            pressure: 1.0,
            tilt: [0.0; 2],
            rotation: 0.0,
            modifiers,
        },
    );
}

fn drag_to(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn up(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: Point::new(at.0, at.1), modifiers });
}

/// A press, a drag and a release; the events.
fn stroke(h: &mut Harness<'_, Rig>, from: (f64, f64), to: (f64, f64), at_press: Modifiers, later: Modifiers) -> Vec<SelectEvent> {
    down(h, from, at_press);
    drag_to(h, to, later);
    up(h, to, later);
    h.state_mut().tool.take_events()
}

fn commit(rect: (f64, f64, f64, f64), operation: SelectionOperation) -> SelectEvent {
    SelectEvent::Commit { shape: SelectionShape::Rectangle(Rect::new(rect.0, rect.1, rect.2, rect.3)), operation }
}

/// "begin", "update 1,0", "commit 2,0 changed", "cancel".
fn tags(events: &[SelectEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            SelectEvent::DragSelection(EditEvent::Begin { .. }) => "begin".to_owned(),
            SelectEvent::DragSelection(EditEvent::Update { value, .. }) => format!("update {},{}", value.delta[0], value.delta[1]),
            SelectEvent::DragSelection(EditEvent::Commit { value, changed, .. }) => {
                format!("commit {},{}{}", value.delta[0], value.delta[1], if *changed { " changed" } else { "" })
            }
            SelectEvent::DragSelection(EditEvent::Cancel { .. }) => "cancel".to_owned(),
            other => format!("{other:?}"),
        })
        .collect()
}

const NONE: Modifiers = Modifiers::NONE;

// Normal style: ⇧ pressed after the press squares, ⌥ pressed after it centres, ⇧ held at the press
// does not (it is Add); Fixed Ratio 2 : 1 and Fixed Size 10 × 10 give PhotoCraft's pixel
// rectangles (`marquee_tests.rs` cases).
#[test]
fn marquee_corners_follow_constraint_and_modifiers() {
    let mut h = rig(MarqueeKind::Rectangle, Fake::default());
    let events = stroke(&mut h, (10.5, 10.5), (50.5, 30.5), NONE, NONE);
    assert_eq!(events, [commit((10.0, 10.0, 51.0, 31.0), SelectionOperation::New)]);

    let events = stroke(&mut h, (10.5, 10.5), (50.5, 30.5), NONE, Modifiers::SHIFT);
    assert_eq!(events, [commit((10.0, 10.0, 51.0, 51.0), SelectionOperation::New)], "⇧ after the press squares");

    let events = stroke(&mut h, (10.5, 10.5), (50.5, 30.5), Modifiers::SHIFT, Modifiers::SHIFT);
    assert_eq!(events, [commit((10.0, 10.0, 51.0, 31.0), SelectionOperation::Add)], "⇧ at the press is Add, not square");

    let events = stroke(&mut h, (100.0, 100.0), (110.0, 120.0), NONE, Modifiers::ALT);
    assert_eq!(events, [commit((90.0, 80.0, 110.0, 120.0), SelectionOperation::New)], "⌥ after the press centres");

    h.state_mut().tool.constraint(Constraint::Ratio(2.0, 1.0));
    let events = stroke(&mut h, (10.0, 10.0), (50.0, 12.0), NONE, NONE);
    assert_eq!(events, [commit((10.0, 10.0, 50.0, 30.0), SelectionOperation::New)]);

    h.state_mut().tool.constraint(Constraint::Size(10.0, 10.0));
    let events = stroke(&mut h, (20.0, 20.0), (40.4, 60.4), NONE, NONE);
    assert_eq!(events, [commit((40.0, 60.0, 50.0, 70.0), SelectionOperation::New)], "the box follows the pointer");
}

// Single Row and Single Column select the one-pixel line under the pointer across the whole
// document, follow the pointer until the release, and stay on the canvas past its edge.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn marquee_single_row_and_column_follow_pointer() {
    let mut h = rig(MarqueeKind::SingleRow, Fake::default());
    down(&mut h, (50.0, 40.7), NONE);
    assert_eq!(h.state().tool.phase(), SelectPhase::Marquee { rect: Some(Rect::new(0.0, 40.0, 400.0, 41.0)) }, "a press selects");
    drag_to(&mut h, (60.0, 90.2), NONE);
    assert_eq!(h.state().tool.phase(), SelectPhase::Marquee { rect: Some(Rect::new(0.0, 90.0, 400.0, 91.0)) });
    drag_to(&mut h, (60.0, 500.0), NONE);
    up(&mut h, (60.0, 500.0), NONE);
    assert_eq!(h.state_mut().tool.take_events(), [commit((0.0, 299.0, 400.0, 300.0), SelectionOperation::New)]);

    let mut h = rig(MarqueeKind::SingleColumn, Fake::default());
    let events = stroke(&mut h, (10.5, 20.0), (450.0, 20.0), NONE, NONE);
    assert_eq!(events, [commit((399.0, 0.0, 400.0, 300.0), SelectionOperation::New)]);

    // A press inside the selection draws the line; it never drags the selection (the other
    // marquees do).
    let selection = Some(Rect::new(100.0, 100.0, 200.0, 200.0));
    for (kind, line) in [(MarqueeKind::SingleRow, (0.0, 150.0, 400.0, 151.0)), (MarqueeKind::SingleColumn, (150.0, 0.0, 151.0, 300.0))] {
        let mut h = rig(kind, Fake { selection, piece: None });
        canvas::press(&mut h, pos2(150.0, 150.0), NONE);
        assert_eq!(h.state().tool.phase(), SelectPhase::Marquee { rect: Some(Rect::new(line.0, line.1, line.2, line.3)) }, "{kind:?}");
        canvas::release(&mut h, pos2(150.0, 150.0), NONE);
        assert_eq!(h.state_mut().tool.take_events(), [commit(line, SelectionOperation::New)], "{kind:?}");
    }
}

// A release within the click slop reports `Deselect`; with Fixed Size the click commits the box at
// the pointer.
#[test]
fn marquee_click_deselects_and_fixed_size_click_places() {
    let mut h = rig(MarqueeKind::Rectangle, Fake::default());
    assert_eq!(stroke(&mut h, (30.0, 30.0), (31.0, 31.0), NONE, NONE), [SelectEvent::Deselect]);
    assert_eq!(h.state().tool.phase(), SelectPhase::Idle);

    h.state_mut().tool.constraint(Constraint::Size(10.0, 10.0));
    down(&mut h, (30.2, 30.2), NONE);
    up(&mut h, (30.2, 30.2), NONE);
    assert_eq!(h.state_mut().tool.take_events(), [commit((30.0, 30.0, 40.0, 40.0), SelectionOperation::New)]);
}

// A plain press inside the selection with the bar on New drags the outline; with ⇧, ⌥ or a bar on
// Add it draws a marquee instead.
#[test]
fn press_inside_selection_drags_outline_only_for_new() {
    let probe = Fake { selection: Some(Rect::new(100.0, 100.0, 200.0, 200.0)), piece: None };
    let mut h = rig(MarqueeKind::Rectangle, probe);
    down(&mut h, (150.0, 150.0), NONE);
    assert_eq!(h.state().tool.phase(), SelectPhase::Dragging(DragKind::Outline));
    drag_to(&mut h, (160.0, 155.0), NONE);
    up(&mut h, (160.0, 155.0), NONE);
    assert_eq!(tags(&h.state_mut().tool.take_events()), ["begin", "update 10,5", "commit 10,5 changed"]);

    let events = stroke(&mut h, (150.0, 150.0), (190.0, 190.0), Modifiers::SHIFT, Modifiers::SHIFT);
    assert_eq!(events, [commit((150.0, 150.0, 190.0, 190.0), SelectionOperation::Add)]);
    let events = stroke(&mut h, (150.0, 150.0), (190.0, 190.0), Modifiers::ALT, Modifiers::ALT);
    assert_eq!(events, [commit((150.0, 150.0, 190.0, 190.0), SelectionOperation::Subtract)]);

    h.state_mut().tool.operation(SelectionOperation::Add);
    let events = stroke(&mut h, (150.0, 150.0), (190.0, 190.0), NONE, NONE);
    assert_eq!(events, [commit((150.0, 150.0, 190.0, 190.0), SelectionOperation::Add)]);
}

// ⌘ inside the selection drags `Pixels { copy: false }`, ⌘⌥ `Pixels { copy: true }`; ⌘ outside it
// is `Ignored` (the host's `ignored_press`).
#[test]
fn command_press_cuts_and_option_copies() {
    let probe = Fake { selection: Some(Rect::new(100.0, 100.0, 200.0, 200.0)), piece: None };
    let mut h = rig(MarqueeKind::Rectangle, probe);
    for (modifiers, copy) in [(Modifiers::COMMAND, false), (Modifiers::COMMAND | Modifiers::ALT, true)] {
        down(&mut h, (150.0, 150.0), modifiers);
        assert_eq!(h.state().tool.phase(), SelectPhase::Dragging(DragKind::Pixels { copy }));
        up(&mut h, (150.0, 150.0), modifiers);
        h.state_mut().tool.take_events();
    }
    assert!(!h.state().ignored);
    down(&mut h, (20.0, 20.0), Modifiers::COMMAND);
    assert!(h.state().ignored, "the Move tool takes a ⌘ press outside the selection");
    assert_eq!(h.state().tool.phase(), SelectPhase::Idle);
}

// With a floating piece a plain press on it drags `Pixels`; one in the margin the cut left draws.
#[test]
fn floating_piece_moves_with_plain_drag() {
    let probe = Fake { selection: Some(Rect::new(100.0, 100.0, 200.0, 200.0)), piece: Some(Rect::new(300.0, 50.0, 360.0, 110.0)) };
    let mut h = rig(MarqueeKind::Rectangle, probe);
    down(&mut h, (330.0, 80.0), NONE);
    assert_eq!(h.state().tool.phase(), SelectPhase::Dragging(DragKind::Pixels { copy: false }));
    up(&mut h, (330.0, 80.0), NONE);
    h.state_mut().tool.take_events();

    let events = stroke(&mut h, (150.0, 150.0), (180.0, 180.0), NONE, NONE);
    assert_eq!(events, [commit((150.0, 150.0, 180.0, 180.0), SelectionOperation::New)], "the margin draws");
}

// A selection drag reports Begin, whole-pixel Updates (only when the rounded delta changes) and
// Commit; Esc reports Cancel and nothing else; a release without travel commits unchanged.
#[test]
fn drag_selection_reports_whole_pixel_delta_and_cancel() {
    let probe = Fake { selection: Some(Rect::new(100.0, 100.0, 200.0, 200.0)), piece: None };
    let mut h = rig(MarqueeKind::Rectangle, probe);
    down(&mut h, (150.0, 150.0), NONE);
    drag_to(&mut h, (150.2, 150.2), NONE);
    drag_to(&mut h, (150.6, 150.0), NONE);
    drag_to(&mut h, (150.7, 150.0), NONE);
    up(&mut h, (152.0, 150.0), NONE);
    assert_eq!(tags(&h.state_mut().tool.take_events()), ["begin", "update 1,0", "commit 2,0 changed"]);

    down(&mut h, (150.0, 150.0), NONE);
    up(&mut h, (150.0, 150.0), NONE);
    assert_eq!(tags(&h.state_mut().tool.take_events()), ["begin", "commit 0,0"]);

    down(&mut h, (150.0, 150.0), NONE);
    drag_to(&mut h, (170.0, 150.0), NONE);
    h.state_mut().cancel = true;
    h.step();
    assert_eq!(tags(&h.state_mut().tool.take_events()), ["begin", "update 20,0", "cancel"]);
    assert_eq!(h.state().tool.phase(), SelectPhase::Idle);
}

// Space held during the drag moves the marquee without resizing it, and the tool takes the key.
#[test]
fn marquee_space_moves_the_marquee() {
    let mut h = rig(MarqueeKind::Rectangle, Fake::default());
    h.state_mut().focus = true;
    h.step();
    down(&mut h, (10.0, 10.0), NONE);
    drag_to(&mut h, (30.0, 30.0), NONE);
    assert_eq!(h.state().tool.phase(), SelectPhase::Marquee { rect: Some(Rect::new(10.0, 10.0, 30.0, 30.0)) });
    h.event(Event::Key { key: Key::Space, physical_key: None, pressed: true, repeat: false, modifiers: NONE });
    h.step();
    assert_eq!(h.state().took_key.map(|k| k.key), Some(Key::Space));
    drag_to(&mut h, (60.0, 50.0), NONE);
    assert_eq!(h.state().tool.phase(), SelectPhase::Marquee { rect: Some(Rect::new(40.0, 30.0, 60.0, 50.0)) }, "same size, moved by the pointer");
    up(&mut h, (60.0, 50.0), NONE);
    assert_eq!(h.state_mut().tool.take_events(), [commit((40.0, 30.0, 60.0, 50.0), SelectionOperation::New)]);
}

// While a marquee is dragged the tool's `node` is "Selection outline" with its pixel size and
// rectangle; with no drag it is `None`.
#[test]
fn marquee_being_dragged_is_one_accesskit_object() {
    let mut h = rig(MarqueeKind::Rectangle, Fake::default());
    h.run();
    assert!(h.query_by_label("Selection outline").is_none());
    down(&mut h, (10.0, 10.0), NONE);
    drag_to(&mut h, (40.0, 30.0), NONE);
    h.run();
    let node = h.get_by_label("Selection outline");
    assert_eq!(node.accesskit_node().role(), Role::GraphicsObject);
    assert_eq!(node.accesskit_node().value().as_deref(), Some("30 px × 20 px"));
    up(&mut h, (40.0, 30.0), NONE);
    h.run();
    assert!(h.query_by_label("Selection outline").is_none());
}
