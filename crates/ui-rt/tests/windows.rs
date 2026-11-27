//! Trials for windows, layers, damage merging, image updates and wakes.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{
    Align, BuildCx, Constraints, Driver, Element, Event, EventCx, Gesture, Handled, ImageChange, ImageKey, Key, Layer, LayerSpec, LayoutCx, Mods,
    NodeId, OutsidePress, PaintCx, Placement, PointerKind, PointerPhase, Pos2, RectU, Rgba, RgbaImage, Runtime, Sense, Show, Side, SizeU, State,
    Vec2, WindowEvent, WindowId, WindowSpec,
};

use super::common::{MAIN, TestPlatform, click, pointer, tokens};

/// What the trials' elements saw, by name.
#[derive(Default)]
struct Log {
    events: Vec<(String, String)>,
    nodes: Vec<(String, NodeId)>,
}

type Shared = Rc<RefCell<Log>>;

fn node(log: &Shared, name: &str) -> NodeId {
    log.borrow().nodes.iter().rev().find(|(n, _)| n == name).map(|(_, id)| *id).unwrap_or_else(|| panic!("no node {name}"))
}

fn events(log: &Shared, name: &str) -> Vec<String> {
    log.borrow().events.iter().filter(|(n, _)| n == name).map(|(_, e)| e.clone()).collect()
}

/// A focusable box of a fixed size that logs presses, keys and dismissals.
struct Boxed {
    name: &'static str,
    size: Vec2,
    log: Shared,
}

impl Element for Boxed {
    type Output = bool;
    type Local = ();

    fn props_key(&self) -> u64 {
        craft_ui_rt::hash_key(&self.name)
    }

    fn init(&self, cx: &mut BuildCx<'_>) {
        self.log.borrow_mut().nodes.push((self.name.into(), cx.node()));
    }

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(self.size)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let hovered = cx.interaction().hovered;
        let bounds = cx.bounds();
        cx.fill_rect(bounds, Rgba([40 + 40 * u8::from(hovered), 40, 40, 255]));
        cx.hit(bounds, Sense::HOVER | Sense::CLICK, 0);
    }

    fn event(&self, _: &mut (), cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        let what = match ev {
            Event::Pointer(p) => match p.gesture {
                Gesture::Down { .. } => "down",
                Gesture::Up { click: true, .. } => "click",
                _ => return Handled::Yes,
            },
            Event::Key(k) if k.down => "key",
            Event::Dismiss(_) => {
                *cx.output::<bool>() = true;
                "dismiss"
            }
            _ => return Handled::No,
        };
        self.log.borrow_mut().events.push((self.name.into(), what.into()));
        Handled::Yes
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Lays out its children in a column at their own sizes.
struct Column<F> {
    children: F,
}

impl<F: Fn(&mut BuildCx<'_>) + 'static> Element for Column<F> {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        (self.children)(cx);
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let mut y = 0.0;
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            let s = cx.layout_child(child, c.loosen());
            cx.place_child(child, Vec2::new(0.0, y));
            y += s.y;
        }
        c.max
    }
}

