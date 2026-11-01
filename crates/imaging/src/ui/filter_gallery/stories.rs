//! Stories of the Filter Gallery workspace (task ST-WS1): not registered. Registering them (task
//! RG2) adds `filter_gallery::stories` to the crate's `register!` list, `FilterGalleryWorkspace`
//! to `COMPONENTS` and [`GalleryStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The stories use `MockGallery`: a six-filter catalog in two folders with `Param` lists (a Range,
//! a Choice, a Bool, a colour), thumbnails from `FakeThumbSource`, and a preview that is
//! `MockImage` tinted per visible effect (no engine). The workspace covers the gallery window, as
//! it does in an app; the `open` arg (or the Overview tile's button) shows it. Every string goes
//! through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/filter-gallery/default` | The Artistic folder open and Sketch closed, the selected thumbnail, the settings, a three-layer stack with one eye off |
//! | `patterns/filter-gallery/rendering` | The zoom bar with a progress bar (arg), then the note |

use std::f64::consts::TAU;
use std::sync::{Arc, Mutex};

use craft_ui::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::full_window::PreviewTicket;
use craft_ui::dialog::{Outcome, Param, ParamKind};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, claim, dialog_closed, dialog_shown};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Ui};

use super::{
    EffectChange, FilterCatalog, FilterCategory, FilterGalleryEvent, FilterGalleryOpen, FilterGallerySession, FilterGalleryWorkspace, GalleryFilter,
    GalleryThumb,
};
use crate::ui::mock_image::MockImage;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["FilterGalleryWorkspace"];

craft_ui::words! {
    /// The names and captions of the Filter Gallery stories, English by default.
    #[non_exhaustive]
    pub(crate) struct GalleryStoryLabels {
        artistic: "galleryStory.artistic" = "Artistic";
        sketch: "galleryStory.sketch" = "Sketch";
        cutout: "galleryStory.cutout" = "Cutout";
        sponge: "galleryStory.sponge" = "Sponge";
        neon_glow: "galleryStory.neonGlow" = "Neon Glow";
        chalk: "galleryStory.chalk" = "Chalk & Charcoal";
        photocopy: "galleryStory.photocopy" = "Photocopy";
        note_paper: "galleryStory.notePaper" = "Note Paper";
        levels: "galleryStory.levels" = "Number of Levels";
        simplicity: "galleryStory.simplicity" = "Edge Simplicity";
        fidelity: "galleryStory.fidelity" = "Edge Fidelity";
        low: "galleryStory.low" = "Low";
        medium: "galleryStory.medium" = "Medium";
        high: "galleryStory.high" = "High";
        brush_size: "galleryStory.brushSize" = "Brush Size";
        smooth: "galleryStory.smooth" = "Smooth edges";
        glow_size: "galleryStory.glowSize" = "Glow Size";
        glow_color: "galleryStory.glowColor" = "Glow Colour";
        detail: "galleryStory.detail" = "Detail";
        darkness: "galleryStory.darkness" = "Darkness";
        image_balance: "galleryStory.imageBalance" = "Image Balance";
        preview_ms: "galleryStory.previewMs" = "Preview {n} ms";
    }
}

