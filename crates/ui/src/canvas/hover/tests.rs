//! Tests of the hover outline and lookup (task K10), over `ManualJobs` so each job finishes when
//! the test says.

use std::cell::Cell;
use std::sync::Arc;

use craft_host::ManualJobs;
use egui::{Context, Event, PointerButton, RawInput};
use kurbo::{Point, Rect};

use super::*;
use crate::canvas::CanvasView;
use crate::theme::{Base, install};

/// A context with the suite theme, a lookup over manual jobs, and a pass runner.
struct Rig {
    ctx: Context,
    jobs: Arc<ManualJobs>,
    lookup: HoverLookup<u32>,
    started: Cell<u32>,
}

#[derive(Clone, Copy)]
struct Pass {
    at: Option<Point>,
    time: f64,
    revision: u64,
    key: u64,
    enabled: bool,
    empty_here: bool,
    pressed: bool,
}

impl Pass {
    fn at(x: f64, y: f64, time: f64) -> Self {
        Pass { at: Some(Point::new(x, y)), time, revision: 1, key: 1, enabled: true, empty_here: false, pressed: false }
    }
}

/// What a pass gave: the shown value, `pending` and the hint.
type Seen = (Option<u32>, bool, Option<HoverHint>);

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let jobs = Arc::new(ManualJobs::new());
        let lookup = HoverLookup::new(&ctx, Some(jobs.clone()));
        Rig { ctx, jobs, lookup, started: Cell::new(0) }
    }

    /// One pass; a job started here answers `answer`. A value covers the points within 10 px of
    /// the one it was found at (its number).
    fn run(&mut self, pass: Pass, answer: HoverAnswer<u32>) -> Seen {
        let mut events = Vec::new();
        if pass.pressed {
            let pos = egui::pos2(1.0, 1.0);
            events.push(Event::PointerMoved(pos));
            events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: Default::default() });
        }
        let mut seen = (None, false, None);
        let (lookup, started) = (&mut self.lookup, &self.started);
        let mut output = self.ctx.run_ui(RawInput { time: Some(pass.time), events, ..RawInput::default() }, |ui| {
            let covers = |r: &u32, p: Point| (p.x - f64::from(*r)).abs() < 10.0;
            let query = HoverQuery {
                at: pass.at,
                revision: pass.revision,
                key: pass.key,
                enabled: pass.enabled,
                empty_here: pass.empty_here,
                covers: &covers,
            };
            let up = lookup.update(ui.ctx(), query, |_| {
                started.set(started.get() + 1);
                let answer = answer.clone();
                move |_: &craft_host::queue::Cancel| answer
            });
            seen = (up.shown.copied(), up.pending, up.hint);
        });
        output.textures_delta.clear();
        seen
    }
}

// The pointer rests 120 ms on one pixel before `job` is called; one job at a time, no second job
// while one runs; `pending` is true until its answer arrives.
#[test]
fn hover_waits_debounce_then_looks_up_once() {
    let mut rig = Rig::new();
    let found = HoverAnswer::Found(50);
    assert_eq!(rig.run(Pass::at(50.5, 5.0, 0.0), found.clone()), (None, true, None));
    assert_eq!(rig.run(Pass::at(50.5, 5.0, 0.10), found.clone()), (None, true, None));
    assert_eq!(rig.started.get(), 0);
    assert_eq!(rig.run(Pass::at(50.5, 5.0, 0.13), found.clone()), (None, true, None));
    assert_eq!(rig.started.get(), 1);
    // The job is queued; more passes start no second job.
    rig.run(Pass::at(50.5, 5.0, 0.20), found.clone());
    assert_eq!(rig.started.get(), 1);
    rig.jobs.run_all();
    assert_eq!(rig.run(Pass::at(50.5, 5.0, 0.30), found), (Some(50), false, None));
    assert_eq!(rig.started.get(), 1);
}