fn column(f: impl Fn(&mut BuildCx<'_>) + 'static) -> Column<impl Fn(&mut BuildCx<'_>) + 'static> {
    Column { children: f }
}

#[test]
fn two_windows_route_events_and_draw_separately() {
    let log: Shared = Rc::default();
    let mut rt = Runtime::new(tokens());
    let l = log.clone();
    rt.set_root(
        MAIN,
        column(move |cx| {
            Boxed { name: "main", size: Vec2::new(100.0, 40.0), log: l.clone() }.show(cx);
        }),
    );
    let mut d = Driver::new(rt, TestPlatform::new(Vec2::new(300.0, 200.0), 1.0), RecordingRenderer::default());
    let l = log.clone();
    let second = d.open_window(
        &WindowSpec { size: Some(Vec2::new(200.0, 100.0)), ..WindowSpec::new("second") },
        column(move |cx| {
            Boxed { name: "second", size: Vec2::new(100.0, 40.0), log: l.clone() }.show(cx);
        }),
    );
    let first = d.step(0.0);
    assert_eq!(first.windows_drawn, 2);
    assert_ne!(second, MAIN);

    // A click at the same window position reaches only the window it came from.
    click(&mut d.platform, second, Pos2::new(10.0, 10.0));
    d.step(1.0);
    assert_eq!(events(&log, "second"), ["down", "click"]);
    assert!(events(&log, "main").is_empty());
    assert_eq!(d.runtime.focused(second), Some(node(&log, "second")));
    assert_eq!(d.runtime.focused(MAIN), None, "each window keeps its own focus");

    // Keys go to the focused node of their window.
    d.platform.push(MAIN, WindowEvent::Key { key: Key::Tab, mods: Mods::NONE, down: true, repeat: false });
    d.platform.push(second, WindowEvent::Key { key: Key::A, mods: Mods::NONE, down: true, repeat: false });
    d.step(2.0);
    assert_eq!(d.runtime.focused(MAIN), Some(node(&log, "main")));
    assert_eq!(events(&log, "second").last().map(String::as_str), Some("key"));
    assert!(!events(&log, "main").contains(&"key".to_string()));

    // The pointer leaving the box it clicked repaints one node and draws one window.
    d.platform.push(second, pointer(PointerKind::Mouse, PointerPhase::Move, Pos2::new(150.0, 80.0)));
    let hover = d.step(3.0);
    assert_eq!((hover.frame.painted, hover.windows_drawn), (1, 1));
    assert_eq!(d.renderer.last().map(|f| f.window), Some(second));

    // Each window has its own access tree, rooted at its window node.
    let tree_of = |w: WindowId| d.platform.updates_for(w).find_map(|u| u.tree.as_ref().map(|t| t.root));
    assert_eq!(tree_of(MAIN), Some(MAIN.root().access_id()));
    assert_eq!(tree_of(second), Some(second.root().access_id()));

    // Closing a window drops its nodes; the other window is untouched.
    d.platform.push(second, WindowEvent::CloseRequested);
    d.step(4.0);
    assert_eq!(d.runtime.take_close_requests(), [second]);
    d.runtime.window_command(second, craft_ui_rt::WindowCommand::Close);
    d.step(5.0);
    d.step(6.0);
    assert!(d.runtime.window_of(node(&log, "second")).is_none());
    assert_eq!(d.runtime.windows().collect::<Vec<_>>(), [MAIN]);
}

/// Shows an anchor button and, while `open`, a menu on the menu layer under it; the menu shows a
/// submenu beside itself.
struct MenuHost {
    open: State<bool>,
    log: Shared,
}

impl Element for MenuHost {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        Boxed { name: "anchor", size: Vec2::new(80.0, 20.0), log: self.log.clone() }.show(cx);
        let anchor = cx.last_child().expect("the anchor");
        Boxed { name: "content", size: Vec2::new(300.0, 200.0), log: self.log.clone() }.show(cx);
        if self.open.get(cx) {
            let spec = LayerSpec::new(Placement::Anchor { node: anchor, side: Side::Below, align: Align::Min, gap: 2.0 })
                .outside_press(OutsidePress::DismissAndConsume);
            let log = self.log.clone();
            let dismissed = cx.layer(Layer::Menu, "menu", spec, Menu { log });
            if dismissed {
                self.open.set(cx, false);
            }
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let mut y = 0.0;
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            let s = cx.layout_child(child, c.loosen());
            cx.place_child(child, Vec2::new(0.0, y));
            y += s.y;
        }
        c.max
    }
}

struct Menu {
    log: Shared,
}

impl Element for Menu {
    type Output = bool;
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, cx: &mut BuildCx<'_>) {
        self.log.borrow_mut().nodes.push(("menu".into(), cx.node()));
    }

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        let me = cx.node();
        let spec = LayerSpec::new(Placement::Anchor { node: me, side: Side::Right, align: Align::Min, gap: 0.0 });
        cx.layer(Layer::Menu, "sub", spec, Boxed { name: "submenu", size: Vec2::new(60.0, 40.0), log: self.log.clone() });
    }

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        Vec2::new(120.0, 60.0)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.fill_rect(bounds, Rgba([70, 70, 70, 255]));
        // A menu takes presses on its background, so they never reach what lies under it.
        cx.hit(bounds, Sense::CLICK, 0);
    }

    fn event(&self, _: &mut (), cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        if let Event::Dismiss(_) = ev {
            self.log.borrow_mut().events.push(("menu".into(), "dismiss".into()));
            *cx.output::<bool>() = true;
            return Handled::Yes;
        }
        Handled::No
    }
}

