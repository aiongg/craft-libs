//! Stories of the pattern fill editor and dialog (task ST-SH2; task RG2 registers them).
//! Registering them adds `stories()` to craft-draw's stories, the components to `COMPONENTS` and
//! [`PatternStoryLabels`] to `WORDS`.
//!
//! Every story uses fake pattern swatches painted by a closure from the theme's colours; every
//! string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/pattern-fill-editor/editor` | Bitmap mode and two-colour preset mode; an empty library |
//! | `patterns/pattern-fill-dialog/dialog` | Live values and Snap to Origin |

use egui::{Painter, Pos2, Rect, Stroke, Ui};

use super::{PatternColors, PatternFillDialog, PatternFillEditor, PatternFillOptions, PatternSwatch};
use crate::ui::stroke::stories::EventKindLabels;
use craft_theme::Srgba;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, lock, reset_on, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PatternFillEditor", "PatternFillDialog"];

craft_ui::words! {
    /// The captions of the pattern fill stories, English by default.
    #[non_exhaustive]
    pub(crate) struct PatternStoryLabels {
        stripes: "patternStory.stripes" = "Stripes";
        checks: "patternStory.checks" = "Checks";
        dots: "patternStory.dots" = "Dots";
        grid: "patternStory.grid" = "Grid";
        waves: "patternStory.waves" = "Waves";
        bubbles: "patternStory.bubbles" = "Bubbles";
        /// The line under the dialog: `{pattern}`, `{scale}`, `{angle}`, `{x}`, `{y}`.
        values: "patternStory.values" = "{pattern}, {scale} %, {angle}°, origin ({x}, {y})";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/pattern-fill-editor/editor",
            Level::Composites,
            "PatternFillEditor",
            "Editor",
            EditorArgs { mode: ModeArg::Bitmap, library: true, thumbnails: true },
            show_editor
        ),
        typed_story!(
            "patterns/pattern-fill-dialog/dialog",
            Level::Patterns,
            "PatternFillDialog",
            "Dialog",
            DialogArgs { open: true, mode: ModeArg::Bitmap },
            show_dialog
        ),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    /// One picture from the library, with angle, scale and link.
    Bitmap,
    /// A two-colour preset: foreground and background.
    Preset,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct EditorArgs {
    mode: ModeArg,
    /// Offers swatches; off shows the "No patterns" note.
    library: bool,
    /// Paints each tile; off shows the swatch names.
    thumbnails: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DialogArgs {
    /// Shows the dialog.
    open: bool,
    mode: ModeArg,
}

// --- the fake library ---

fn swatches(ui: &Ui) -> Vec<PatternSwatch> {
    let words = PatternStoryLabels::of(ui);
    [words.stripes, words.checks, words.dots, words.grid, words.waves, words.bubbles]
        .into_iter()
        .enumerate()
        .map(|(i, name)| PatternSwatch { id: format!("swatch-{i}"), name })
        .collect()
}

/// Paints a swatch's tile: a motif per id, in the theme's text colour over its surface.
fn paint(tokens: &UiTokens) -> impl Fn(&Painter, Rect, &PatternSwatch) + Send + Sync + 'static {
    let ink = Content::Normal.color(tokens).to_egui();
    let paper = Content::Muted.color(tokens).to_egui().gamma_multiply(0.25);
    let width = tokens.component.line_guide.width;
    move |painter, rect, swatch| {
        let painter = painter.with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, paper);
        let stroke = Stroke::new(width, ink);
        let step = rect.width() / 5.0;
        let kind = swatch.id.rsplit('-').next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(0);
        for i in 0..=10 {
            let t = step * i as f32;
            match kind {
                0 => {
                    let at = rect.min + egui::vec2(t, 0.0);
                    painter.line_segment([at, at + egui::vec2(-rect.height(), rect.height())], stroke);
                }
                1 => {
                    for j in 0..5 {
                        if (i + j) % 2 == 0 && i < 5 {
                            let min = rect.min + egui::vec2(step * i as f32, step * j as f32);
                            painter.rect_filled(Rect::from_min_size(min, egui::vec2(step, step)), 0.0, ink);
                        }
                    }
                }
                2 => {
                    for j in 0..5 {
                        if i < 5 {
                            let c = rect.min + egui::vec2(step * (i as f32 + 0.5), step * (j as f32 + 0.5));
                            painter.circle_filled(c, step / 4.0, ink);
                        }
                    }
                }
                3 => {
                    painter.line_segment([rect.min + egui::vec2(t, 0.0), rect.min + egui::vec2(t, rect.height())], stroke);
                    painter.line_segment([rect.min + egui::vec2(0.0, t), rect.min + egui::vec2(rect.width(), t)], stroke);
                }
                4 => {
                    let y = rect.min.y + step * (i as f32 / 2.0);
                    let points: Vec<Pos2> = (0..=8)
                        .map(|k| {
                            let x = rect.min.x + rect.width() * k as f32 / 8.0;
                            Pos2::new(x, y + if k % 2 == 0 { -step / 4.0 } else { step / 4.0 })
                        })
                        .collect();
                    painter.add(egui::Shape::line(points, stroke));
                }
                _ => {
                    if i < 5 {
                        for j in 0..5 {
                            let c = rect.min + egui::vec2(step * (i as f32 + 0.5), step * (j as f32 + 0.5));
                            painter.circle_stroke(c, step / 3.0 + step / 8.0 * ((i + j) % 2) as f32, stroke);
                        }
                    }
                }
            }
        }
    }
}

