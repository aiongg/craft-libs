//! Trials for input routing across windows: payload drags between windows, OS file drops,
//! pinches, pen samples, cursors, IME sessions, access actions and windows nodes open.

use std::cell::RefCell;
use std::rc::Rc;

use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{
    BuildCx, Constraints, Cursor, CursorIcon, DragGhost, DragStart, Driver, DropOperation, DropPhase, Element, Event, EventCx, FileDrag, Gesture,
    Handled, ImePurpose, ImeRequest, Key, LayoutCx, Mods, NodeId, Os, OsFile, PaintCx, PointerButton, PointerEvent, PointerKind, PointerPhase,
    PointerSample, Pos2, Rgba, Runtime, Sense, Show, Vec2, WheelOptions, WheelPhase, WheelSource, WheelUnit, WindowEvent, WindowId, WindowSpec,
};

use super::common::{MAIN, TestPlatform, click, pointer, tokens};

type Log = Rc<RefCell<Vec<String>>>;

fn take(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.borrow_mut())
}

/// A box that logs what reaches it, configured per trial.
#[derive(Clone)]
struct Probe {
    name: &'static str,
    size: Vec2,
    sense: Sense,
    shape_cursor: Option<Cursor>,
    node_cursor: Option<Cursor>,
    drop: bool,
    window_drop: bool,
    drags: Option<&'static str>,
    text: bool,
    opens: bool,
    log: Log,
    ids: Rc<RefCell<Vec<(&'static str, NodeId)>>>,
}

impl Probe {
    fn new(name: &'static str, size: Vec2, log: &Log) -> Probe {
        Probe {
            name,
            size,
            sense: Sense::HOVER | Sense::CLICK | Sense::DRAG,
            shape_cursor: None,
            node_cursor: None,
            drop: false,
            window_drop: false,
            drags: None,
            text: false,
            opens: false,
            log: log.clone(),
            ids: Rc::default(),
        }
    }

    fn put(&self, what: impl Into<String>) {
        self.log.borrow_mut().push(format!("{} {}", self.name, what.into()));
    }
}

impl Element for Probe {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        craft_ui_rt::hash_key(&self.name)
    }

    fn init(&self, cx: &mut BuildCx<'_>) {
        self.ids.borrow_mut().push((self.name, cx.node()));
    }

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(self.size)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.fill_rect(bounds, Rgba([60, 60, 60, 255]));
        match self.shape_cursor {
            Some(c) => cx.hit_cursor(bounds, self.sense, 0, c),
            None => cx.hit(bounds, self.sense, 0),
        }
        if let Some(c) = self.node_cursor {
            cx.cursor(c);
        }
        if self.drop {
            cx.drop_target(bounds, 0);
        }
        if self.window_drop {
            cx.window_drop_target();
        }
    }

    fn event(&self, _: &mut (), cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Pointer(p) => match p.gesture {
                Gesture::Down { .. } => self.put("down"),
                Gesture::Up { click: true, .. } => {
                    self.put("click");
                    if self.opens {
                        cx.open_window(WindowSpec::new("opened"), Probe::new("inner", Vec2::new(50.0, 50.0), &self.log));
                    }
                }
                Gesture::DragStart { .. } => {
                    if let Some(payload) = self.drags {
                        let log = self.log.clone();
                        let ghost = DragGhost::new(Vec2::new(20.0, 10.0), Vec2::new(4.0, 4.0), move |cx: &mut PaintCx<'_>| {
                            log.borrow_mut().push("ghost paint".into());
                            let b = cx.bounds();
                            cx.fill_rect(b, Rgba([200, 200, 200, 128]));
                        });
                        let started = cx.start_drag(DragStart::new(payload).ghost(ghost));
                        self.put(format!("start {started}"));
                    }
                }
                Gesture::Drag { .. } => {
                    if let Some(s) = p.sample {
                        self.put(format!("pen {:.1} eraser {}", s.pressure, s.eraser));
                    }
                }
                Gesture::Cancel(reason) => self.put(format!("cancel {reason:?}")),
                _ => {}
            },
            Event::Wheel(w) => {
                if !self.sense.intersects(Sense::WHEELS) {
                    return Handled::No;
                }
                self.put(format!(
                    "wheel {:?} zoom {:.2} turn {:.2} delta {},{} touch {}",
                    w.phase, w.zoom, w.rotation, w.delta.x, w.delta.y, w.touch
                ));
            }
            Event::Drop(d) => {
                if !(self.drop || self.window_drop) {
                    return Handled::No;
                }
                let what = d
                    .data
                    .get::<&'static str>()
                    .map(|s| s.to_string())
                    .or_else(|| d.data.files().map(|f| f.files.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(",")));
                let at = d.pos.map_or("-".to_string(), |p| format!("{},{}", p.x, p.y));
                match d.phase {
                    DropPhase::Over => {
                        self.put(format!("over {} at {at}", what.unwrap_or_default()));
                        return cx.accept_drop(DropOperation::Move);
                    }
                    DropPhase::Leave => self.put("leave"),
                    DropPhase::Drop => self.put(format!("drop {} at {at}", what.unwrap_or_default())),
                }
            }
            Event::DragEnd(o) => self.put(format!("end {o:?}")),
            Event::Access(a) => self.put(format!("access {:?}", a.action)),
            Event::WindowOpened(w) => self.put(format!("opened {}", w.0)),
            _ => return Handled::No,
        }
        Handled::Yes
    }

    fn focusable(&self, _: &Self::Local) -> bool {
        self.text
    }

    fn text_input(&self) -> Option<ImePurpose> {
        self.text.then_some(ImePurpose::Normal)
    }
}

