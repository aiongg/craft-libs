//! Temporary frame diagnostics (speed-diag). Enabled by the environment variable `CRAFT_DIAG`.
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Default)]
pub struct Stats {
    pub on: Option<bool>,
    pub depth: u32,
    /// Outermost `run` time, ns (the framework's whole component time).
    pub run_top_ns: u64,
    pub run_depth: u32,
    pub meas_depth: u32,
    /// Outermost measured_extent time, ns.
    pub meas_top_ns: u64,
    pub meas_count: u64,
    pub galley_calls: u64,
    pub galley_ns: u64,
    pub galley_depth: u32,
    /// (type name, sizing) -> (count, inclusive ns)
    pub by_type: HashMap<(&'static str, bool), (u64, u64)>,
    pub runs_real: u64,
    pub runs_sizing: u64,
    pub content_meas: u64,
    pub content_real: u64,
    pub content_by_type: HashMap<(&'static str, bool), u64>,
    pub reused: u64,
    pub calls: HashMap<String, u64>,
}

thread_local! { pub static STATS: RefCell<Stats> = RefCell::new(Stats::default()); }

pub fn on() -> bool {
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        *s.on.get_or_insert_with(|| std::env::var_os("CRAFT_DIAG").is_some())
    })
}

pub fn with<R>(f: impl FnOnce(&mut Stats) -> R) -> R {
    STATS.with(|s| f(&mut s.borrow_mut()))
}

pub struct RunGuard {
    start: Instant,
    name: &'static str,
    sizing: bool,
}
pub fn run_guard(name: &'static str, sizing: bool) -> Option<RunGuard> {
    if !on() {
        return None;
    }
    with(|s| {
        s.run_depth += 1;
        if sizing { s.runs_sizing += 1 } else { s.runs_real += 1 }
    });
    Some(RunGuard { start: Instant::now(), name, sizing })
}
impl Drop for RunGuard {
    fn drop(&mut self) {
        let ns = self.start.elapsed().as_nanos() as u64;
        with(|s| {
            s.run_depth -= 1;
            if s.run_depth == 0 {
                s.run_top_ns += ns;
            }
            let e = s.by_type.entry((self.name, self.sizing)).or_default();
            e.0 += 1;
            e.1 += ns;
        });
    }
}

pub struct MeasGuard(Instant);
pub fn meas_guard() -> Option<MeasGuard> {
    if !on() {
        return None;
    }
    with(|s| {
        s.meas_depth += 1;
        s.meas_count += 1;
    });
    Some(MeasGuard(Instant::now()))
}
impl Drop for MeasGuard {
    fn drop(&mut self) {
        let ns = self.0.elapsed().as_nanos() as u64;
        with(|s| {
            s.meas_depth -= 1;
            if s.meas_depth == 0 {
                s.meas_top_ns += ns;
            }
        });
    }
}

pub struct GalleyGuard(Instant);
pub fn galley_guard() -> Option<GalleyGuard> {
    if !on() {
        return None;
    }
    with(|s| {
        s.galley_calls += 1;
        s.galley_depth += 1;
    });
    Some(GalleyGuard(Instant::now()))
}
impl Drop for GalleyGuard {
    fn drop(&mut self) {
        let ns = self.0.elapsed().as_nanos() as u64;
        with(|s| {
            s.galley_depth -= 1;
            s.galley_ns += ns;
        });
    }
}

pub fn content(name: &'static str, sizing: bool) {
    if !on() {
        return;
    }
    with(|s| {
        if sizing { s.content_meas += 1 } else { s.content_real += 1 }
        *s.content_by_type.entry((name, sizing)).or_default() += 1;
    });
}

/// Prints and resets the frame's stats.
pub fn report(frame_ms: f64, ui_ms: f64, passes: u64, cpu_prev: Option<f32>, tag: &str) {
    if !on() {
        return;
    }
    let s = STATS.with(|s| std::mem::take(&mut *s.borrow_mut()));
    let ms = |ns: u64| ns as f64 / 1e6;
    eprintln!(
        "DIAG {tag} frame_ui={ui_ms:.2}ms interval={frame_ms:.1}ms passes={passes} eframe_cpu_prev={:?} | run_top={:.2}ms runs(real={},sizing={}) meas_top={:.2}ms meas_calls={} content(meas={},real={}) galley_calls={} galley={:.2}ms reused={}",
        cpu_prev.map(|c| (c * 1000.0 * 100.0).round() / 100.0),
        ms(s.run_top_ns), s.runs_real, s.runs_sizing, ms(s.meas_top_ns), s.meas_count, s.content_meas, s.content_real, s.galley_calls, ms(s.galley_ns), s.reused
    );
    let mut types: Vec<_> = s.by_type.iter().collect();
    types.sort_by_key(|(_, v)| std::cmp::Reverse(v.1));
    for ((name, sizing), (count, ns)) in types.iter().take(24) {
        let short = name.rsplit("::").next().unwrap_or(name);
        eprintln!("DIAG   run {short:<28} {} n={count:<5} incl={:.2}ms", if *sizing { "sizing" } else { "real  " }, ms(*ns));
    }
    let mut calls: Vec<_> = s.calls.iter().collect();
    calls.sort_by_key(|(_, v)| std::cmp::Reverse(**v));
    for (k, n) in calls.iter().take(14) {
        eprintln!("DIAG   measure_in {k} n={n}");
    }
    let mut cs: Vec<_> = s.content_by_type.iter().collect();
    cs.sort_by_key(|(_, v)| std::cmp::Reverse(**v));
    for ((name, sizing), n) in cs.iter().take(24) {
        let short = name.rsplit("::").next().unwrap_or(name);
        eprintln!("DIAG   content {short:<28} {} n={n}", if *sizing { "meas" } else { "real" });
    }
}

pub fn call(kind: &str, name: &str, at: &std::panic::Location<'_>) {
    if !on() {
        return;
    }
    let short = name.rsplit("::").next().unwrap_or(name);
    with(|s| *s.calls.entry(format!("{kind} {short} @ {}:{}", at.file(), at.line())).or_default() += 1);
}
