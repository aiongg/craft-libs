//! Risk R1 (spike plan 7.1): an app root shaped like PhotoCraft's (8 versioned shell slots, 12
//! memoized dock panels, a canvas surface) on the headless platform. The root, each slot and each
//! panel count their builds; the frame report counts the nodes built, laid out and painted.
//! Pass: hover, wheel and idle rerun nothing outside the node whose pixels change.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{
    BuildCx, Constraints, Element, Event, EventCx, FrameReport, Handled, LayoutCx, PaintCx, Pos2, Rect, Rgba, Sense, State, SurfaceKey, Vec2,
};
use craft_ui_test_kit::rt::Rig;
use craft_ui_test_kit::rt::rig::FRAME;

const SLOTS: usize = 8;
const PANELS: usize = 12;
const ROWS: usize = 4;
/// The slot that holds the dock and the one that hosts the canvas.
const DOCK: usize = 5;
const CANVAS: usize = 7;
const SURFACE: SurfaceKey = SurfaceKey(1);
const WINDOW: Vec2 = Vec2::new(1600.0, 1000.0);

#[derive(Default)]
struct Counts {
    root: Cell<u32>,
    slots: [Cell<u32>; SLOTS],
    panels: [Cell<u32>; PANELS],
    /// The slot versions and the canvas zoom, created in the root's `init`.
    cells: RefCell<Option<([State<u64>; SLOTS], State<f32>)>>,
}

impl Counts {
    fn total(&self) -> (u32, u32, u32) {
        let sum = |c: &[Cell<u32>]| c.iter().map(Cell::get).sum();
        (self.root.get(), sum(&self.slots), sum(&self.panels))
    }
}

/// Where each slot sits: title, menu, options, document tabs, tool rail, dock, status bar,
/// canvas.
fn slot_rect(i: usize) -> Rect {
    let r = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
    match i {
        0 => r(0.0, 0.0, 1600.0, 30.0),
        1 => r(0.0, 30.0, 1600.0, 24.0),
        2 => r(0.0, 54.0, 1600.0, 36.0),
        3 => r(40.0, 90.0, 1260.0, 28.0),
        4 => r(0.0, 90.0, 40.0, 886.0),
        DOCK => r(1300.0, 90.0, 300.0, 886.0),
        6 => r(0.0, 976.0, 1600.0, 24.0),
        _ => r(40.0, 118.0, 1260.0, 858.0),
    }
}

fn panel_height() -> f32 {
    slot_rect(DOCK).height() / PANELS as f32
}

/// The row centre the hover trials point at.
fn row_center(panel: usize, row: usize) -> Pos2 {
    let dock = slot_rect(DOCK);
    let y = dock.min.y + panel as f32 * panel_height() + (row as f32 + 0.5) * 20.0;
    Pos2::new(dock.center().x, y)
}

struct AppRoot {
    counts: Rc<Counts>,
}

impl Element for AppRoot {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, cx: &mut BuildCx<'_>) {
        let versions = std::array::from_fn(|_| cx.state(0u64));
        let zoom = cx.state(1.0f32);
        *self.counts.cells.borrow_mut() = Some((versions, zoom));
    }

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        self.counts.root.set(self.counts.root.get() + 1);
        let (versions, zoom) = self.counts.cells.borrow().expect("init made the cells");
        for (index, version) in versions.into_iter().enumerate() {
            cx.child(Slot { index, version, zoom, counts: self.counts.clone() });
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        for i in 0..cx.child_count() {
            let rect = slot_rect(i);
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(rect.size()));
            cx.place_child(child, rect.min.to_vec2());
        }
        c.max
    }
}

/// A shell slot: it reads its own version at its own turn, so a new version rebuilds this slot
/// and nothing around it.
struct Slot {
    index: usize,
    version: State<u64>,
    zoom: State<f32>,
    counts: Rc<Counts>,
}

impl Element for Slot {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        self.index as u64
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        let count = &self.counts.slots[self.index];
        count.set(count.get() + 1);
        let version = self.version.get(cx);
        match self.index {
            DOCK => {
                for panel in 0..PANELS {
                    let counts = self.counts.clone();
                    cx.memo(panel, panel, move |&panel, cx| {
                        let count = &counts.panels[panel];
                        count.set(count.get() + 1);
                        cx.child(Panel);
                    });
                }
            }
            CANVAS => {
                cx.child(Canvas { zoom: self.zoom });
            }
            _ => {
                cx.child(Bar { version });
            }
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let size = c.max;
        let n = cx.child_count();
        let h = if self.index == DOCK { panel_height() } else { size.y };
        for i in 0..n {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(Vec2::new(size.x, h)));
            cx.place_child(child, Vec2::new(0.0, i as f32 * h));
        }
        size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }
}

/// A shell bar's content: paints its version as a shade.
struct Bar {
    version: u64,
}

impl Element for Bar {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        self.version
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let shade = 40 + (self.version % 100) as u8;
        cx.fill_rect(cx.bounds(), Rgba([shade, shade, shade, 255]));
    }
}