/// Lays its children out left to right at their own sizes.
struct Row(Vec<Probe>);

impl Element for Row {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for p in &self.0 {
            p.clone().show(cx);
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let mut x = 0.0;
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            let s = cx.layout_child(child, c.loosen());
            cx.place_child(child, Vec2::new(x, 0.0));
            x += s.x;
        }
        c.max
    }
}

fn mouse(phase: PointerPhase, x: f32, y: f32) -> WindowEvent {
    pointer(PointerKind::Mouse, phase, Pos2::new(x, y))
}

fn touch(id: u64, phase: PointerPhase, x: f32, y: f32) -> WindowEvent {
    WindowEvent::Pointer(PointerEvent::new(id, PointerKind::Touch, phase, PointerButton::Primary, Pos2::new(x, y), Mods::NONE))
}

fn wheel(x: f32, y: f32, delta: Vec2, unit: WheelUnit, mods: Mods) -> WindowEvent {
    WindowEvent::Wheel { pos: Pos2::new(x, y), delta, unit, phase: WheelPhase::Move, source: WheelSource::Unknown, mods }
}

fn key(key: Key, down: bool) -> WindowEvent {
    WindowEvent::Key { key, mods: Mods::NONE, down, repeat: false }
}

/// MAIN (300 x 200) at the desktop origin and a second window (200 x 100) to its right at
/// x = 400, both at scale 1.
fn two_windows(main: Vec<Probe>, second: Vec<Probe>) -> (Driver<TestPlatform, RecordingRenderer>, WindowId) {
    let mut rt = Runtime::new(tokens());
    rt.set_root(MAIN, Row(main));
    let mut d = Driver::new(rt, TestPlatform::new(Vec2::new(300.0, 200.0), 1.0), RecordingRenderer::default());
    let second_id = d.open_window(&WindowSpec { size: Some(Vec2::new(200.0, 100.0)), ..WindowSpec::new("second") }, Row(second));
    d.platform.push(MAIN, WindowEvent::Moved(Pos2::new(0.0, 0.0)));
    d.platform.push(second_id, WindowEvent::Moved(Pos2::new(400.0, 0.0)));
    d.step(0.0);
    (d, second_id)
}

