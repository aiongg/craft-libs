//! Trials of the headless platform and the rt test kit: injection routes to the right window, the
//! clock drives timed wakes exactly, and the bench counts what a frame ran.

use std::cell::RefCell;
use std::rc::Rc;

use craft_ui_platform_headless::{HeadlessPlatform, Input};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{
    BuildCx, Constraints, Element, Event, EventCx, Gesture, Handled, Item, Key, LayoutCx, Mods, PaintCx, Platform, Pos2, Rgba, RgbaImage, Sense,
    Show, SizeU, Stroke, Vec2, WindowCommand, WindowId,
};
use craft_ui_test_kit::rt::bench::bench;
use craft_ui_test_kit::rt::pixels::{Capture, Rule, check_pixels, compare_pixels};
use craft_ui_test_kit::rt::{Rig, Scenario, Step, first_difference};

#[path = "rt/root.rs"]
mod root;

type Log = Rc<RefCell<Vec<String>>>;

/// A focusable box that logs the presses and keys it gets and tints itself when hovered.
struct Boxed {
    name: &'static str,
    log: Log,
}

impl Element for Boxed {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        craft_ui_rt::hash_key(&self.name)
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(Vec2::new(100.0, 40.0))
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let hovered = cx.interaction().hovered;
        let bounds = cx.bounds();
        cx.fill_rect(bounds, Rgba([40 + 40 * u8::from(hovered), 40, 40, 255]));
        cx.hit(bounds, Sense::HOVER | Sense::CLICK, 0);
    }

    fn event(&self, _: &mut (), _: &mut EventCx<'_>, ev: &Event) -> Handled {
        let what = match ev {
            Event::Pointer(p) => match p.gesture {
                Gesture::Up { click: true, .. } => "click",
                _ => return Handled::Yes,
            },
            Event::Key(k) if k.down => "key",
            _ => return Handled::No,
        };
        self.log.borrow_mut().push(format!("{}:{what}", self.name));
        Handled::Yes
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Children at their own sizes, one under the other.
struct Column<F>(F);

impl<F: Fn(&mut BuildCx<'_>) + 'static> Element for Column<F> {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        (self.0)(cx);
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

fn boxes(names: &'static [&'static str], log: &Log) -> Column<impl Fn(&mut BuildCx<'_>) + 'static> {
    let log = log.clone();
    Column(move |cx: &mut BuildCx<'_>| {
        for name in names {
            Boxed { name, log: log.clone() }.show(&mut *cx);
        }
    })
}

#[test]
fn input_injected_in_one_window_reaches_only_that_window() {
    let log: Log = Rc::default();
    let mut rig = Rig::new(boxes(&["main"], &log), Vec2::new(300.0, 200.0), 1.0, RecordingRenderer::default());
    let second = rig.open_window("second", Vec2::new(200.0, 100.0), boxes(&["second"], &log));
    let main = rig.main;
    assert_ne!(second, main);
    assert_eq!(rig.step().windows_drawn, 2);

    let at = Pos2::new(10.0, 10.0);
    let scenario = Scenario::new(
        "two windows",
        vec![
            Step::on("click second", [(second, Input::mouse_move(at)), (second, Input::mouse_down(at)), (second, Input::mouse_up(at))]),
            Step::on(
                "keys",
                [
                    (main, Input::Key { key: Key::Tab, mods: Mods::NONE, down: true }),
                    (second, Input::Key { key: Key::A, mods: Mods::NONE, down: true }),
                ],
            ),
            Step::on("focus main", [(main, Input::Focus(true))]).expect(|s| (s.windows.len() == 2).then_some(()).ok_or("two windows".into())),
        ],
    );
    scenario.run(&mut rig).expect("scenario");
    assert_eq!(*log.borrow(), ["second:click", "second:key"], "the click and the key stayed in the window they were injected into");
    assert_eq!(rig.platform().focused(), Some(main));
    assert_eq!(rig.platform().window_state(second).map(|w| w.focused), Some(false));

    // A window command comes back as the events a desktop platform sends.
    rig.platform().window(second, WindowCommand::Size(Vec2::new(120.0, 80.0)));
    rig.platform().window(second, WindowCommand::Focus);
    rig.step();
    assert_eq!(rig.platform().focused(), Some(second));
    assert_eq!(
        {
            let env = rig.runtime().env(second);
            env.viewport.get(&mut rig.driver.runtime)
        },
        Vec2::new(120.0, 80.0)
    );
    assert_eq!(rig.platform().stacking().last(), Some(&second), "focus raises");
    rig.platform().window(second, WindowCommand::Close);
    rig.step();
    assert_eq!(rig.runtime().windows().collect::<Vec<WindowId>>(), [main]);
}

/// A caret drawn in the first half of each period, repainted only at the edges.
struct Caret(f64);

impl Element for Caret {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let half = self.0;
        let now = cx.time_until(|t| Some(((t / half).floor() + 1.0) * half));
        if ((now / half).floor() as i64) % 2 == 0 {
            cx.line(Pos2::new(4.0, 2.0), Pos2::new(4.0, 18.0), Stroke::new(2.0, Rgba([192, 222, 255, 255])));
        }
    }
}

#[test]
fn the_clock_runs_a_blink_wake_at_exactly_its_edge() {
    const HALF: f64 = 0.53;
    let mut rig = Rig::new(Caret(HALF), Vec2::new(200.0, 32.0), 1.0, RecordingRenderer::default());
    rig.settle(10);
    let shown = |rig: &Rig<RecordingRenderer>| {
        rig.driver.renderer.last().is_some_and(|f| f.draws.iter().any(|d| d.list.items.iter().any(|i| matches!(i, Item::Line { .. }))))
    };
    let mut visible = shown(&rig);
    let mut edge = rig.platform().pending_wake().expect("a blinking caret asks for a wake");
    for _ in 0..4 {
        // Just before the edge nothing runs; at the edge exactly one node repaints.
        rig.platform().set_now(edge - 1e-6);
        assert!(!rig.step().ran, "no frame before the edge");
        let step = rig.run_to_wake().expect("a wake is pending");
        assert_eq!(rig.platform().now(), edge, "the clock stands on the edge");
        assert!(step.ran);
        assert_eq!((step.frame.built, step.frame.laid_out, step.frame.painted), (0, 0, 1));
        assert_ne!(shown(&rig), visible, "the caret toggles");
        visible = !visible;
        let next = rig.platform().pending_wake().expect("the next edge");
        assert!((next - edge - HALF).abs() < 1e-9);
        edge = next;
    }
}

#[test]
fn the_bench_reports_frames_run_and_nodes_painted() {
    let log: Log = Rc::default();
    let make = || {
        let mut rig = Rig::new(boxes(&["a", "b"], &log), Vec2::new(200.0, 100.0), 1.0, RecordingRenderer::default());
        rig.settle(10);
        rig
    };
    let over = |y: f32| Input::mouse_move(Pos2::new(10.0, y));
    let scenario = Scenario::new("hover a, hover b, idle", vec![Step::input("a", [over(10.0)]), Step::input("b", [over(50.0)]), Step::idle("idle")]);
    let row = bench(make, &scenario, 0, 30);
    assert_eq!(row.frames, 30);
    assert_eq!(row.ran_frames, 20, "the idle frame does not run");
    assert_eq!(row.painted, 10 * (1 + 2), "entering a repaints a; moving to b repaints a and b");
    assert_eq!((row.built, row.laid_out), (0, 0));
    assert!(row.prepared <= row.draws);
}

fn solid(size: SizeU, rgba: [u8; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(size);
    img.pixels = rgba.repeat((size.width * size.height) as usize);
    img
}

#[test]
fn pixel_rules_and_snapshot_diffs_find_what_differs() {
    let size = SizeU::new(20, 10);
    let a = solid(size, [10, 20, 30, 255]);
    let mut b = a.clone();
    assert_eq!(compare_pixels(&a, &b, &[]).map(|s| s.differing), Some(0));
    b.pixels[0..4].copy_from_slice(&[14, 20, 30, 255]);
    let stats = compare_pixels(&a, &b, &[]).expect("same size");
    assert_eq!((stats.differing, stats.max_delta), (1, 4));

    let snapshot = |bounds| craft_ui_test_kit::rt::WindowSnapshot {
        window: HeadlessPlatform::MAIN,
        focus: None,
        access: vec![craft_ui_test_kit::rt::AccessNode {
            role: "Button".into(),
            name: "OK".into(),
            value: None,
            description: None,
            toggled: None,
            selected: None,
            focused: false,
            bounds,
        }],
    };
    let capture = |image, bounds| Capture { image, snapshot: snapshot(bounds), text_runs: Vec::new(), scale: 1.0 };
    let reference = capture(a.clone(), [0.0, 0.0, 10.0, 10.0]);
    assert!(!check_pixels(&reference, &capture(b.clone(), [0.0, 0.0, 10.0, 10.0]), Rule::Identical).pass, "one pixel breaks rule (a)");
    assert!(check_pixels(&reference, &capture(b, [0.2, 0.0, 10.0, 10.0]), Rule::Tolerance).pass, "one pixel and a 0.2 pt shift are inside rule (b)");
    assert!(!check_pixels(&reference, &capture(a, [1.0, 0.0, 11.0, 10.0]), Rule::Tolerance).pass, "a 1 pt shift is not");

    let snap = |bounds| craft_ui_test_kit::rt::Snapshot { windows: vec![snapshot(bounds)], state: "s".into() };
    let labels = vec!["one".to_owned(), "two".to_owned()];
    let a = [snap([0.0, 0.0, 10.0, 10.0]), snap([0.0, 0.0, 10.0, 10.0])];
    let b = [snap([0.0, 0.0, 10.0, 10.0]), snap([0.0, 0.0, 10.0, 12.0])];
    let diff = first_difference(&labels, &a, &b).expect("the second step differs");
    assert_eq!((diff.step, diff.label.as_str()), (1, "two"));
}