/// A dock panel: a header and [`ROWS`] hover-tinted rows.
struct Panel;

impl Element for Panel {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for row in 0..ROWS {
            cx.child(Row { row });
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(Vec2::new(c.max.x, 20.0)));
            cx.place_child(child, Vec2::new(0.0, i as f32 * 20.0));
        }
        c.max
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        cx.fill_rect(cx.bounds(), Rgba([32, 32, 36, 255]));
    }
}

struct Row {
    row: usize,
}

impl Element for Row {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        self.row as u64
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        let hovered = cx.interaction().hovered;
        cx.fill_rect(bounds, Rgba([40 + 30 * u8::from(hovered), 44, 52, 255]));
        cx.hit(bounds, Sense::HOVER | Sense::CLICK, 0);
    }
}

/// The document canvas: an app surface under a zoom the wheel changes.
struct Canvas {
    zoom: State<f32>,
}

impl Element for Canvas {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        let zoom = self.zoom.get(cx);
        let image = Rect::from_center_size(bounds.center(), Vec2::new(600.0, 400.0) * zoom).intersect(bounds);
        cx.surface(SURFACE, image);
        cx.hit(bounds, Sense::HOVER | Sense::WHEEL, 0);
    }

    fn event(&self, _: &mut (), cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        if let Event::Wheel(w) = ev {
            let step = 1.0 + w.delta.y / 100.0;
            self.zoom.update(cx, |z| *z = (*z * step).clamp(0.1, 8.0));
            return Handled::Yes;
        }
        Handled::No
    }
}

fn rig() -> (Rig<RecordingRenderer>, Rc<Counts>) {
    let counts = Rc::new(Counts::default());
    let mut rig = Rig::new(AppRoot { counts: counts.clone() }, WINDOW, 1.0, RecordingRenderer::default());
    rig.settle(10);
    (rig, counts)
}

fn ran(r: &FrameReport) -> (u32, u32, u32) {
    (r.built, r.laid_out, r.painted)
}

#[test]
fn a_photocraft_shaped_root_reruns_only_what_hover_wheel_or_a_slot_version_changes() {
    let (mut rig, counts) = rig();
    let main = rig.main;
    assert_eq!(counts.total(), (1, SLOTS as u32, PANELS as u32), "the first frame builds each scope once");

    // Idle: no frame runs.
    for _ in 0..30 {
        assert!(!rig.frame(FRAME).ran, "an idle frame ran");
    }

    // Hover into a row of panel 5, then along to the next row: only the rows entered and left
    // repaint; no scope rebuilds and nothing is laid out.
    rig.platform().mouse_move(main, row_center(5, 1));
    let into = rig.frame(FRAME);
    rig.platform().mouse_move(main, row_center(5, 2));
    let along = rig.frame(FRAME);
    rig.platform().mouse_move(main, row_center(5, 2) + Vec2::new(30.0, 0.0));
    let within = rig.frame(FRAME);
    eprintln!("R1 hover: into {:?}, along {:?}, within {}", ran(&into.frame), ran(&along.frame), within.ran);
    assert_eq!(ran(&into.frame), (0, 0, 1), "hover into a row");
    assert_eq!(ran(&along.frame), (0, 0, 2), "hover from one row to the next");
    assert!(!within.ran, "a move inside the hovered row runs no frame");
    assert_eq!(counts.total(), (1, SLOTS as u32, PANELS as u32), "hover rebuilt a scope");

    // Wheel over the canvas: the zoom changes, the canvas repaints, nothing else runs.
    let over = slot_rect(CANVAS).center();
    rig.platform().mouse_move(main, over);
    rig.frame(FRAME);
    let mut wheels = Vec::new();
    for _ in 0..5 {
        rig.platform().wheel(main, over, Vec2::new(0.0, 10.0));
        wheels.push(ran(&rig.frame(FRAME).frame));
    }
    eprintln!("R1 wheel: {wheels:?}");
    assert!(wheels.iter().all(|w| *w == (0, 0, 1)), "each wheel step repaints the canvas only: {wheels:?}");
    assert_eq!(counts.total(), (1, SLOTS as u32, PANELS as u32), "the wheel rebuilt a scope");

    // A slot's new version (the options bar after a tool switch) rebuilds that slot and its bar:
    // not the root, not the other slots, not the panels.
    let (versions, _) = counts.cells.borrow().expect("cells");
    versions[2].update(&mut rig.driver.runtime, |v| *v += 1);
    let bump = rig.frame(FRAME);
    eprintln!("R1 slot version: {:?}", ran(&bump.frame));
    assert_eq!(counts.root.get(), 1, "the root reran");
    assert_eq!(counts.slots[2].get(), 2, "the options slot rebuilt once");
    assert_eq!(counts.total(), (1, SLOTS as u32 + 1, PANELS as u32), "only the options slot rebuilt");
    assert_eq!(bump.frame.built, 2, "the slot and its new bar");

    // And idle again.
    assert_eq!(rig.settle(5), 0, "frames kept running after the changes");
}
