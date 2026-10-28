//! PNG sheets for review: every story, rendered with kittest and wgpu in light and dark, standard
//! and high contrast, at the Compact size, one PNG per component.
//!
//! `CRAFT_UI_SHEETS=<dir> cargo test -p craft-ui --features screen-testing sheets`. Skipped
//! without the variable, and when wgpu finds no adapter. The directory must be outside the
//! repository: sheets are never committed.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use craft_theme::{Contrast, Mode, Size};
use craft_ui::story::{Args, Story, registry};
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf, install, scope};
use egui::{Align, Frame, Layout, Margin, RichText, Ui, vec2};
use egui_kittest::Harness;

const APPEARANCES: [(&str, Mode, Contrast); 4] = [
    ("Light", Mode::Light, Contrast::Standard),
    ("Light, high contrast", Mode::Light, Contrast::High),
    ("Dark", Mode::Dark, Contrast::Standard),
    ("Dark, high contrast", Mode::Dark, Contrast::High),
];
const CELL_WIDTH: f32 = 360.0;
const NAME_WIDTH: f32 = 120.0;

#[test]
fn sheets() {
    let Some(dir) = std::env::var_os("CRAFT_UI_SHEETS") else {
        eprintln!("sheets: CRAFT_UI_SHEETS is not set, nothing rendered");
        return;
    };
    let dir = output_dir(PathBuf::from(dir));
    if let Err(reason) = probe() {
        eprintln!("sheets: skipped, wgpu can't render here: {reason}");
        return;
    }

    let mut components: Vec<(String, Vec<Story>)> = Vec::new();
    for story in registry() {
        let slug = story.id.split('/').take(2).collect::<Vec<_>>().join("-");
        match components.iter_mut().find(|(s, _)| *s == slug) {
            Some((_, stories)) => stories.push(story),
            None => components.push((slug, vec![story])),
        }
    }
    for (slug, stories) in components {
        let path = dir.join(format!("{slug}.png"));
        render(&stories).save(&path).unwrap();
        eprintln!("sheets: wrote {}", path.display());
    }
}

/// Creates `dir` and refuses one inside the repository.
fn output_dir(dir: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.canonicalize().unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    assert!(!dir.starts_with(&repo), "CRAFT_UI_SHEETS ({}) must be outside the repository", dir.display());
    dir
}

/// Renders a one-label harness; the error says why wgpu couldn't.
fn probe() -> Result<(), String> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut harness = Harness::new_ui(|ui| _ = ui.label("probe"));
        harness.run();
        harness.render().map(drop)
    }));
    std::panic::set_hook(hook);
    match result {
        Ok(rendered) => rendered,
        Err(panic) => {
            Err(panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned())).unwrap_or_default())
        }
    }
}

/// One row per story, one column per appearance.
fn render(stories: &[Story]) -> image::RgbaImage {
    let args: Vec<Vec<Args>> = stories.iter().map(|s| APPEARANCES.iter().map(|_| s.defaults()).collect()).collect();
    let stories = stories.to_vec();
    let width = NAME_WIDTH + CELL_WIDTH * APPEARANCES.len() as f32 + 64.0;
    let mut harness = Harness::builder()
        .with_size([width, 4000.0])
        .with_accessibility_check(false)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(move |ui, args: &mut Vec<Vec<Args>>| sheet(ui, &stories, args), args);
    harness.fit_contents();
    harness.run();
    harness.render().unwrap()
}

fn sheet(ui: &mut Ui, stories: &[Story], args: &mut [Vec<Args>]) {
    ui.horizontal_top(|ui| {
        ui.add_sized([NAME_WIDTH, 0.0], egui::Label::new(RichText::new(stories[0].component).strong()));
        for (name, _, _) in APPEARANCES {
            ui.add_sized([CELL_WIDTH, 0.0], egui::Label::new(name));
        }
    });
    for (row, (story, args)) in stories.iter().zip(args).enumerate() {
        ui.separator();
        ui.horizontal_top(|ui| {
            ui.add_sized([NAME_WIDTH, 0.0], egui::Label::new(story.name).wrap());
            for (col, ((_, mode, contrast), args)) in APPEARANCES.into_iter().zip(args.iter_mut()).enumerate() {
                ui.push_id((row, col), |ui| cell(ui, story, mode, contrast, args));
            }
        });
    }
}

fn cell(ui: &mut Ui, story: &Story, mode: Mode, contrast: Contrast, args: &mut Args) {
    let overrides = |o: &mut craft_ui::theme::Overrides| {
        o.mode = Some(mode);
        o.contrast = Some(contrast);
        o.size = Some(Size::Compact);
    };
    scope(ui, overrides, |ui| {
        let tokens = Tokens::of(ui);
        // Stories draw top-down, as in the gallery's canvas, not in the row's horizontal layout.
        ui.allocate_ui_with_layout(vec2(CELL_WIDTH, 0.0), Layout::top_down(Align::Min), |ui| {
            Frame::new().fill(tokens.color.surface.panel.to_egui()).inner_margin(Margin::same(8)).show(ui, |ui| {
                ui.set_width(CELL_WIDTH - 16.0);
                (story.show)(ui, args);
            });
        });
    });
}