// A Found answer is shown; moving to points it `covers` shows it again with no job.
#[test]
fn hover_cache_serves_points_inside_found_mask() {
    let mut rig = Rig::new();
    let found = HoverAnswer::Found(50);
    rig.run(Pass::at(50.0, 5.0, 0.0), found.clone());
    rig.run(Pass::at(50.0, 5.0, 0.2), found.clone());
    rig.jobs.run_all();
    assert_eq!(rig.run(Pass::at(50.0, 5.0, 0.3), found.clone()).0, Some(50));
    assert_eq!(rig.run(Pass::at(56.0, 9.0, 0.31), found.clone()).0, Some(50));
    assert_eq!(rig.started.get(), 1);
    // Outside the object it is dropped, and a new rest looks up again.
    assert_eq!(rig.run(Pass::at(80.0, 5.0, 0.32), found.clone()).0, None);
}

// A Nothing answer suppresses lookups within 2 px on each axis; only the newest 64 misses count.
#[test]
fn hover_miss_suppresses_nearby_points() {
    let mut rig = Rig::new();
    let nothing = HoverAnswer::Nothing;
    rig.run(Pass::at(100.0, 100.0, 0.0), nothing.clone());
    rig.run(Pass::at(100.0, 100.0, 0.2), nothing.clone());
    rig.jobs.run_all();
    assert_eq!(rig.run(Pass::at(100.0, 100.0, 0.3), nothing.clone()), (None, false, None));
    // Within 2 px: no new job even after resting.
    rig.run(Pass::at(102.0, 98.0, 0.4), nothing.clone());
    assert_eq!(rig.run(Pass::at(102.0, 98.0, 0.6), nothing.clone()), (None, false, None));
    assert_eq!(rig.started.get(), 1);
    // Three pixels away looks up.
    rig.run(Pass::at(103.5, 100.0, 0.7), nothing.clone());
    rig.run(Pass::at(103.5, 100.0, 0.9), nothing.clone());
    assert_eq!(rig.started.get(), 2);
    // Sixty-four more misses push the first out.
    for i in 0..64 {
        let x = 200.0 + f64::from(i) * 10.0;
        rig.run(Pass::at(x, 300.0, 1.0 + f64::from(i)), nothing.clone());
        rig.run(Pass::at(x, 300.0, 1.2 + f64::from(i)), nothing.clone());
        rig.jobs.run_all();
        rig.run(Pass::at(x, 300.0, 1.3 + f64::from(i)), nothing.clone());
    }
    let before = rig.started.get();
    rig.run(Pass::at(100.0, 100.0, 100.0), nothing.clone());
    rig.run(Pass::at(100.0, 100.0, 100.2), nothing);
    assert_eq!(rig.started.get(), before + 1);
}

// A button down, `enabled` false, the pointer off the canvas, `clear`, and a revision or key change
// each clear the shown result; a job answering after a key change is dropped.
#[test]
fn hover_clears_on_press_tool_change_leave() {
    let found = HoverAnswer::Found(50);
    let shown = |rig: &mut Rig| {
        rig.run(Pass::at(50.0, 5.0, 0.0), found.clone());
        rig.run(Pass::at(50.0, 5.0, 0.2), found.clone());
        rig.jobs.run_all();
        assert_eq!(rig.run(Pass::at(50.0, 5.0, 0.3), found.clone()).0, Some(50));
    };
    let mut rig = Rig::new();
    shown(&mut rig);
    assert_eq!(rig.run(Pass { pressed: true, ..Pass::at(50.0, 5.0, 0.4) }, found.clone()).0, None);
    assert_eq!(rig.lookup.shown(), None);

    let mut rig = Rig::new();
    shown(&mut rig);
    assert_eq!(rig.run(Pass { enabled: false, ..Pass::at(50.0, 5.0, 0.4) }, found.clone()).0, None);

    let mut rig = Rig::new();
    shown(&mut rig);
    assert_eq!(rig.run(Pass { at: None, ..Pass::at(0.0, 0.0, 0.4) }, found.clone()).0, None);

    let mut rig = Rig::new();
    shown(&mut rig);
    rig.lookup.clear();
    assert_eq!(rig.lookup.shown(), None);

    let mut rig = Rig::new();
    shown(&mut rig);
    assert_eq!(rig.run(Pass { revision: 2, ..Pass::at(50.0, 5.0, 0.4) }, found.clone()).0, None);

    // A job running when the key changes is discarded when it answers.
    let mut rig = Rig::new();
    rig.run(Pass::at(50.0, 5.0, 0.0), found.clone());
    rig.run(Pass::at(50.0, 5.0, 0.2), found.clone());
    assert_eq!(rig.started.get(), 1);
    rig.run(Pass { key: 2, ..Pass::at(50.0, 5.0, 0.21) }, found.clone());
    rig.jobs.run_all();
    assert_eq!(rig.run(Pass { key: 2, ..Pass::at(50.0, 5.0, 0.22) }, found).0, None);
}