/// The time the note reports, in milliseconds.
const NOTE_MS: f64 = 120.0;
/// The share of the tint a filter adds to the preview (0 leaves the picture, 1 multiplies it).
const TINT_STRENGTH: f64 = 0.25;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/filter-gallery/default",
            Level::Patterns,
            "FilterGalleryWorkspace",
            "Default",
            DefaultArgs { open: true },
            show_default
        )
        .on(Surface::Own),
        typed_story!(
            "patterns/filter-gallery/rendering",
            Level::Patterns,
            "FilterGalleryWorkspace",
            "Rendering",
            RenderingArgs { open: true, state: RenderState::Rendering, progress: 40.0 },
            show_rendering
        )
        .on(Surface::Own),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DefaultArgs {
    /// Whether the workspace is open.
    open: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RenderState {
    Rendering,
    #[choice(name = "Done, with the note")]
    Done,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct RenderingArgs {
    /// Whether the workspace is open.
    open: bool,
    /// Whether a preview renders or has finished.
    state: RenderState,
    /// How far the render is, in percent.
    #[arg(range = 0.0..=100.0)]
    progress: f64,
}

// --- the fake app ---

/// The app's catalog: six filters in two folders.
struct MockGallery {
    labels: GalleryStoryLabels,
}

impl MockGallery {
    const ARTISTIC: [&'static str; 3] = ["cutout", "sponge", "neon"];
    const SKETCH: [&'static str; 3] = ["chalk", "photocopy", "paper"];

    fn range(key: &str, label: &str, min: f64, max: f64, default: f64) -> Param {
        Param::new(key, label, ParamKind::Range { min, max, default, scale: Default::default() })
    }
}

impl FilterCatalog for MockGallery {
    fn categories(&self) -> Vec<FilterCategory> {
        let keys = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
        vec![
            FilterCategory { key: "artistic".to_owned(), label: self.labels.artistic.clone(), filters: keys(&Self::ARTISTIC) },
            FilterCategory { key: "sketch".to_owned(), label: self.labels.sketch.clone(), filters: keys(&Self::SKETCH) },
        ]
    }

    fn filter(&self, key: &str) -> Option<GalleryFilter> {
        let l = &self.labels;
        let (label, category, params, uses_colors) = match key {
            "cutout" => {
                let options = vec![("1".to_owned(), l.low.clone()), ("2".to_owned(), l.medium.clone()), ("3".to_owned(), l.high.clone())];
                let fidelity = Param::new("fidelity", l.fidelity.as_str(), ParamKind::Choice { options, default: Some("2".to_owned()) });
                (
                    &l.cutout,
                    "artistic",
                    vec![Self::range("levels", &l.levels, 2.0, 8.0, 6.0), Self::range("simplicity", &l.simplicity, 0.0, 10.0, 4.0), fidelity],
                    false,
                )
            }
            "sponge" => {
                let smooth = Param::new("smooth", l.smooth.as_str(), ParamKind::Bool { default: true });
                (
                    &l.sponge,
                    "artistic",
                    vec![Self::range("size", &l.brush_size, 0.0, 10.0, 2.0), Self::range("detail", &l.detail, 1.0, 25.0, 5.0), smooth],
                    false,
                )
            }
            "neon" => {
                let glow = Param::new("glow", l.glow_color.as_str(), ParamKind::Color { default: Some([0.9, 0.3, 0.6]) });
                (&l.neon_glow, "artistic", vec![Self::range("size", &l.glow_size, -24.0, 24.0, 5.0), glow], true)
            }
            "chalk" => (
                &l.chalk,
                "sketch",
                vec![Self::range("darkness", &l.darkness, 0.0, 20.0, 6.0), Self::range("detail", &l.detail, 0.0, 20.0, 1.0)],
                true,
            ),
            "photocopy" => (
                &l.photocopy,
                "sketch",
                vec![Self::range("detail", &l.detail, 1.0, 24.0, 24.0), Self::range("darkness", &l.darkness, 1.0, 50.0, 5.0)],
                true,
            ),
            "paper" => (&l.note_paper, "sketch", vec![Self::range("balance", &l.image_balance, 0.0, 50.0, 25.0)], true),
            _ => return None,
        };
        Some(GalleryFilter { key: key.to_owned(), label: label.clone(), category: category.to_owned(), params, uses_colors })
    }
}

/// The start: Cutout, Sponge with its eye off, and Neon Glow on top, selected.
fn start(catalog: &MockGallery) -> FilterGallerySession {
    let mut session = FilterGallerySession::new("cutout", catalog);
    let mut selected = 0;
    for (index, filter) in [(1, "sponge"), (2, "neon")] {
        session.apply(EffectChange::Add, catalog, &mut selected);
        session.apply(EffectChange::SetFilter { index, filter: filter.to_owned() }, catalog, &mut selected);
    }
    session.apply(EffectChange::Visible { index: 1, visible: false }, catalog, &mut selected);
    session
}

/// The preview: the picture multiplied by one tint per visible effect, so each filter shows.
struct Preview {
    image: MockImage,
    tints: Vec<[f64; 3]>,
    revision: u64,
}

impl Preview {
    /// The tint of the filter at `index` of the catalog: a hue round the circle.
    fn tint(index: usize) -> [f64; 3] {
        let phase = index as f64 / 6.0;
        [0.0, -1.0 / 3.0, 1.0 / 3.0].map(|shift| 1.0 - TINT_STRENGTH * (1.0 + (TAU * (phase + shift)).cos()) / 2.0)
    }

    fn of(session: &FilterGallerySession, image: MockImage, revision: u64) -> Self {
        let all: Vec<&str> = MockGallery::ARTISTIC.iter().chain(&MockGallery::SKETCH).copied().collect();
        let tints = session
            .effects
            .iter()
            .filter(|effect| effect.visible)
            .filter_map(|effect| all.iter().position(|key| *key == effect.filter))
            .map(Self::tint)
            .collect();
        Self { image, tints, revision }
    }
}

impl RasterSource for Preview {
    fn size(&self) -> [u32; 2] {
        self.image.size()
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        self.image.read(region, factor, out);
        for pixel in &mut out.pixels {
            let [mut r, mut g, mut b, a] = pixel.to_array().map(f64::from);
            for tint in &self.tints {
                (r, g, b) = (r * tint[0], g * tint[1], b * tint[2]);
            }
            *pixel = Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, a as u8);
        }
    }
}

// --- the kept workspace ---

/// The workspace a story keeps between passes.
struct Kept {
    seed: Option<RenderState>,
    workspace: FilterGalleryWorkspace,
    /// The ticket asked for and not answered.
    pending: Option<PreviewTicket>,
    /// Counts the answered tickets: the preview's revision.
    revision: u64,
}

type Slot = Arc<Mutex<Option<Kept>>>;

/// Shows the workspace while the story is open. `state` is `None` for a preview that answers at
/// once; `Some` holds a ticket while it is `Rendering` (at `progress` percent) and answers it, with
/// the note, once `Done`. A changed `state` opens a fresh workspace, which asks for a new ticket.
fn show_workspace(ui: &mut Ui, open: &mut bool, name: &'static str, state: Option<(RenderState, f64)>) {
    let id = ui.make_persistent_id(("craft_imaging::filter_gallery::story", name));
    let slot: Slot = ui.data_mut(|d| d.get_temp_mut_or_insert_with::<Slot>(id, Slot::default).clone());
    let Ok(mut kept) = slot.lock() else { return };
    if !(dialog_shown(ui, *open) && claim(ui.ctx(), "filter_gallery", ui.scope_id())) {
        *kept = None;
        return;
    }
    let ctx = ui.ctx().clone();
    let image = MockImage::default();
    let catalog = MockGallery { labels: GalleryStoryLabels::of(ui) };
    let seed = state.map(|(state, _)| state);
    if kept.as_ref().is_none_or(|k| k.seed != seed) {
        let subject = StoryLabels::of(ui).photo;
        let open = FilterGalleryOpen::new(image.size(), subject, start(&catalog));
        *kept = Some(Kept { seed, workspace: FilterGalleryWorkspace::new(open), pending: None, revision: 1 });
    }
    let Some(kept) = kept.as_mut() else { return };
    let thumbs = Thumbnails::<GalleryThumb>::in_context(&ctx, "gallery-story-thumbs", ThumbnailOptions::default());
    let preview = Preview::of(kept.workspace.session(), image, kept.revision);
    let out = kept.workspace.show(&ctx, &catalog, &thumbs, &FakeThumbSource::<GalleryThumb>::new(), &preview);
    for event in out.events {
        // A swatch press asks the app for a colour; the story has no picker.
        if let FilterGalleryEvent::PickColor(request) = event {
            kept.workspace.set_color(&request, [0.2, 0.7, 0.9]);
        }
    }
    if let Some(ticket) = out.preview_wanted {
        kept.pending = Some(ticket);
    }
    match (state, kept.pending) {
        (Some((RenderState::Rendering, progress)), Some(ticket)) => kept.workspace.preview_progress(ticket, (progress / 100.0) as f32),
        (_, Some(ticket)) => {
            kept.workspace.preview_ready(ticket);
            kept.pending = None;
            kept.revision += 1;
            if state.is_some() {
                let n = LocaleNumbers::of(ui).format(NOTE_MS, 0..=0);
                kept.workspace.set_note(Some(craft_i18n::fmt(&catalog.labels.preview_ms, &[("n", n.as_str())])));
            }
        }
        _ => {}
    }
    if out.outcome.is_some_and(Outcome::closes) {
        dialog_closed(ui, open);
    }
}

fn show_default(ui: &mut Ui, args: &mut DefaultArgs) {
    show_workspace(ui, &mut args.open, "default", None);
}

fn show_rendering(ui: &mut Ui, args: &mut RenderingArgs) {
    let state = Some((args.state, args.progress));
    show_workspace(ui, &mut args.open, "rendering", state);
}
