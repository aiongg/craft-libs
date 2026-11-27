//! Count trials for input: hover reaches one row's paint, touch needs no hover, Tab walks the stops
//! in tree order, and a `TreeUpdate` holds only the nodes that changed.

use std::cell::RefCell;
use std::rc::Rc;

use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{
    AccessCx, BuildCx, Constraints, Driver, Element, Event, EventCx, Gesture, Handled, Key, LayoutCx, Mods, NodeId, PaintCx, PointerKind,
    PointerPhase, Pos2, Rgba, Runtime, Sense, Show, State, Vec2, WindowEvent,
};

use super::common::{MAIN, TestPlatform, pointer, tokens};

const ROW: f32 = 20.0;
const ROWS: usize = 6;

#[derive(Default)]
struct Shared {
    rows: Vec<(NodeId, State<bool>)>,
    stops: Vec<NodeId>,
    gestures: Vec<Gesture>,
}

type Log = Rc<RefCell<Shared>>;

struct Panel {
    log: Log,
}

impl Element for Panel {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for i in 0..ROWS {
            cx.keyed(i, |cx| Row { index: i, log: self.log.clone() }.show(cx));
        }
        for _ in 0..2 {
            Stop { log: self.log.clone() }.show(cx);
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(Vec2::new(c.max.x, ROW)));
            cx.place_child(child, Vec2::new(0.0, i as f32 * ROW));
        }
        c.max
    }
}

struct Row {
    index: usize,
    log: Log,
}

impl Element for Row {
    type Output = ();
    type Local = State<bool>;

    fn props_key(&self) -> u64 {
        self.index as u64
    }

    fn init(&self, cx: &mut BuildCx<'_>) -> State<bool> {
        let selected = cx.state(false);
        self.log.borrow_mut().rows.push((cx.node(), selected));
        selected
    }

    fn paint(&self, selected: &State<bool>, cx: &mut PaintCx<'_>) {
        let hovered = cx.interaction().hovered;
        let shade = 30 + 30 * u8::from(hovered) + 60 * u8::from(selected.get(cx));
        let bounds = cx.bounds();
        cx.fill_rect(bounds, Rgba([shade, shade, shade, 255]));
        cx.hit(bounds, Sense::HOVER | Sense::CLICK | Sense::DRAG, 0);
    }

    fn event(&self, selected: &mut State<bool>, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        let Event::Pointer(p) = ev else {
            return Handled::No;
        };
        self.log.borrow_mut().gestures.push(p.gesture);
        if let Gesture::Up { click: true, .. } = p.gesture {
            selected.update(cx, |s| *s = !*s);
        }
        Handled::Yes
    }

    fn access(&self, selected: &State<bool>, cx: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        let mut node = accesskit::Node::new(accesskit::Role::ListItem);
        node.set_label(format!("Row {}", self.index));
        if selected.get(cx) {
            node.set_selected(true);
        }
        Some(node)
    }

    fn tag(&self) -> u64 {
        self.index as u64 + 1
    }
}

struct Stop {
    log: Log,
}

impl Element for Stop {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, cx: &mut BuildCx<'_>) {
        self.log.borrow_mut().stops.push(cx.node());
    }

    fn access(&self, _: &(), _: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        Some(accesskit::Node::new(accesskit::Role::Button))
    }

    fn focusable(&self) -> bool {
        true
    }
}

fn setup() -> (Driver<TestPlatform, RecordingRenderer>, Log) {
    let size = Vec2::new(300.0, 200.0);
    let log: Log = Rc::default();
    let mut rt = Runtime::new(tokens());
    rt.set_root(MAIN, Panel { log: log.clone() });
    let mut driver = Driver::new(rt, TestPlatform::new(size, 1.0), RecordingRenderer::default());
    assert!(driver.step(0.0).ran);
    (driver, log)
}

fn key(key: Key, mods: Mods) -> WindowEvent {
    WindowEvent::Key { key, mods, down: true, repeat: false }
}

fn row_at(i: usize) -> Pos2 {
    Pos2::new(100.0, i as f32 * ROW + ROW / 2.0)
}