// `empty_here` gives the hint, no shown result and no job; the hint is gone on the next point.
#[test]
fn hover_empty_layer_gives_the_hint() {
    let mut rig = Rig::new();
    let found = HoverAnswer::Found(50);
    rig.run(Pass { empty_here: true, ..Pass::at(50.0, 5.0, 0.0) }, found.clone());
    let seen = rig.run(Pass { empty_here: true, ..Pass::at(50.0, 5.0, 0.5) }, found.clone());
    assert_eq!(seen, (None, false, Some(HoverHint::EmptySampledLayer)));
    assert_eq!(rig.started.get(), 0);
    assert_eq!(rig.run(Pass::at(60.0, 5.0, 0.6), found).2, None);
}

// A 2000 × 500 mask samples to 667 × 167 (3 units per texel); a 300 × 200 one stays 300 × 200;
// `is_edge` marks the border and the texels beside uncovered ones.
#[test]
fn hover_mask_texture_is_reduced_to_768() {
    let wide = HoverMask::sampled(Rect::new(0.0, 0.0, 2000.0, 500.0), |_, _| 1.0);
    assert_eq!(wide.size(), [667, 167]);
    let small =
        HoverMask::sampled(
            Rect::new(10.0, 10.0, 310.0, 210.0),
            |x, y| if (100.0..200.0).contains(&x) && (50.0..150.0).contains(&y) { 1.0 } else { 0.0 },
        );
    assert_eq!(small.size(), [300, 200]);
    // Texel (x, y) is read at document (10 + x, 10 + y): covered for x in 90..190, y in 40..140.
    assert_eq!(small.coverage(100, 50), 1.0);
    assert_eq!(small.coverage(10, 10), 0.0);
    assert_eq!(small.coverage(300, 0), 0.0);
    assert!(small.is_edge(90, 50) && small.is_edge(189, 100) && small.is_edge(100, 40) && small.is_edge(100, 139));
    assert!(!small.is_edge(100, 100) && !small.is_edge(89, 100));
    assert!(wide.is_edge(0, 0) && wide.is_edge(666, 166) && !wide.is_edge(5, 5));
}

// Two outlines with one salt in two canvases keep two texture entries: the key is made in the
// canvas's own id scope.
#[test]
fn hover_mask_textures_are_keyed_per_canvas_scope() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let masks = [HoverMask::sampled(Rect::new(0.0, 0.0, 8.0, 8.0), |_, _| 1.0), HoverMask::sampled(Rect::new(0.0, 0.0, 6.0, 6.0), |_, _| 1.0)];
    let view = CanvasView::new(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0)), Point::new(4.0, 4.0), 1.0);
    let mut ids = Vec::new();
    for time in [0.0, 0.1] {
        ids.clear();
        let mut output = ctx.run_ui(RawInput { time: Some(time), ..RawInput::default() }, |ui| {
            for (canvas, mask) in ["left", "right"].into_iter().zip(&masks) {
                ui.push_id(canvas, |ui| {
                    ids.push(ui.make_persistent_id(egui::IdSalt::new("same")));
                    HoverOutline::new("same", Some(HoverShape::Mask(mask.clone()))).show(ui, &view);
                });
            }
        });
        output.textures_delta.clear();
    }
    assert_ne!(ids[0], ids[1]);
    for (id, mask) in ids.iter().zip(&masks) {
        let kept = ctx.data(|d| d.get_temp::<(u64, egui::Color32, TextureHandle)>(*id));
        assert_eq!(kept.map(|k| k.0), Some(mask.generation), "each canvas keeps its own mask");
    }
}