#[test]
fn a_drag_carries_its_payload_into_another_window_and_drops_there() {
    let log: Log = Rc::default();
    let mut source = Probe::new("source", Vec2::new(100.0, 40.0), &log);
    source.drags = Some("row-7");
    let mut target = Probe::new("target", Vec2::new(200.0, 100.0), &log);
    target.drop = true;
    let ids = target.ids.clone();
    let (mut d, second) = two_windows(vec![source], vec![target]);
    take(&log);

    // Past slop the source starts the drag; the pointer is still over MAIN, where nothing takes it.
    d.platform.push(MAIN, mouse(PointerPhase::Down, 10.0, 10.0));
    d.platform.push(MAIN, mouse(PointerPhase::Move, 30.0, 10.0));
    d.step(1.0);
    assert_eq!(take(&log), ["source down", "source start true", "ghost paint"]);
    assert_eq!(d.runtime.drag().and_then(|i| i.window), Some(MAIN));
    assert_eq!(d.platform.cursors.last(), Some(&(MAIN, Cursor::Icon(CursorIcon::NotAllowed))));

    // The captured pointer leaves MAIN to the right: the second window's target is offered the
    // payload at the pointer in its own coordinates, and the ghost moves into that window.
    d.platform.push(MAIN, mouse(PointerPhase::Move, 450.0, 50.0));
    d.step(2.0);
    assert_eq!(take(&log), ["target over row-7 at 50,50", "ghost paint"]);
    let info = d.runtime.drag().expect("a drag runs");
    assert_eq!(info.window, Some(second));
    assert_eq!(info.op, Some(DropOperation::Move));
    assert_eq!(d.platform.cursors.last(), Some(&(second, Cursor::Icon(CursorIcon::Grabbing))));

    // A move inside the target moves the ghost: nothing builds or repaints.
    d.platform.push(MAIN, mouse(PointerPhase::Move, 460.0, 50.0));
    let moved = d.step(3.0);
    assert_eq!(take(&log), ["target over row-7 at 60,50"]);
    assert_eq!((moved.frame.built, moved.frame.painted), (0, 0), "the ghost moves without a repaint");
    assert_eq!(moved.windows_drawn, 1);

    // The release offers the payload at its own position, then drops on the target; the source
    // hears where, and the drag is over.
    d.platform.push(MAIN, mouse(PointerPhase::Up, 460.0, 50.0));
    d.step(4.0);
    let target_id = ids.borrow()[0].1;
    assert_eq!(
        take(&log),
        [
            "target over row-7 at 60,50".to_string(),
            "target drop row-7 at 60,50".to_string(),
            format!("source end Dropped {{ target: {target_id:?}, window: {second:?}, op: Move }}")
        ]
    );
    assert!(d.runtime.drag().is_none());
}

#[test]
fn escape_aborts_a_drag_and_the_target_and_source_hear_it() {
    let log: Log = Rc::default();
    let mut source = Probe::new("source", Vec2::new(100.0, 40.0), &log);
    source.drags = Some("row-7");
    let mut target = Probe::new("target", Vec2::new(100.0, 40.0), &log);
    target.drop = true;
    let (mut d, _) = two_windows(vec![source, target], vec![]);
    d.platform.push(MAIN, mouse(PointerPhase::Down, 10.0, 10.0));
    d.platform.push(MAIN, mouse(PointerPhase::Move, 150.0, 10.0));
    d.step(1.0);
    take(&log);
    d.platform.push(MAIN, key(Key::Escape, true));
    d.step(2.0);
    assert_eq!(take(&log), ["target leave", "source end Cancelled(Escape)", "source cancel Escape"]);
    assert!(d.runtime.drag().is_none());
    // The release that follows drops nothing.
    d.platform.push(MAIN, mouse(PointerPhase::Up, 150.0, 10.0));
    d.step(3.0);
    assert!(take(&log).is_empty());
}

