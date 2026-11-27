//! The frame cost table: one row per scenario with how many frames ran, how many nodes each phase
//! touched and what the frames cost. Counts are the result that does not move between machines;
//! times and allocations are for comparing runs on one machine.

use std::fmt::Write;

use craft_ui_rt::Renderer;

use super::alloc::{self, measure};
use super::rig::Rig;
use super::scenario::Scenario;
use super::stats::{Stat, summarize};

/// Frames discarded before measuring.
pub const WARMUP_FRAMES: usize = 40;
/// Frames measured.
pub const MEASURED_FRAMES: usize = 200;

/// One scenario's measured frames.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchRow {
    pub scenario: String,
    /// Frames measured (a frame is one step, run or not).
    pub frames: usize,
    /// Frames that ran: something was dirty and a surface was there.
    pub ran_frames: usize,
    /// Nodes built, laid out and painted, summed over the measured frames.
    pub built: u64,
    pub laid_out: u64,
    pub painted: u64,
    /// Display lists in the frames rendered, and those converted rather than reused.
    pub draws: u64,
    pub prepared: u64,
    /// Build, layout, paint and access time per frame, milliseconds.
    pub ui_ms: Stat,
    /// Render time per frame, milliseconds.
    pub render_ms: Stat,
    /// Renderer preparation (tessellation, picture recording) per frame, milliseconds.
    pub prepare_ms: Stat,
    /// Access tree time per frame, milliseconds.
    pub access_ms: Stat,
    /// The CPU cost the experiment's tables compare: the driver step without the renderer's work
    /// after preparing (`total - render + prepare`), milliseconds.
    pub cpu_ms: Stat,
    pub total_ms: Stat,
    /// False where no counting allocator is installed; `allocs` and `bytes` are then zeros.
    pub alloc_counted: bool,
    pub allocs: Stat,
    pub bytes: Stat,
}

/// Runs `scenario` on rigs from `make` for `warmup + measured` frames: its steps in order, a fresh
/// rig whenever it ends. Each step's input is injected outside the timed section; the timed frame
/// is the clock advance and the driver step.
pub fn bench<R: Renderer>(mut make: impl FnMut() -> Rig<R>, scenario: &Scenario, warmup: usize, measured: usize) -> BenchRow {
    bench_with(|_| make(), scenario, warmup, measured)
}

/// [`bench`] where `make` gets the renderer of the rig it replaces (`None` the first time), so a
/// GPU device or a font atlas survives from round to round.
pub fn bench_with<R: Renderer>(mut make: impl FnMut(Option<R>) -> Rig<R>, scenario: &Scenario, warmup: usize, measured: usize) -> BenchRow {
    let mut rig = make(None);
    let mut next = 0;
    let (mut ui, mut render, mut total, mut allocs, mut bytes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut prepare, mut access, mut cpu) = (Vec::new(), Vec::new(), Vec::new());
    let mut row = BenchRow {
        scenario: scenario.name.clone(),
        frames: 0,
        ran_frames: 0,
        built: 0,
        laid_out: 0,
        painted: 0,
        draws: 0,
        prepared: 0,
        ui_ms: Stat::default(),
        render_ms: Stat::default(),
        prepare_ms: Stat::default(),
        access_ms: Stat::default(),
        cpu_ms: Stat::default(),
        total_ms: Stat::default(),
        alloc_counted: false,
        allocs: Stat::default(),
        bytes: Stat::default(),
    };
    let mut frame_no = 0;
    while row.frames < measured {
        let dt = match scenario.steps.get(next) {
            Some(step) => {
                step.inject(&mut rig);
                step.dt
            }
            None if scenario.steps.is_empty() => super::rig::FRAME,
            None => {
                let Rig { driver, .. } = rig;
                rig = make(Some(driver.renderer));
                next = 0;
                continue;
            }
        };
        next += 1;
        let (report, cost) = measure(|| rig.frame(dt));
        if frame_no >= warmup {
            let f = report.frame;
            row.frames += 1;
            row.ran_frames += usize::from(report.ran);
            row.built += u64::from(f.built);
            row.laid_out += u64::from(f.laid_out);
            row.painted += u64::from(f.painted);
            row.draws += u64::from(report.render.draws);
            row.prepared += u64::from(report.render.prepared);
            let ui_ms = f.build_ms + f.layout_ms + f.paint_ms + f.access_ms;
            ui.push(ui_ms);
            render.push(report.render_ms);
            prepare.push(report.render.prepare_ms);
            access.push(f.access_ms);
            cpu.push((cost.ms - report.render_ms + report.render.prepare_ms).max(0.0));
            total.push(cost.ms);
            allocs.push(cost.allocs as f64);
            bytes.push(cost.bytes as f64);
        }
        frame_no += 1;
    }
    row.ui_ms = summarize(&ui);
    row.render_ms = summarize(&render);
    row.prepare_ms = summarize(&prepare);
    row.access_ms = summarize(&access);
    row.cpu_ms = summarize(&cpu);
    row.total_ms = summarize(&total);
    row.alloc_counted = alloc::installed();
    row.allocs = summarize(&allocs);
    row.bytes = summarize(&bytes);
    row
}

/// A text table, one line per row.
pub fn bench_table(rows: &[BenchRow]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<28} {:>6} {:>5} {:>7} {:>7} {:>7} {:>7} {:>9} {:>9} {:>9}",
        "scenario", "frames", "ran", "built", "layout", "painted", "prep", "ui ms", "rend ms", "p95 tot"
    );
    for r in rows {
        let _ = writeln!(
            out,
            "{:<28} {:>6} {:>5} {:>7} {:>7} {:>7} {:>7} {:>9.3} {:>9.3} {:>9.3}",
            r.scenario, r.frames, r.ran_frames, r.built, r.laid_out, r.painted, r.prepared, r.ui_ms.median, r.render_ms.median, r.total_ms.p95
        );
    }
    out
}
