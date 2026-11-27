//! Count trials for the run model: a frame runs only the nodes and phases a change reached.

use std::cell::RefCell;
use std::rc::Rc;

use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{
    BuildCx, Constraints, Driver, Element, Item, LayoutCx, PaintCx, Pos2, Rgba, Runtime, Show, State, Stroke, Vec2, hash_key, virtual_band,
};

use super::common::{MAIN, TestPlatform, tokens};

const ROW: f32 = 20.0;

type Cells = Rc<RefCell<Vec<(String, State<bool>)>>>;

struct Scroll {
    offset: State<f32>,
    rows: State<Vec<String>>,
    cells: Cells,
}

impl Element for Scroll {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        Column { rows: self.rows, cells: self.cells.clone() }.show(cx);
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let offset = self.offset.get(cx);
        let content = cx.child_at(0);
        cx.layout_child(content, c.width_only());
        cx.place_child(content, Vec2::new(0.0, -offset));
        c.max
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.clip(bounds);
    }
}

struct Column {
    rows: State<Vec<String>>,
    cells: Cells,
}

impl Element for Column {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for label in self.rows.get(cx) {
            cx.keyed(&label, |cx| Row { label: label.clone(), cells: self.cells.clone() }.show(cx));
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let n = cx.child_count();
        for i in 0..n {
            let row = cx.child_at(i);
            cx.layout_child(row, Constraints::tight(Vec2::new(c.max.x, ROW)));
            cx.place_child(row, Vec2::new(0.0, i as f32 * ROW));
        }
        Vec2::new(c.max.x, n as f32 * ROW)
    }
}

struct Row {
    label: String,
    cells: Cells,
}

impl Element for Row {
    type Output = ();
    type Local = State<bool>;

    fn props_key(&self) -> u64 {
        hash_key(&self.label)
    }

    fn init(&self, cx: &mut BuildCx<'_>) -> State<bool> {
        let hovered = cx.state(false);
        self.cells.borrow_mut().push((self.label.clone(), hovered));
        hovered
    }

    fn paint(&self, hovered: &State<bool>, cx: &mut PaintCx<'_>) {
        let color = if hovered.get(cx) { Rgba([60, 60, 60, 255]) } else { Rgba([30, 30, 30, 255]) };
        let bounds = cx.bounds();
        cx.fill_rect(bounds, color);
    }
}

struct Setup {
    driver: Driver<TestPlatform, RecordingRenderer>,
    offset: State<f32>,
    rows: State<Vec<String>>,
    cells: Cells,
}

fn setup(n: usize) -> Setup {
    let size = Vec2::new(300.0, 200.0);
    let mut rt = Runtime::new(tokens());
    let offset = rt.store_mut().create(0.0);
    let rows = rt.store_mut().create((0..n).map(|i| format!("Layer {i}")).collect::<Vec<_>>());
    let cells: Cells = Rc::default();
    rt.set_root(MAIN, Scroll { offset, rows, cells: cells.clone() });
    let driver = Driver::new(rt, TestPlatform::new(size, 1.0), RecordingRenderer::default());
    Setup { driver, offset, rows, cells }
}

fn cell(s: &Setup, label: &str) -> State<bool> {
    s.cells.borrow().iter().find(|(l, _)| l == label).map(|(_, c)| *c).expect("row exists")
}

#[test]
fn a_frame_runs_only_what_changed() {
    let mut s = setup(8);
    let first = s.driver.step(0.0);
    assert!(first.ran);
    assert_eq!(first.frame.built, 10);

    let idle = s.driver.step(1.0);
    assert!(!idle.ran, "nothing dirty, no frame");
    assert!(!s.driver.platform.take_frame_request());

    // Hover: one row's paint, nothing else.
    let hovered = cell(&s, "Layer 3");
    hovered.set(&mut s.driver.runtime, true);
    let hover = s.driver.step(2.0);
    assert_eq!((hover.frame.built, hover.frame.laid_out, hover.frame.painted), (0, 0, 1));
    let rec = s.driver.renderer.last().expect("recorded");
    assert_eq!(rec.damage.len(), 1);
    let d = rec.damage[0];
    assert!(d.min.y <= 3.0 * ROW && d.max.y >= 4.0 * ROW && d.height() < 2.0 * ROW, "the row's damage, with room for anti-aliasing: {d:?}");
    assert_eq!(hover.render.prepared, 1);

    // Scroll: one transform moves; no row builds, lays out or repaints.
    s.offset.set(&mut s.driver.runtime, 30.0);
    let scroll = s.driver.step(3.0);
    assert_eq!((scroll.frame.built, scroll.frame.painted), (0, 0));
    assert_eq!(scroll.frame.laid_out, 1, "only the scroll node; the column keeps its size");
    assert_eq!(s.driver.renderer.last().map(|f| f.moved.len()), Some(1));

    // Insert at the front: keyed rows keep their nodes; only the column and the new row build.
    s.rows.update(&mut s.driver.runtime, |r| r.insert(0, "New".into()));
    let insert = s.driver.step(4.0);
    assert_eq!(insert.frame.built, 2);
    assert!(s.driver.renderer.last().is_some_and(|f| f.removed.is_empty()));
    assert!(cell(&s, "Layer 3") == hovered, "the row kept its node and its cell");

    // Remove: the node goes and is reported once.
    s.rows.update(&mut s.driver.runtime, |r| r.retain(|l| l != "Layer 5"));
    s.driver.step(5.0);
    assert_eq!(s.driver.renderer.last().map(|f| f.removed.len()), Some(1));
}