/// The options a mode starts from; the preset's colours are the theme's text and surface colours.
fn start(ui: &Ui, mode: ModeArg, first: &str) -> PatternFillOptions {
    match mode {
        ModeArg::Bitmap => PatternFillOptions::bitmap(first),
        ModeArg::Preset => {
            let tokens = Tokens::of(ui);
            let colors =
                PatternColors { foreground: Srgba::from(Content::Normal.color(&tokens)), background: Srgba::from(tokens.color.accent.solid) };
            PatternFillOptions::preset(first, colors)
        }
    }
}

// --- editor ---

#[derive(Clone)]
struct EditorKept {
    options: PatternFillOptions,
    event: Option<String>,
}

fn show_editor(ui: &mut Ui, args: &mut EditorArgs) {
    let words = StoryLabels::of(ui);
    let library = if args.library { swatches(ui) } else { Vec::new() };
    let first = library.first().map_or_else(String::new, |s| s.id.clone());
    reset_on(ui, "editor", &args.mode);
    let kept = shared(ui, "editor", || EditorKept { options: start(ui, args.mode, &first), event: None });
    let mut kept = lock(&kept);
    let tokens = Tokens::of(ui);
    let mut editor = PatternFillEditor::new("pattern-story", &kept.options, &library);
    if args.thumbnails {
        editor = editor.thumbnails(paint(&tokens));
    }
    if let Some(event) = editor.show(ui).event {
        kept.event = Some(EventKindLabels::of(ui).kind(&event));
        if let Some(value) = event.value() {
            kept.options = value.clone();
        }
    }
    let note = match &kept.event {
        Some(event) => craft_i18n::fmt(&words.last_event, &[("event", event)]),
        None => words.no_event.clone(),
    };
    ui.add(Text::new(note).content(Content::Muted));
}

// --- dialog ---

fn describe(own: &PatternStoryLabels, numbers: &LocaleNumbers, options: &PatternFillOptions) -> String {
    let number = |v: f64| numbers.format(v, 0..=2);
    craft_i18n::fmt(
        &own.values,
        &[
            ("pattern", &options.pattern),
            ("scale", &number(options.scale)),
            ("angle", &number(options.angle)),
            ("x", &number(options.phase[0])),
            ("y", &number(options.phase[1])),
        ],
    )
}

/// The dialog on a `DialogHost`; the line under it shows the values the fields hold now, the
/// app's live preview input.
fn show_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    let own = PatternStoryLabels::of(ui);
    let mode = args.mode;
    let mut live = None;
    {
        let own = own.clone();
        let numbers = LocaleNumbers::of(ui);
        let (library, options, painter) = (swatches(ui), start_for(ui, mode), paint(&Tokens::of(ui)));
        let describe_ok = move |options: PatternFillOptions| describe(&own, &numbers, &options);
        craft_ui::story::drive(
            ui,
            "pattern-fill-dialog",
            &mut args.open,
            mode,
            move || PatternFillDialog::new(options, library).thumbnails(painter),
            |dialog| live = Some(dialog.current().clone()),
            describe_ok,
        );
    }
    if let Some(options) = live {
        ui.add(Text::new(describe(&own, &LocaleNumbers::of(ui), &options)).content(Content::Muted));
    }
}

fn start_for(ui: &Ui, mode: ModeArg) -> PatternFillOptions {
    let first = swatches(ui).first().map_or_else(String::new, |s| s.id.clone());
    start(ui, mode, &first)
}
