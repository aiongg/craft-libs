//! The checks of the P0 exit: behaviour against the v1 reference, pixels against its renders, the
//! frame cost table, and the panel in a window.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use craft_host::target::Os;
use craft_ui_platform_headless::HeadlessPlatform;
use craft_ui_platform_winit::{WinitPlatform, run};
use craft_ui_rt::{Driver, Platform, Pos2, ReadBack, Rect, Renderer, RgbaImage, SizeU, Vec2, WindowCommand, WindowSpec};
use craft_ui_test_kit::rt::{BenchRow, Rig, Scenario, bench_with, compare_pixels, rig::FRAME};
use serde::{Deserialize, Serialize};

use crate::doc::DocCell;
use crate::fixture::{Doc, PANEL_HEIGHT, PANEL_WIDTH};
use crate::look::pro_dark_tokens;
use crate::mount::mount;
use crate::observe::{Observation, diff, max_bounds_delta, observe_rig};
use crate::panel::LayersPanel;
use crate::scenarios::{CHECK_LAYERS, DOC_LAYERS, SETTLE_FRAMES, bench_scenarios, bench_steps, check_scenarios, scenario, states};

/// Frames discarded before measuring, and frames measured (the experiment's).
pub const WARMUP_FRAMES: usize = 40;
pub const MEASURED_FRAMES: usize = 200;

pub fn panel_size() -> Vec2 {
    Vec2::new(PANEL_WIDTH, PANEL_HEIGHT)
}

/// A fresh document of `layers` layers in a headless window at `scale`, the first frames run (the
/// access tree's activation, the virtual rows' band) and two settle frames after them.
pub fn fresh_rig<R: Renderer>(renderer: R, layers: usize, scale: f32) -> (Rig<R>, DocCell) {
    let platform = HeadlessPlatform::with_window(panel_size(), scale);
    let (runtime, cell) = mount(HeadlessPlatform::MAIN, Doc::new(layers));
    let mut rig = Rig::from_parts(runtime, platform, renderer);
    for _ in 0..5 {
        rig.frame(FRAME);
    }
    (rig, cell)
}

fn into_renderer<R: Renderer>(rig: Rig<R>) -> R {
    let Rig { driver, .. } = rig;
    driver.renderer
}

// ---- behaviour -------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckReference {
    pub variant: String,
    pub layers: usize,
    pub scenarios: Vec<ScenarioRun>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioRun {
    pub name: String,
    pub labels: Vec<String>,
    pub observations: Vec<Observation>,
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs the six behaviour scenarios and diffs each step's observation with the reference's.
/// Returns whether all passed; `observed` receives the run as the reference's own format.
pub fn check<R: Renderer>(renderer: R, reference_dir: &Path, observed: Option<&Path>) -> Result<bool, String> {
    let reference: CheckReference = read_json(&reference_dir.join("check.json"))?;
    let mut renderer = Some(renderer);
    let mut runs = Vec::new();
    let mut all_ok = true;
    for (name, gestures) in check_scenarios() {
        let gs = scenario(name, &gestures, true);
        let (mut rig, cell) = fresh_rig(renderer.take().expect("a renderer"), CHECK_LAYERS, 1.0);
        let mut observations = Vec::new();
        for (i, step) in gs.scenario.steps.iter().enumerate() {
            Scenario::run_step(step, &mut rig)?;
            if gs.ends.contains(&i) {
                observations.push(observe_rig(&rig, cell));
            }
        }
        renderer = Some(into_renderer(rig));
        let want = reference.scenarios.iter().find(|s| s.name == name).ok_or(format!("the reference has no scenario {name}"))?;
        let mut first = None;
        if want.labels != gs.labels {
            first = Some(format!("step labels differ: reference {:?}, run {:?}", want.labels, gs.labels));
        }
        for (i, (a, b)) in want.observations.iter().zip(&observations).enumerate() {
            if first.is_none()
                && let Some(d) = diff(a, b)
            {
                first = Some(format!("step {i} ({:?}): {d}", gs.labels.get(i).cloned().unwrap_or_default()));
            }
        }
        if first.is_none() && want.observations.len() != observations.len() {
            first = Some(format!("step count: reference {} run {}", want.observations.len(), observations.len()));
        }
        match &first {
            None => println!("{name:<16} identical to the reference in {} steps", observations.len()),
            Some(d) => println!("{name:<16} DIFFERS: {d}"),
        }
        all_ok &= first.is_none();
        runs.push(ScenarioRun { name: name.into(), labels: gs.labels, observations });
    }
    if let Some(dir) = observed {
        let _ = std::fs::create_dir_all(dir);
        let run = CheckReference { variant: "rt-runner".into(), layers: CHECK_LAYERS, scenarios: runs };
        let _ = std::fs::write(dir.join("rt-runner-check.json"), serde_json::to_string(&run).unwrap_or_default());
    }
    Ok(all_ok)
}

// ---- pixels ----------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PixelReference {
    shots: Vec<Shot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Shot {
    state: String,
    scale: f32,
    image: String,
    observation: Observation,
    #[serde(default)]
    text_runs: Vec<[f32; 4]>,
}

/// Which comparison a renderer is held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Every pixel equal; at scale 2 a difference of at most 1 in a channel (translation at upload).
    Identical,
    /// Outside text boxes at most 0.5 % of pixels differ, by at most 8 in a channel; every node's
    /// bounds within 0.5 pt; text compares by its box only.
    Tolerance,
}