#[test]
fn surface_loss_keeps_state_and_redraws_everything_on_return() {
    let mut s = setup(4);
    s.driver.step(0.0);
    s.driver.platform.lose_surface(MAIN);
    let lost = s.driver.step(1.0);
    assert!(!lost.ran);
    cell(&s, "Layer 1").set(&mut s.driver.runtime, true);
    assert!(!s.driver.step(2.0).ran, "no surface, no frame");
    s.driver.platform.restore_surface(MAIN);
    let back = s.driver.step(3.0);
    assert!(back.ran);
    assert_eq!(back.frame.painted, 1, "only the row that changed re-records");
    assert!(s.driver.renderer.last().is_some_and(|f| f.full));
}

#[test]
fn the_virtual_band_moves_only_when_the_view_leaves_it() {
    let band = virtual_band(0.0, 100.0, ROW, 600, 4, 0..0);
    assert_eq!(band, 0..9);
    assert_eq!(virtual_band(40.0, 100.0, ROW, 600, 4, band.clone()), band, "still inside");
    assert_eq!(virtual_band(200.0, 100.0, ROW, 600, 4, band), 6..19);
    assert_eq!(virtual_band(11_950.0, 100.0, ROW, 600, 4, 0..0), 593..600);
}

/// A caret that blinks through `time_until`: drawn in the first half of each period.
struct Caret {
    half: f64,
}

impl Element for Caret {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let half = self.half;
        let now = cx.time_until(|t| Some(((t / half).floor() + 1.0) * half));
        if ((now / half).floor() as i64) % 2 == 0 {
            cx.line(Pos2::new(4.0, 2.0), Pos2::new(4.0, 18.0), Stroke::new(2.0, Rgba([192, 222, 255, 255])));
        }
    }
}

#[test]
fn a_blinking_caret_wakes_the_loop_only_at_its_blink_edges() {
    const HALF: f64 = 0.53;
    const TICK: f64 = 1.0 / 60.0;
    let size = Vec2::new(200.0, 32.0);
    let mut rt = Runtime::new(tokens());
    rt.set_root(MAIN, Caret { half: HALF });
    let mut driver = Driver::new(rt, TestPlatform::new(size, 1.0), RecordingRenderer::default());
    let caret_shown = |driver: &Driver<TestPlatform, RecordingRenderer>| {
        driver.renderer.last().is_some_and(|f| f.draws.iter().any(|d| d.list.items.iter().any(|i| matches!(i, Item::Line { .. }))))
    };

    let mut t = 0.0;
    while driver.step(t).ran {
        t += TICK;
    }
    let mut wake = driver.platform.take_wake().expect("a blinking caret asks for a wake");
    assert!(wake > t - TICK && wake <= t - TICK + HALF + 1e-9);
    assert!(!driver.platform.take_frame_request(), "no frame wanted between the edges");

    let mut shown = caret_shown(&driver);
    let mut edges = 0;
    while edges < 6 {
        t += TICK;
        let step = driver.step(t);
        assert_eq!(step.ran, t >= wake, "a frame runs at {t:.3} only if the wake at {wake:.3} came");
        if !step.ran {
            continue;
        }
        edges += 1;
        assert_eq!((step.frame.built, step.frame.laid_out, step.frame.painted), (0, 0, 1), "an edge repaints the caret only");
        assert_ne!(caret_shown(&driver), shown, "the caret toggles at each edge");
        shown = !shown;
        let next = driver.platform.take_wake().expect("the next edge");
        assert!((next - wake - HALF).abs() < 1e-9, "edges are a half period apart");
        assert!(!driver.platform.take_frame_request());
        wake = next;
    }
}

/// A label that lays out its text each time a cell it reads changes.
struct Label {
    text: State<&'static str>,
    tick: State<u32>,
    shaped: Rc<RefCell<Vec<craft_ui_rt::TextLayout>>>,
}

impl Element for Label {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let _ = self.tick.get(cx);
        let spec = craft_ui_rt::TextSpec {
            text: self.text.get(cx).into(),
            font: craft_ui_rt::FontSpec { slot: craft_theme::FontSlot::Ui, size: 12.0 },
            line_height: None,
            color: Rgba([255, 255, 255, 255]),
            max_width: None,
            overflow: craft_ui_rt::Overflow::Wrap,
        };
        let layout = cx.text_layout(&spec);
        let size = c.constrain(layout.size);
        self.shaped.borrow_mut().push(layout);
        size
    }
}

#[test]
fn a_relayout_shapes_its_text_again_only_when_the_text_changed() {
    let mut rt = Runtime::new(tokens());
    let text = rt.store_mut().create("Layer 1");
    let tick = rt.store_mut().create(0u32);
    let shaped = Rc::new(RefCell::new(Vec::new()));
    rt.set_root(MAIN, Label { text, tick, shaped: shaped.clone() });
    let mut driver = Driver::new(rt, TestPlatform::new(Vec2::new(200.0, 32.0), 1.0), RecordingRenderer::default());
    driver.step(0.0);
    tick.set(&mut driver.runtime, 1);
    assert_eq!(driver.step(1.0).frame.laid_out, 1);
    text.set(&mut driver.runtime, "Layer 2");
    assert_eq!(driver.step(2.0).frame.laid_out, 1);
    let shaped = shaped.borrow();
    assert_eq!(shaped.len(), 3);
    assert!(shaped[0].handle.ptr_eq(&shaped[1].handle), "the same text at the same scale is not shaped again");
    assert!(!shaped[1].handle.ptr_eq(&shaped[2].handle), "a changed text is");
}