#[test]
fn an_os_file_drop_reaches_the_target_under_it_or_the_window_target() {
    let log: Log = Rc::default();
    let mut target = Probe::new("target", Vec2::new(100.0, 100.0), &log);
    target.drop = true;
    let mut whole = Probe::new("window", Vec2::new(10.0, 10.0), &log);
    whole.window_drop = true;
    let (mut d, _) = two_windows(vec![target, whole], vec![]);
    let file = |name: &str| OsFile { name: name.into(), ..OsFile::default() };

    // With a position, the shape under it takes the files.
    d.platform.push(MAIN, WindowEvent::Files(FileDrag::Enter { files: vec![file("a.png")], pos: Some(Pos2::new(20.0, 20.0)) }));
    d.step(1.0);
    assert_eq!(take(&log), ["target over a.png at 20,20"]);
    d.platform.push(MAIN, WindowEvent::Files(FileDrag::Drop { files: vec![file("a.png")], pos: Some(Pos2::new(30.0, 20.0)) }));
    d.step(2.0);
    assert_eq!(take(&log), ["target over a.png at 30,20", "target drop a.png at 30,20"]);

    // Away from every shape the window target takes them.
    d.platform.push(MAIN, WindowEvent::Files(FileDrag::Enter { files: vec![file("b.psd")], pos: Some(Pos2::new(250.0, 150.0)) }));
    d.platform.push(MAIN, WindowEvent::Files(FileDrag::Leave));
    d.step(3.0);
    assert_eq!(take(&log), ["window over b.psd at 250,150", "window leave"]);

    // A drop with no position (the desktop OS drag loop) goes to the window target, at no point.
    let mut rt = Runtime::new(tokens());
    let mut whole = Probe::new("window", Vec2::new(10.0, 10.0), &log);
    whole.window_drop = true;
    rt.set_root(MAIN, Row(vec![whole]));
    let mut d = Driver::new(rt, TestPlatform::new(Vec2::new(300.0, 200.0), 1.0), RecordingRenderer::default());
    d.step(0.0);
    d.platform.push(MAIN, WindowEvent::Files(FileDrag::Enter { files: vec![file("c.tif")], pos: None }));
    d.platform.push(MAIN, WindowEvent::Files(FileDrag::Drop { files: vec![file("c.tif")], pos: None }));
    d.step(1.0);
    assert_eq!(take(&log), ["window over c.tif at -", "window drop c.tif at -"]);
}

#[test]
fn two_touch_contacts_pinch_the_node_under_their_centre() {
    let log: Log = Rc::default();
    let mut canvas = Probe::new("canvas", Vec2::new(300.0, 200.0), &log);
    canvas.sense = Sense::CLICK | Sense::DRAG | Sense::WHEEL;
    let (mut d, _) = two_windows(vec![canvas], vec![]);
    take(&log);

    // The second finger cancels the first finger's press and starts the pinch.
    d.platform.push(MAIN, touch(7, PointerPhase::Down, 100.0, 100.0));
    d.platform.push(MAIN, touch(8, PointerPhase::Down, 200.0, 100.0));
    d.step(1.0);
    assert_eq!(take(&log), ["canvas down", "canvas cancel Gesture", "canvas wheel Start zoom 1.00 turn 0.00 delta 0,0 touch true"]);

    // Spreading to twice the distance zooms by 2 and pans by the centre's movement.
    d.platform.push(MAIN, touch(8, PointerPhase::Move, 300.0, 100.0));
    d.step(2.0);
    assert_eq!(take(&log), ["canvas wheel Move zoom 2.00 turn 0.00 delta 50,0 touch true"]);

    // Lifting one finger ends the pinch; the other presses nothing until it lifts too.
    d.platform.push(MAIN, touch(7, PointerPhase::Up, 100.0, 100.0));
    d.platform.push(MAIN, touch(8, PointerPhase::Move, 250.0, 120.0));
    d.platform.push(MAIN, touch(8, PointerPhase::Up, 250.0, 120.0));
    d.step(3.0);
    assert_eq!(take(&log), ["canvas wheel End zoom 1.00 turn 0.00 delta 0,0 touch false"].map(|s| s.replace("touch false", "touch true")));

    // A touchpad pinch the system recognised.
    d.platform.push(MAIN, WindowEvent::Pinch { pos: Pos2::new(50.0, 50.0), zoom: 1.25, rotation: 0.0, mods: Mods::NONE });
    d.step(4.0);
    assert_eq!(take(&log), ["canvas wheel Move zoom 1.25 turn 0.00 delta 0,0 touch false"]);
}