pub const SCALES: [f32; 3] = [1.0, 1.5, 2.0];

pub fn read_png(path: &Path) -> Result<RgbaImage, String> {
    let file = std::io::BufReader::new(std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?);
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut data).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}: not 8-bit RGBA", path.display()));
    }
    data.truncate(info.buffer_size());
    let mut image = RgbaImage::new(SizeU::new(info.width, info.height));
    image.pixels = data;
    Ok(image)
}

pub fn write_png(path: &Path, img: &RgbaImage) -> Result<(), String> {
    let file = std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| e.to_string())?);
    let mut enc = png::Encoder::new(file, img.size.width, img.size.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().map_err(|e| e.to_string())?.write_image_data(&img.pixels).map_err(|e| e.to_string())
}

/// The window's last frame, read back; GPU reads finish in their own time.
fn read_back<R: Renderer>(rig: &mut Rig<R>) -> Result<RgbaImage, String> {
    let id = rig.driver.renderer.request_read_back(rig.main, None);
    let deadline = web_time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match rig.driver.renderer.poll_read_back(id) {
            ReadBack::Ready(image) => return Ok(image),
            ReadBack::Failed => return Err("the renderer failed the read-back".into()),
            ReadBack::Pending if web_time::Instant::now() > deadline => return Err("the read-back did not finish in 10 s".into()),
            ReadBack::Pending => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
}

/// The text boxes of [`Rule::Tolerance`] in physical pixels, grown by 1 px.
fn text_boxes(obs: &Observation, text_runs: &[[f32; 4]], scale: f32) -> Vec<Rect> {
    let nodes = obs.access.iter().filter(|n| n.role == "label" || n.role == "text_input").map(|n| n.bounds);
    nodes
        .chain(text_runs.iter().copied())
        .map(|b| Rect::from_min_max(Pos2::new(b[0] * scale - 1.0, b[1] * scale - 1.0), Pos2::new(b[2] * scale + 1.0, b[3] * scale + 1.0)))
        .collect()
}

/// Renders every state at every scale and holds it to `rule` against the reference's shot.
pub fn pixels<R: Renderer>(make_renderer: fn() -> R, rule: Rule, reference_dir: &Path, out: Option<&Path>) -> Result<bool, String> {
    let reference: PixelReference = read_json(&reference_dir.join("pixels.json"))?;
    let mut all_ok = true;
    for (state, gestures) in states() {
        for scale in SCALES {
            let labelled: Vec<_> = gestures.iter().map(|g| (state.to_owned(), g.clone())).collect();
            let gs = scenario(state, &labelled, true);
            let (mut rig, cell) = fresh_rig(make_renderer(), DOC_LAYERS, scale);
            for step in &gs.scenario.steps {
                Scenario::run_step(step, &mut rig)?;
            }
            // A shot shows the state, not a moment of a transition into it.
            for _ in 0..SETTLE_FRAMES {
                rig.frame(FRAME);
            }
            let observation = observe_rig(&rig, cell);
            let image = read_back(&mut rig)?;
            drop(rig);
            let Some(want) = reference.shots.iter().find(|s| s.state == state && s.scale == scale) else {
                println!("  {state:<9} x{scale:<4} FAIL  no reference shot");
                all_ok = false;
                continue;
            };
            let reference_image = read_png(&reference_dir.join(&want.image))?;
            let ignore = if rule == Rule::Tolerance { text_boxes(&want.observation, &want.text_runs, scale) } else { Vec::new() };
            let stats = compare_pixels(&reference_image, &image, &ignore);
            let bounds = max_bounds_delta(&want.observation, &observation);
            let pass = match (&stats, rule) {
                (Some(s), Rule::Identical) => s.differing == 0 || (scale == 2.0 && s.max_delta <= 1),
                (Some(s), Rule::Tolerance) => s.percent() <= 0.5 && s.max_delta <= 8 && bounds.is_some_and(|d| d <= 0.5),
                (None, _) => false,
            };
            all_ok &= pass;
            match &stats {
                Some(s) => println!(
                    "  {state:<9} x{scale:<4} {}  differing {:.3}% ({} px)  max delta {}  node bounds delta {}",
                    if pass { "pass" } else { "FAIL" },
                    s.percent(),
                    s.differing,
                    s.max_delta,
                    bounds.map_or("missing node".to_owned(), |d| format!("{d:.2}"))
                ),
                None => println!("  {state:<9} x{scale:<4} FAIL  the sizes differ"),
            }
            if let Some(dir) = out {
                let _ = std::fs::create_dir_all(dir);
                let _ = write_png(&dir.join(format!("rt-{state}-{scale}.png")), &image);
                let _ = write_png(
                    &dir.join(format!("rt-{state}-{scale}-diff.png")),
                    &craft_ui_test_kit::rt::pixels::diff_mask(&reference_image, &image, &ignore),
                );
            }
        }
    }
    Ok(all_ok)
}

// ---- frame cost ------------------------------------------------------------------------------

/// One row per (layers, scale, scenario).
pub struct Row {
    pub layers: usize,
    pub scale: f32,
    pub row: BenchRow,
}

pub fn bench<R: Renderer>(make_renderer: fn() -> R, layers: &[usize], scales: &[f32], only: Option<&[String]>) -> Vec<Row> {
    let mut rows = Vec::new();
    for &n in layers {
        for &scale in scales {
            for sc in bench_scenarios() {
                if only.is_some_and(|o| !o.iter().any(|x| x == sc.name)) {
                    continue;
                }
                let steps = bench_steps(&sc, n);
                // One renderer (and its GPU device) per scenario, kept across the rounds.
                let make = |old: Option<R>| fresh_rig(old.unwrap_or_else(make_renderer), n, scale).0;
                rows.push(Row { layers: n, scale, row: bench_with(make, &steps, WARMUP_FRAMES, MEASURED_FRAMES) });
            }
        }
    }
    rows
}

pub fn bench_table(rows: &[Row]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:>5} {:>5} {:<8} {:>5} {:>6} {:>7} {:>7} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>9}",
        "N",
        "scale",
        "scenario",
        "ran",
        "built",
        "layout",
        "painted",
        "ui ms",
        "prep ms",
        "acc ms",
        "cpu ms",
        "cpu p95",
        "rend ms",
        "tot ms",
        "allocs"
    );
    for r in rows {
        let b = &r.row;
        let _ = writeln!(
            out,
            "{:>5} {:>5} {:<8} {:>5} {:>6} {:>7} {:>7} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>9.0}",
            r.layers,
            r.scale,
            b.scenario,
            b.ran_frames,
            b.built,
            b.laid_out,
            b.painted,
            b.ui_ms.median - b.access_ms.median,
            b.prepare_ms.median,
            b.access_ms.median,
            b.cpu_ms.median,
            b.cpu_ms.p95,
            b.render_ms.median,
            b.total_ms.median,
            b.allocs.median
        );
    }
    out
}