#[test]
fn hover_repaints_only_the_rows_it_enters_and_leaves() {
    let (mut d, log) = setup();
    d.platform.access_updates.clear();
    d.platform.push(MAIN, pointer(PointerKind::Mouse, PointerPhase::Move, row_at(3)));
    let enter = d.step(1.0);
    assert_eq!((enter.frame.built, enter.frame.laid_out, enter.frame.painted), (0, 0, 1));
    let row3 = log.borrow().rows[3].0;
    assert!(d.runtime.interaction(row3).hovered);

    d.platform.push(MAIN, pointer(PointerKind::Mouse, PointerPhase::Move, row_at(4)));
    assert_eq!(d.step(2.0).frame.painted, 2, "row 3 leaves, row 4 enters");
    assert!(d.platform.access_updates.is_empty(), "hover changes no access node");
}

#[test]
fn a_touch_tap_selects_with_no_hover_as_a_click_does() {
    let (mut d, log) = setup();
    d.platform.push(MAIN, pointer(PointerKind::Touch, PointerPhase::Down, row_at(2)));
    d.platform.push(MAIN, pointer(PointerKind::Touch, PointerPhase::Up, row_at(2)));
    let tap = d.step(1.0);
    assert_eq!(tap.frame.painted, 1, "the selection, no hover paint");
    let (row2, selected2) = log.borrow().rows[2];
    assert!(*d.runtime.store().peek(selected2));
    assert!(!d.runtime.interaction(row2).hovered);
    assert_eq!(log.borrow().gestures, [Gesture::Down { clicks: 1 }, Gesture::Up { click: true, clicks: 1 }]);

    d.platform.push(MAIN, pointer(PointerKind::Mouse, PointerPhase::Down, row_at(4)));
    d.platform.push(MAIN, pointer(PointerKind::Mouse, PointerPhase::Up, row_at(4)));
    d.step(2.0);
    let selected4 = log.borrow().rows[4].1;
    assert!(*d.runtime.store().peek(selected4), "a click reaches the same state");
}

#[test]
fn a_long_press_then_a_move_starts_a_long_pressed_drag() {
    let (mut d, log) = setup();
    d.platform.push(MAIN, pointer(PointerKind::Touch, PointerPhase::Down, row_at(1)));
    d.step(1.0);
    assert!(d.platform.take_frame_request(), "waits for the long press");
    d.step(1.0 + craft_ui_rt::LONG_PRESS_TIME);
    d.platform.push(MAIN, pointer(PointerKind::Touch, PointerPhase::Move, row_at(3)));
    d.platform.push(MAIN, pointer(PointerKind::Touch, PointerPhase::Up, row_at(3)));
    d.step(2.0);
    let gestures = log.borrow().gestures.clone();
    assert_eq!(gestures[1], Gesture::LongPress);
    assert!(matches!(gestures[2], Gesture::DragStart { long_pressed: true, .. }));
    assert!(matches!(gestures.last(), Some(Gesture::Up { click: false, .. })));
}

#[test]
fn tab_walks_the_stops_in_tree_order() {
    let (mut d, log) = setup();
    let stops = log.borrow().stops.clone();
    let focus_after = |d: &mut Driver<TestPlatform, RecordingRenderer>, ev, t| {
        d.platform.push(MAIN, ev);
        d.step(t);
        d.runtime.focused(MAIN)
    };
    assert_eq!(focus_after(&mut d, key(Key::Tab, Mods::NONE), 1.0), Some(stops[0]));
    assert_eq!(focus_after(&mut d, key(Key::Tab, Mods::NONE), 2.0), Some(stops[1]));
    assert_eq!(focus_after(&mut d, key(Key::Tab, Mods::NONE), 3.0), Some(stops[0]), "wraps");
    assert_eq!(focus_after(&mut d, key(Key::Tab, Mods::SHIFT), 4.0), Some(stops[1]));
    let last = d.platform.updates_for(MAIN).last().expect("focus moves are sent");
    assert_eq!(last.focus, stops[1].access_id());
    assert!(last.nodes.is_empty(), "a focus move changes no node");
}

#[test]
fn a_tree_update_holds_only_the_changed_nodes() {
    let (mut d, log) = setup();
    let first = d.platform.updates_for(MAIN).next().expect("full tree");
    assert!(first.tree.is_some());
    assert_eq!(first.nodes.len(), 2 + ROWS + 2, "the window, the panel, the rows and the stops; layers add no node");

    let (row2, selected2) = log.borrow().rows[2];
    selected2.set(&mut d.runtime, true);
    d.step(1.0);
    let update = d.platform.updates_for(MAIN).last().expect("selection is sent");
    let ids: Vec<_> = update.nodes.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, [row2.access_id()]);
}