#[test]
fn ctrl_wheel_steps_pinch_as_craft_reads_them() {
    let log: Log = Rc::default();
    let mut canvas = Probe::new("canvas", Vec2::new(300.0, 200.0), &log);
    canvas.sense = Sense::WHEEL;
    let (mut d, _) = two_windows(vec![canvas], vec![]);
    // An app that zooms with Alt (Photoshop's): Ctrl+wheel only zooms as a driver's pinch.
    d.runtime.set_wheel_options(WheelOptions { zoom_mods: Mods::ALT, ..WheelOptions::default() });
    d.runtime.set_os(Os::Windows);
    take(&log);
    let line = |dy: f32| wheel(50.0, 50.0, Vec2::new(0.0, dy), WheelUnit::Line, Mods::CMD);

    // A whole notch with no pinch before it scrolls.
    d.platform.push(MAIN, line(1.0));
    d.step(1.0);
    assert_eq!(take(&log), ["canvas wheel Move zoom 1.00 turn 0.00 delta 0,1 touch false"]);
    // A fractional step is the driver's pinch: zoom exp(40 * 0.5 / 200), no scroll.
    d.platform.push(MAIN, line(0.5));
    d.step(1.1);
    assert_eq!(take(&log), ["canvas wheel Move zoom 1.11 turn 0.00 delta 0,0 touch false"]);
    // A whole notch within 0.25 s still belongs to the pinch; after that it scrolls again.
    d.platform.push(MAIN, line(1.0));
    d.step(1.3);
    d.platform.push(MAIN, line(1.0));
    d.step(2.0);
    assert_eq!(
        take(&log),
        ["canvas wheel Move zoom 1.22 turn 0.00 delta 0,0 touch false", "canvas wheel Move zoom 1.00 turn 0.00 delta 0,1 touch false"]
    );
    // The zoom modifiers zoom any step.
    d.platform.push(MAIN, wheel(50.0, 50.0, Vec2::new(0.0, -1.0), WheelUnit::Line, Mods::ALT));
    d.step(3.0);
    assert_eq!(take(&log), ["canvas wheel Move zoom 0.82 turn 0.00 delta 0,0 touch false"]);
    // On macOS Cmd+wheel is never a driver's pinch.
    d.runtime.set_os(Os::Mac);
    d.platform.push(MAIN, line(0.5));
    d.step(4.0);
    assert_eq!(take(&log), ["canvas wheel Move zoom 1.00 turn 0.00 delta 0,0.5 touch false"]);
}

#[test]
fn pen_samples_reach_the_press_owner_with_every_move() {
    let log: Log = Rc::default();
    let canvas = Probe::new("canvas", Vec2::new(300.0, 200.0), &log);
    let (mut d, _) = two_windows(vec![canvas], vec![]);
    take(&log);
    let pen = |phase, x: f32, pressure: f32, eraser: bool| {
        let mut sample = PointerSample::new(pressure);
        sample.eraser = eraser;
        WindowEvent::Pointer(
            PointerEvent::new(3, PointerKind::Pen, phase, PointerButton::Primary, Pos2::new(x, 50.0), Mods::NONE).with_sample(sample),
        )
    };
    d.platform.push(MAIN, pen(PointerPhase::Down, 10.0, 0.1, true));
    d.platform.push(MAIN, pen(PointerPhase::Move, 30.0, 0.4, true));
    d.platform.push(MAIN, pen(PointerPhase::Move, 32.0, 0.6, true));
    d.platform.push(MAIN, pen(PointerPhase::Move, 34.0, 0.9, true));
    d.step(1.0);
    assert_eq!(take(&log), ["canvas down", "canvas pen 0.6 eraser true", "canvas pen 0.9 eraser true"]);
}

#[test]
fn the_cursor_changes_only_when_the_pointer_reaches_another_node() {
    let log: Log = Rc::default();
    let mut splitter = Probe::new("splitter", Vec2::new(100.0, 100.0), &log);
    splitter.shape_cursor = Some(Cursor::Icon(CursorIcon::ResizeHorizontal));
    let mut field = Probe::new("field", Vec2::new(100.0, 100.0), &log);
    field.node_cursor = Some(Cursor::Icon(CursorIcon::Text));
    let plain = Probe::new("plain", Vec2::new(100.0, 100.0), &log);
    let (mut d, _) = two_windows(vec![splitter, field, plain], vec![]);
    let sent = |d: &mut Driver<TestPlatform, RecordingRenderer>| std::mem::take(&mut d.platform.cursors);
    sent(&mut d);
    let resize = Cursor::Icon(CursorIcon::ResizeHorizontal);

    for x in [10.0, 20.0, 30.0] {
        d.platform.push(MAIN, mouse(PointerPhase::Move, x, 10.0));
    }
    d.step(1.0);
    assert_eq!(sent(&mut d), [(MAIN, resize)], "moves within one node send one cursor");
    for x in [110.0, 120.0] {
        d.platform.push(MAIN, mouse(PointerPhase::Move, x, 10.0));
    }
    d.step(2.0);
    assert_eq!(sent(&mut d), [(MAIN, Cursor::Icon(CursorIcon::Text))]);
    d.platform.push(MAIN, mouse(PointerPhase::Move, 210.0, 10.0));
    d.step(3.0);
    assert_eq!(sent(&mut d), [(MAIN, Cursor::Default)]);

    // A press on the splitter keeps its cursor wherever the pointer goes, until the release.
    d.platform.push(MAIN, mouse(PointerPhase::Move, 50.0, 10.0));
    d.platform.push(MAIN, mouse(PointerPhase::Down, 50.0, 10.0));
    d.platform.push(MAIN, mouse(PointerPhase::Move, 150.0, 10.0));
    d.platform.push(MAIN, mouse(PointerPhase::Move, 250.0, 10.0));
    d.step(4.0);
    assert_eq!(sent(&mut d), [(MAIN, resize)]);
    d.platform.push(MAIN, mouse(PointerPhase::Up, 250.0, 10.0));
    d.step(5.0);
    assert_eq!(sent(&mut d), [(MAIN, Cursor::Default)]);
}

