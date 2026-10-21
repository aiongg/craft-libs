//! The minimum widget height for the UI language's script face.
//!
//! An app's theme sets `interact_size` for rows of its own face. When the UI language's script face
//! has taller rows, widgets need that much more height; the theme's own height and padding stay as
//! they are.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use egui::epaint::text::{Fonts, TextOptions};
use egui::{FontData, FontDefinitions, FontFamily, FontId, Style, TextStyle, Theme};

const BASE: &str = "craft-ui base face";
const SCRIPT: &str = "craft-ui script face";

/// The app's face and the UI language's script face, each as its own family.
#[derive(Clone)]
pub(super) struct RowFaces(FontDefinitions);

impl RowFaces {
    pub(super) fn new(base: Arc<FontData>, script: Arc<FontData>) -> Self {
        let mut definitions = FontDefinitions::empty();
        for (name, data) in [(BASE, base), (SCRIPT, script)] {
            definitions.font_data.insert(name.to_owned(), data);
            definitions.families.insert(FontFamily::Name(name.into()), vec![name.to_owned()]);
        }
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            definitions.families.entry(family).or_default();
        }
        Self(definitions)
    }
}

/// How much taller a row of the script face is than a row of the app's face, at `style`'s body and
/// button sizes; `0` when it isn't taller.
pub(super) fn extra_height(faces: Option<&RowFaces>, style: &Style) -> f32 {
    let Some(RowFaces(definitions)) = faces else { return 0.0 };
    let mut fonts = Fonts::new(TextOptions::default(), definitions.clone());
    let mut view = fonts.with_pixels_per_point(1.0);
    let mut row = |size: f32, face: &str| view.row_height(&FontId::new(size, FontFamily::Name(face.into())));
    let extra = text_sizes(style).into_iter().map(|size| row(size, SCRIPT) - row(size, BASE)).fold(0.0_f32, f32::max);
    if extra.is_finite() { extra } else { 0.0 }
}

fn text_sizes(style: &Style) -> Vec<f32> {
    [TextStyle::Body, TextStyle::Button].iter().filter_map(|s| style.text_styles.get(s)).map(|font| font.size).collect()
}

/// A context's fit, kept in its data.
#[derive(Default)]
struct Fitted {
    faces: Option<RowFaces>,
    /// [`extra_height`] by text sizes, for the faces installed.
    measured: Vec<(Vec<f32>, f32)>,
    /// Per theme: the style's own height and the height it was set to.
    applied: [Option<(f32, f32)>; 2],
}

type Shared = Arc<Mutex<Fitted>>;

fn id() -> egui::Id {
    egui::Id::unique("craft_ui::fonts::fitted")
}

fn lock(shared: &Shared) -> MutexGuard<'_, Fitted> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Fit `ctx`'s styles to `faces` now, and again at the start of every pass after the app sets a
/// style with another height or text size.
pub(super) fn install(ctx: &egui::Context, faces: Option<RowFaces>) {
    let shared: Shared = ctx.data_mut(|d| Arc::clone(d.get_temp_mut_or_default::<Shared>(id())));
    {
        let mut fitted = lock(&shared);
        fitted.faces = faces;
        fitted.measured.clear();
    }
    ctx.add_plugin(Refit);
    refit(ctx, &shared);
}

fn refit(ctx: &egui::Context, shared: &Shared) {
    let mut fitted = lock(shared);
    for (i, theme) in [Theme::Dark, Theme::Light].into_iter().enumerate() {
        let style = ctx.style_of(theme);
        let height = style.spacing.interact_size.y;
        // Any other height than the one set last is the app's, e.g. from a theme applied since.
        let base = match fitted.applied[i] {
            Some((base, set)) if set == height => base,
            _ => height,
        };
        let sizes = text_sizes(&style);
        let extra = match fitted.measured.iter().find(|(s, _)| *s == sizes) {
            Some((_, extra)) => *extra,
            None => {
                let extra = extra_height(fitted.faces.as_ref(), &style);
                // Bounded, for an app that changes text sizes often.
                if fitted.measured.len() >= 4 {
                    fitted.measured.remove(0);
                }
                fitted.measured.push((sizes, extra));
                extra
            }
        };
        let target = base + extra;
        if target != height {
            ctx.style_mut_of(theme, |style| style.spacing.interact_size.y = target);
        }
        fitted.applied[i] = Some((base, target));
    }
}

/// Refits a context's styles at the start of each pass.
struct Refit;

impl egui::plugin::Plugin for Refit {
    fn debug_name(&self) -> &'static str {
        "craft_ui::fonts::Refit"
    }

    fn on_begin_pass(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let shared: Option<Shared> = ctx.data(|d| d.get_temp(id()));
        if let Some(shared) = shared {
            refit(&ctx, &shared);
        }
    }
}
