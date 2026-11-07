//! Records the behaviour of every legacy story as text: its AccessKit tree without bounds, then
//! the changes a generic scripted interaction makes to it.
//!
//! `parity [--out DIR] [FILTER...]` writes `<DIR>/<level>/<component>--<story>.txt` for each
//! story whose id contains one of the filters (all stories without filters), and
//! `<DIR>/SKIPPED.md` for the stories that panic. `DIR` defaults to `capture` at the repository
//! root. The faces come from `CRAFT_FONTS_DIR`.
//!
//! The text format is the contract a replay on the new stories compares against: see [`record`].

mod dump;
mod script;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Mutex;

use craft_theme::SystemAppearance;
use craft_ui::story::{Args, Story};
use craft_ui::testing::symbols;
use craft_ui::theme::install;
use craft_ui_gallery::{Appearance, Gallery, faces};
use egui_kittest::Harness;

/// The frame length every step runs at, so animations advance by the same amount on every run.
const STEP_DT: f32 = 1.0 / 60.0;

/// The size of the surface a story is shown on, in points.
const SURFACE: egui::Vec2 = egui::Vec2::new(640.0, 1200.0);

/// Records one story: the tree at rest, then the scripted interaction.
fn record(story: &Story) -> String {
    let appearance = Appearance::default();
    let system = SystemAppearance::default();
    let base = appearance.base(system);
    let setup = appearance.clone();
    let builder =
        Harness::builder().allow_missing_glyphs().with_size(SURFACE).with_step_dt(STEP_DT).with_accessibility_check(false).with_context_setup(
            move |ctx| {
                symbols::install(ctx);
                setup.configure(ctx, None, false, system);
                if faces::available() {
                    install_fallback_glyphs(ctx);
                }
            },
        );
    let story = *story;
    let mut harness = builder.build_ui_state(
        move |ui, args: &mut Args| {
            install(ui.ctx(), &base);
            ui.set_min_width(ui.available_width());
            ui.push_id(story.id, |ui| (story.show)(ui, args));
        },
        story.defaults(),
    );
    script::settle(&mut harness);
    let mut out = String::new();
    let _ = writeln!(out, "story {}", story.id);
    let _ = writeln!(out, "args");
    out.push_str(&dump::args(harness.state()));
    let _ = writeln!(out, "tree");
    out.push_str(&dump::tree(&harness).text());
    script::run(&mut harness, &mut out);
    out
}

/// egui's own faces behind the suite's: a font set with only Latin and symbol faces lacks glyphs
/// some stories draw, and the harness refuses to draw a glyph no installed face has.
fn install_fallback_glyphs(ctx: &egui::Context) {
    for (name, data) in egui::FontDefinitions::default().font_data {
        let families = [egui::FontFamily::Proportional, egui::FontFamily::Monospace]
            .map(|family| egui::epaint::text::InsertFontFamily { family, priority: egui::epaint::text::FontPriority::Lowest })
            .into();
        ctx.add_font(egui::epaint::text::FontInsert::new(&name, (*data).clone(), families));
    }
}

/// `<level>/<component>--<story>.txt` for the story's id.
fn file_of(id: &str) -> PathBuf {
    let mut parts = id.split('/');
    let level = parts.next().unwrap_or(id);
    let rest: Vec<&str> = parts.collect();
    PathBuf::from(level).join(format!("{}.txt", if rest.is_empty() { id.to_owned() } else { rest.join("--") }))
}

fn main() -> ExitCode {
    let mut out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../capture");
    let mut filters: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--out" {
            let Some(dir) = args.next() else {
                eprintln!("--out needs a directory");
                return ExitCode::FAILURE;
            };
            out_dir = PathBuf::from(dir);
        } else {
            filters.push(arg);
        }
    }
    let stories: Vec<Story> =
        Gallery::new(None).stories().iter().filter(|s| filters.is_empty() || filters.iter().any(|f| s.id.contains(f.as_str()))).copied().collect();

    // A panic's message is the reason a story is skipped; the default hook would print every one.
    static LAST_PANIC: Mutex<String> = Mutex::new(String::new());
    panic::set_hook(Box::new(|info| {
        let message = match info.payload().downcast_ref::<&str>() {
            Some(s) => (*s).to_owned(),
            None => info.payload().downcast_ref::<String>().cloned().unwrap_or_default(),
        };
        let place = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        *LAST_PANIC.lock().unwrap_or_else(|e| e.into_inner()) = format!("{}{place}", message.lines().next().unwrap_or(""));
    }));

    let mut written = 0usize;
    let mut skipped: BTreeMap<&str, String> = BTreeMap::new();
    for story in &stories {
        match panic::catch_unwind(AssertUnwindSafe(|| record(story))) {
            Ok(text) => {
                let path = out_dir.join(file_of(story.id));
                if let Some(dir) = path.parent()
                    && let Err(e) = std::fs::create_dir_all(dir)
                {
                    eprintln!("{}: {e}", dir.display());
                    return ExitCode::FAILURE;
                }
                if let Err(e) = std::fs::write(&path, text) {
                    eprintln!("{}: {e}", path.display());
                    return ExitCode::FAILURE;
                }
                written += 1;
            }
            Err(_) => {
                let reason = LAST_PANIC.lock().unwrap_or_else(|e| e.into_inner()).clone();
                skipped.insert(story.id, reason);
            }
        }
    }
    if filters.is_empty() {
        let mut text = String::new();
        for (id, reason) in &skipped {
            let _ = writeln!(text, "- `{id}`: panicked: {}", reason.replace('\n', " "));
        }
        let path = out_dir.join("SKIPPED.md");
        if let Err(e) = std::fs::create_dir_all(&out_dir).and_then(|()| std::fs::write(&path, text)) {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    } else {
        for (id, reason) in &skipped {
            eprintln!("skipped {id}: {reason}");
        }
    }
    println!("{written} captured, {} skipped", skipped.len());
    ExitCode::SUCCESS
}