#[test]
fn a_menu_layer_child_is_placed_at_its_anchor_and_dismissed_by_an_outside_press() {
    let log: Shared = Rc::default();
    let mut rt = Runtime::new(tokens());
    let open = rt.store_mut().create(true);
    rt.set_root(MAIN, MenuHost { open, log: log.clone() });
    let mut d = Driver::new(rt, TestPlatform::new(Vec2::new(400.0, 300.0), 1.0), RecordingRenderer::default());
    d.step(0.0);

    let menu = d.runtime.window_rect(node(&log, "menu")).expect("the menu is shown");
    assert_eq!(menu.min, Pos2::new(0.0, 22.0), "under the anchor, 2 pt apart");
    let sub = d.runtime.window_rect(node(&log, "submenu")).expect("the submenu is shown");
    assert_eq!(sub.min, Pos2::new(120.0, 22.0), "beside the menu");

    // Inside the menu, inside the submenu and on the anchor: no dismissal.
    click(&mut d.platform, MAIN, Pos2::new(30.0, 40.0));
    click(&mut d.platform, MAIN, Pos2::new(150.0, 30.0));
    click(&mut d.platform, MAIN, Pos2::new(10.0, 10.0));
    d.step(1.0);
    assert!(events(&log, "menu").is_empty());
    assert_eq!(events(&log, "submenu"), ["down", "click"], "the submenu sits above the content and takes its press");
    assert!(events(&log, "content").is_empty(), "nothing reached the content under the menus");

    // Outside: the menu is dismissed, the press is consumed, and both layer children go.
    click(&mut d.platform, MAIN, Pos2::new(250.0, 150.0));
    d.step(2.0);
    assert_eq!(events(&log, "menu"), ["dismiss"]);
    assert!(events(&log, "content").is_empty(), "the dismissing press activates nothing");
    assert!(!*d.runtime.store().peek(open));
    assert!(d.runtime.window_rect(node(&log, "menu")).is_none());
    assert!(d.runtime.window_rect(node(&log, "submenu")).is_none(), "what the menu owned goes with it");

    // With the menu closed, the same press reaches the content.
    click(&mut d.platform, MAIN, Pos2::new(250.0, 150.0));
    d.step(3.0);
    assert_eq!(events(&log, "content"), ["down", "click"]);
}

/// A grid of cells, each painting from its own cell.
struct Grid {
    cells: Rc<RefCell<Vec<State<bool>>>>,
}

const GRID: usize = 20;
const CELL: f32 = 50.0;

impl Element for Grid {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for i in 0..GRID * GRID {
            cx.keyed(i, |cx| Cell { cells: self.cells.clone() }.show(cx));
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(Vec2::splat(CELL - 10.0)));
            cx.place_child(child, Vec2::new((i % GRID) as f32 * CELL, (i / GRID) as f32 * CELL));
        }
        c.max
    }
}

struct Cell {
    cells: Rc<RefCell<Vec<State<bool>>>>,
}

impl Element for Cell {
    type Output = ();
    type Local = State<bool>;

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, cx: &mut BuildCx<'_>) -> State<bool> {
        let on = cx.state(false);
        self.cells.borrow_mut().push(on);
        on
    }

    fn paint(&self, on: &State<bool>, cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        let color = if on.get(cx) { Rgba([200, 0, 0, 255]) } else { Rgba([0, 0, 0, 255]) };
        cx.fill_rect(bounds, color);
    }
}