pub fn bench_json(renderer: &str, rows: &[Row]) -> String {
    let stat = |s: &craft_ui_test_kit::rt::stats::Stat| serde_json::json!({"median": s.median, "p95": s.p95});
    let rows: Vec<_> = rows
        .iter()
        .map(|r| {
            let b = &r.row;
            serde_json::json!({
                "layers": r.layers, "scale": r.scale, "scenario": b.scenario, "frames": b.frames, "ran_frames": b.ran_frames,
                "built": b.built, "laid_out": b.laid_out, "painted": b.painted, "prepared": b.prepared,
                "ui_ms": stat(&b.ui_ms), "prepare_ms": stat(&b.prepare_ms), "access_ms": stat(&b.access_ms),
                "cpu_ms": stat(&b.cpu_ms), "render_ms": stat(&b.render_ms), "total_ms": stat(&b.total_ms),
                "allocs": stat(&b.allocs), "bytes": stat(&b.bytes),
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({"renderer": renderer, "rows": rows})).unwrap_or_default()
}

// ---- window ----------------------------------------------------------------------------------

/// The panel in a window until it closes (or `exit_after` seconds).
pub fn window<R: Renderer + 'static>(make: fn() -> R, layers: usize, exit_after: Option<f64>) -> Result<(), String> {
    let started = web_time::Instant::now();
    let mut drawn = 0usize;
    let result = run(
        move |platform: WinitPlatform| async move {
            let mut d = Driver::new(craft_ui_rt::Runtime::new(pro_dark_tokens()), platform, make());
            let doc: DocCell = d.runtime.store_mut().create(Arc::new(Doc::new(layers)));
            let spec = WindowSpec { size: Some(panel_size()), ..WindowSpec::new("Layers") };
            d.open_window(&spec, LayersPanel { doc, os: Os::current() });
            if let Some(secs) = exit_after {
                d.platform.request_frame_at(secs);
            }
            d
        },
        move |d, report| {
            drawn += usize::from(report.windows_drawn > 0);
            for w in d.runtime.take_close_requests() {
                d.platform.window(w, WindowCommand::Close);
            }
            if exit_after.is_some_and(|s| started.elapsed().as_secs_f64() >= s) {
                println!("{drawn} frames drawn in {:.1} s; closing", started.elapsed().as_secs_f64());
                for w in d.platform.surfaces().windows() {
                    d.platform.window(w, WindowCommand::Close);
                }
            }
        },
    );
    result.map_err(|e| e.to_string())
}

pub fn reference_dir(arg: Option<String>) -> PathBuf {
    PathBuf::from(arg.unwrap_or_else(|| "results/reference".into()))
}