#[test]
fn the_ime_session_follows_key_focus_between_windows() {
    let log: Log = Rc::default();
    let mut a = Probe::new("a", Vec2::new(100.0, 20.0), &log);
    a.text = true;
    let mut b = Probe::new("b", Vec2::new(100.0, 20.0), &log);
    b.text = true;
    let (mut d, second) = two_windows(vec![a], vec![b]);
    d.platform.push(second, WindowEvent::Focus(false));
    click(&mut d.platform, MAIN, Pos2::new(10.0, 10.0));
    d.step(1.0);
    let requests = |d: &mut Driver<TestPlatform, RecordingRenderer>| std::mem::take(&mut d.platform.ime_requests);
    let start = |w: f32| ImeRequest::Start { purpose: ImePurpose::Normal, cursor: craft_ui_rt::Rect::from_min_size(Pos2::ZERO, Vec2::new(w, 20.0)) };
    assert_eq!(requests(&mut d), [(MAIN, start(100.0))]);

    // The user clicks into the second window: the session leaves MAIN and starts there.
    d.platform.push(MAIN, WindowEvent::Focus(false));
    d.platform.push(second, WindowEvent::Focus(true));
    click(&mut d.platform, second, Pos2::new(10.0, 10.0));
    d.step(2.0);
    assert_eq!(requests(&mut d), [(MAIN, ImeRequest::Stop), (second, start(100.0))]);

    // Back to MAIN by window focus alone: its focused field takes the session again.
    d.platform.push(second, WindowEvent::Focus(false));
    d.platform.push(MAIN, WindowEvent::Focus(true));
    d.step(3.0);
    assert_eq!(requests(&mut d), [(MAIN, start(100.0)), (second, ImeRequest::Stop)]);
}

#[test]
fn access_actions_reach_only_nodes_of_the_window_that_sent_them() {
    let log: Log = Rc::default();
    let b = Probe::new("b", Vec2::new(100.0, 20.0), &log);
    let ids = b.ids.clone();
    let (mut d, second) = two_windows(vec![], vec![b]);
    let target = ids.borrow()[0].1;
    let action = || {
        WindowEvent::Access(accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target.access_id(),
            data: None,
        })
    };
    d.platform.push(MAIN, action());
    d.step(1.0);
    assert!(take(&log).is_empty(), "MAIN's adapter cannot act on the second window's node");
    d.platform.push(second, action());
    d.step(2.0);
    assert_eq!(take(&log), ["b access Click"]);
}

#[test]
fn a_node_opens_a_window_through_the_driver() {
    let log: Log = Rc::default();
    let mut opener = Probe::new("opener", Vec2::new(100.0, 20.0), &log);
    opener.opens = true;
    let (mut d, second) = two_windows(vec![opener], vec![]);
    take(&log);
    click(&mut d.platform, MAIN, Pos2::new(10.0, 10.0));
    d.step(1.0);
    let opened = d.runtime.windows().find(|w| *w != MAIN && *w != second).expect("a third window");
    assert_eq!(take(&log), ["opener down".to_string(), "opener click".to_string(), format!("opener opened {}", opened.0)]);
    let report = d.step(2.0);
    assert!(report.frame.built >= 1, "the new window's root builds");
    assert!(d.runtime.root(opened).is_some());
}