#[test]
fn damage_merge_bounds_the_redraw_area() {
    let cells: Rc<RefCell<Vec<State<bool>>>> = Rc::default();
    let mut rt = Runtime::new(tokens());
    rt.set_root(MAIN, Grid { cells: cells.clone() });
    let size = Vec2::splat(GRID as f32 * CELL);
    let mut d = Driver::new(rt, TestPlatform::new(size, 1.0), RecordingRenderer::default());
    d.step(0.0);

    // 40 cells scattered over the window change at once.
    for i in (0..GRID * GRID).step_by(10) {
        let cell = cells.borrow()[i];
        cell.set(&mut d.runtime, true);
    }
    let step = d.step(1.0);
    assert_eq!(step.frame.painted, 40);
    let damage = &d.renderer.last().expect("drawn").damage;
    let area: f32 = damage.iter().map(|r| r.area()).sum();
    let window = size.x * size.y;
    assert!(damage.len() <= 8, "{} rects", damage.len());
    assert!(area < 0.5 * window, "redrawn {:.0}% of the window", 100.0 * area / window);
}

/// Shows an image and logs its paints.
struct Picture {
    paints: Rc<RefCell<u32>>,
}

impl Element for Picture {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, cx: &mut BuildCx<'_>) {
        cx.images().insert(ImageKey(1), Arc::new(RgbaImage::new(SizeU::new(64, 64))));
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        *self.paints.borrow_mut() += 1;
        cx.image(ImageKey(1), craft_ui_rt::Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::splat(64.0)), Rgba([255; 4]));
    }
}

#[test]
fn an_image_sub_rect_update_uploads_that_part_and_damages_only_its_draw() {
    let paints = Rc::new(RefCell::new(0));
    let mut rt = Runtime::new(tokens());
    rt.set_root(MAIN, Picture { paints: paints.clone() });
    let mut d = Driver::new(rt, TestPlatform::new(Vec2::new(400.0, 300.0), 1.0), RecordingRenderer::default());
    d.step(0.0);
    assert_eq!(d.renderer.image_syncs, [ImageChange::Set(ImageKey(1))]);

    let rect = RectU::new(8, 8, 16, 12);
    assert!(d.runtime.images_mut().update(ImageKey(1), rect, &[255; 8 * 4 * 4]));
    let step = d.step(1.0);
    assert!(step.ran);
    assert_eq!(*paints.borrow(), 1, "no display list is re-recorded");
    assert_eq!(d.renderer.image_syncs.last(), Some(&ImageChange::Pixels(ImageKey(1), rect)));
    let damage = &d.renderer.last().expect("drawn").damage;
    assert_eq!(damage.len(), 1);
    assert!(damage[0].min.x >= 99.0 && damage[0].max.x <= 165.0 && damage[0].max.y <= 115.0, "the image's draw: {:?}", damage[0]);
}

/// Counts its builds; reads the wake cell so a background job's wake reruns it.
struct Inbox {
    builds: Rc<RefCell<u32>>,
}

impl Element for Inbox {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        let wakes = cx.env().wakes;
        wakes.get(cx);
        *self.builds.borrow_mut() += 1;
    }
}

#[test]
fn a_waker_from_another_thread_reruns_what_reads_the_wake_cell() {
    let builds = Rc::new(RefCell::new(0));
    let mut rt = Runtime::new(tokens());
    rt.set_root(MAIN, Inbox { builds: builds.clone() });
    let mut d = Driver::new(rt, TestPlatform::new(Vec2::new(100.0, 100.0), 1.0), RecordingRenderer::default());
    d.step(0.0);
    assert!(!d.step(1.0).ran);
    let waker = d.waker();
    std::thread::spawn(move || waker.wake()).join().expect("the job thread");
    let step = d.step(2.0);
    assert!(step.ran);
    assert_eq!((*builds.borrow(), step.frame.built), (2, 1));
}
