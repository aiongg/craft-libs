//! Stories of the mask views (task ST-SE2): not registered. Registering them (task RG2) adds
//! `mask_view::stories` to the crate's `register!` list and `MaskViewControls` to `COMPONENTS`, and
//! regenerates the registry dump. The story has no words of its own: its captions are
//! `MaskViewLabels`.
//!
//! The story composites the recipe itself over `MockImage` with `MockImage::subject` as the soft
//! mask (the house), so every view shows without an engine ([`Composite`]). Every string goes
//! through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/mask-view/modes` | The seven views over the picture; the opacity kept per view; Show Edge; Show Original; hold X |

use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, Surface};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Ui};

use super::{MaskCoverage, MaskViewControls, MaskViewKeys, MaskViewLabels, MaskViewMode, MaskViewRecipe, MaskViewState, VeilMask};
use crate::ui::mock_image::{MockImage, MockSubject};
use crate::ui::selection::{SelectionOverlay, SelectionOverlayLabels};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["MaskViewControls"];

/// The story.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("patterns/mask-view/modes", Level::Patterns, "MaskViewControls", "Modes", NoArgs {}, show_modes).on(Surface::Canvas)]
}

/// The size of the large view and of each of the seven small ones (the picture drawn at that
/// size; the composite computes every pixel, so they are smaller than the default picture).
const LARGE: [u32; 2] = [960, 640];
const SMALL: [u32; 2] = [240, 160];
/// Show Edge's band: its half width as a fraction of the picture's width.
const BAND: f64 = 0.008;

/// The view the engine would build from a [`MaskViewRecipe`], drawn on the spot: the layer (the
/// picture) with its mask raised to the recipe's floor, then each veil over it.
pub(crate) struct Composite {
    pub(crate) image: MockImage,
    pub(crate) recipe: MaskViewRecipe,
    pub(crate) revision: u64,
}

impl Composite {
    /// The coverage the recipe shows at `(x, y)`.
    fn coverage(&self, x: f64, y: f64) -> f32 {
        match self.recipe.coverage {
            MaskCoverage::Refined => self.image.subject(x, y),
            // The selection before refinement: the hard outline.
            MaskCoverage::Original => f32::from(self.image.subject_distance(x, y) < 0.0),
            // The band either side of the edge.
            MaskCoverage::EdgeBand => {
                let reach = BAND * f64::from(self.image.size()[0]);
                (1.0 - (self.image.subject_distance(x, y).abs() / reach).clamp(0.0, 1.0)) as f32
            }
        }
    }

    /// The composite at `(x, y)`: straight sRGBA.
    fn at(&self, x: f64, y: f64) -> Color32 {
        let [r, g, b, _] = self.image.pixel(x, y);
        if self.recipe.hidden {
            return Color32::from_rgb(r, g, b);
        }
        let c = self.coverage(x, y);
        let mut colour = [f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0];
        let mut alpha = self.recipe.layer_floor.map_or(1.0, |floor| c + (1.0 - c) * floor);
        for veil in &self.recipe.veils {
            let mask = match veil.mask {
                VeilMask::Everywhere => 1.0,
                VeilMask::Coverage => c,
                VeilMask::Inverse => 1.0 - c,
            };
            let on = veil.opacity * mask * veil.color.a;
            let out = on + alpha * (1.0 - on);
            if out > 0.0 {
                let over = [veil.color.r, veil.color.g, veil.color.b];
                for (channel, over) in colour.iter_mut().zip(over) {
                    *channel = (over * on + *channel * alpha * (1.0 - on)) / out;
                }
            }
            alpha = out;
        }
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(byte(colour[0]), byte(colour[1]), byte(colour[2]), byte(alpha))
    }
}

impl RasterSource for Composite {
    fn size(&self) -> [u32; 2] {
        self.image.size()
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [columns, rows] = out.size;
        let factor = f64::from(factor.max(1));
        for row in 0..rows {
            let y = f64::from(region.y0) + (row as f64 + 0.5) * factor;
            for column in 0..columns {
                out.pixels[row * columns + column] = self.at(f64::from(region.x0) + (column as f64 + 0.5) * factor, y);
            }
        }
    }
}

/// The revision of a composite kept under `id`: it counts up whenever the recipe changes.
fn revision_of(ui: &Ui, id: impl egui::AsIdSalt, recipe: &MaskViewRecipe) -> u64 {
    let key = ui.make_persistent_id(id);
    let kept = ui.data(|d| d.get_temp::<(MaskViewRecipe, u64)>(key));
    match kept {
        Some((kept, revision)) if kept == *recipe => revision,
        other => {
            let revision = other.map_or(1, |(_, revision)| revision + 1);
            ui.data_mut(|d| d.insert_temp(key, (recipe.clone(), revision)));
            revision
        }
    }
}

/// A canvas of the recipe's view over a picture of `size`, with the ants when the recipe has them.
fn view(ui: &mut Ui, salt: &'static str, size: [u32; 2], recipe: MaskViewRecipe, width: f32) {
    let image = MockImage::new(size);
    let revision = revision_of(ui, ("mask-view-story", salt), &recipe);
    let ants = recipe.ants && !recipe.hidden;
    let composite = Composite { image, recipe, revision };
    let (response, view) = image.canvas_of(ui, ("mask-view-story", salt), width, None, None, &composite);
    SelectionOverlay::new(("mask-view-story", salt), &MockSubject(image))
        .labels(SelectionOverlayLabels::of(ui))
        .visible(ants)
        .show(ui, &response, &view);
}

/// The View Mode controls and keys drive one large view; the seven views show small below, each
/// at its own kept opacity.
fn show_modes(ui: &mut Ui, _: &mut NoArgs) {
    let labels = MaskViewLabels::of(ui);
    let id = ui.make_persistent_id("mask-view-story-state");
    let mut state = ui.data(|d| d.get_temp::<MaskViewState>(id)).unwrap_or_default();
    MaskViewKeys::read(ui.ctx(), &mut state);
    MaskViewControls::new("mask-view-story", &mut state).labels(labels.clone()).show(ui);
    let width = ui.available_width();
    view(ui, "large", LARGE, state.recipe(), width);

    for modes in MaskViewMode::ALL.chunks(4) {
        ui.columns(4, |columns| {
            for (column, &mode) in columns.iter_mut().zip(modes) {
                column.add(Text::new(mode.label(&labels)).content(Content::Muted));
                let each = MaskViewState { mode, hide_view: false, ..state.clone() };
                let width = column.available_width();
                view(column, mode.id(), SMALL, each.recipe(), width);
            }
        });
    }
    ui.data_mut(|d| d.insert_temp(id, state));
}
