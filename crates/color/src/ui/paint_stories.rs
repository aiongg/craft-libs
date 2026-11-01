//! Stories of the paint kinds and the paint list's row actions (task ST-SH2; task RG2 registers
//! them). Registering them adds `paint_stories::stories()` to the colour stories, the components
//! to `COMPONENTS` and [`PaintStoryLabels`] to `WORDS`.
//!
//! Every story uses fake pattern and picture previews (small generated textures) and fake paints;
//! the Pattern and Picture tabs hold fake editors made of buttons, since craft-color cannot use
//! craft-draw's. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/paint-chip/kinds` | A chip with five tabs (pattern and picture editors from fakes) |
//! | `patterns/paint-list/list` | Rows with hex, opacity, a bound style, remove (`PaintListOptions::SELECTION_COLORS`) |

#![expect(dead_code, reason = "registered by task RG2")]

use craft_theme::Srgba;
use craft_ui::controls::Button;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Gap, Stack, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryLabels, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions, Ui};

use super::{
    FillKind, GradientPreview, ImagePreview, Paint, PaintChip, PaintKindLabels, PaintLabels, PaintLink, PaintLinkKind, PaintList, PaintListEvent,
    PaintListOptions, PaintRow,
};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PaintChip", "PaintList"];

craft_ui::words! {
    /// The captions of the paint stories, English by default.
    #[non_exhaustive]
    pub(crate) struct PaintStoryLabels {
        fill: "paintStory.fill" = "Fill";
        brand: "paintStory.brand" = "Brand";
        shadow: "paintStory.shadow" = "Shadow";
        sunrise: "paintStory.sunrise" = "Sunrise";
        bubbles: "paintStory.bubbles" = "Bubbles";
        checks: "paintStory.checks" = "Checks";
        harbour: "paintStory.harbour" = "Harbour.png";
        meadow: "paintStory.meadow" = "Meadow.jpg";
        /// The name of a colour style a row is bound to.
        brand_style: "paintStory.brandStyle" = "Brand blue";
        /// The kind line under the chip: `{kind}`.
        kind: "paintStory.kind" = "Kind: {kind}";
        /// The last list action: `{action}` and the row `{row}` (from 1).
        action: "paintStory.action" = "{action}, row {row}";
        edited: "paintStory.edited" = "Edited";
        hex: "paintStory.hex" = "Hex";
        opacity: "paintStory.opacity" = "Opacity";
        bound: "paintStory.bound" = "Bind";
        detached: "paintStory.detached" = "Detach";
        selected_users: "paintStory.selectedUsers" = "Select layers";
        added: "paintStory.added" = "Add";
        removed: "paintStory.removed" = "Remove";
        renamed: "paintStory.renamed" = "Rename";
    }
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/paint-chip/kinds", Level::Composites, "PaintChip", "Kinds", ChipArgs { no_color: true, disabled: false }, show_chip),
        typed_story!(
            "patterns/paint-list/list",
            Level::Patterns,
            "PaintList",
            "List",
            ListArgs { actions: true, add: true, remove: true },
            show_list
        ),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ChipArgs {
    /// Offers the No Color tab.
    #[arg(name = "no color")]
    no_color: bool,
    disabled: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ListArgs {
    /// FigCraft's Selection colors: edit, hex, opacity, bind and select users.
    actions: bool,
    /// Adds the Add button under the rows.
    add: bool,
    /// Adds a Remove button per row.
    remove: bool,
}

// --- the fakes ---

/// The fake previews' textures, kept for the life of the story.
#[derive(Clone)]
struct Textures {
    patterns: [TextureHandle; 2],
    pictures: [TextureHandle; 2],
}

fn image(size: [usize; 2], pixel: impl Fn(usize, usize) -> Color32) -> ColorImage {
    let mut bytes = Vec::with_capacity(size[0] * size[1] * 4);
    for y in 0..size[1] {
        for x in 0..size[0] {
            bytes.extend_from_slice(&pixel(x, y).to_array());
        }
    }
    ColorImage::from_rgba_unmultiplied(size, &bytes)
}

/// The size of a pattern tile in pixels, and of a picture.
const TILE: usize = 8;
const PICTURE: [usize; 2] = [40, 24];

fn textures(ctx: &Context, ink: Color32, paper: Color32) -> Textures {
    let load = |name: &str, image: ColorImage| ctx.load_texture(name, image, TextureOptions::NEAREST);
    let half = TILE / 2;
    Textures {
        patterns: [
            load("paint-story-bubbles", image([TILE, TILE], |x, y| if (x % half == 0) && (y % half == 0) { ink } else { paper })),
            load("paint-story-checks", image([TILE, TILE], |x, y| if (x / half + y / half).is_multiple_of(2) { ink } else { paper })),
        ],
        pictures: [
            load("paint-story-harbour", image(PICTURE, |x, _| ink.lerp_to_gamma(paper, x as f32 / PICTURE[0] as f32))),
            load("paint-story-meadow", image(PICTURE, |_, y| ink.lerp_to_gamma(paper, y as f32 / PICTURE[1] as f32))),
        ],
    }
}

fn preview(name: String, texture: &TextureHandle) -> ImagePreview {
    let [w, h] = texture.size();
    ImagePreview { name, texture: Some(texture.id()), size: [w as f32, h as f32] }
}

/// A fake editor: one button per image; a press returns that image's preview.
fn pick_image(ui: &mut Ui, images: &[(&str, &TextureHandle)]) -> Option<ImagePreview> {
    let mut picked = None;
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (name, texture) in images {
            if ui.add(Button::new(*name)).clicked() {
                picked = Some(preview((*name).to_owned(), texture));
            }
        }
    });
    picked
}

fn fake_textures(ui: &Ui) -> Textures {
    let tokens = Tokens::of(ui);
    let ctx = ui.ctx();
    let kept = shared(ui, "paint-textures", || textures(ctx, tokens.color.accent.solid.to_egui(), Content::Muted.color(&tokens).to_egui()));
    lock(&kept).clone()
}

fn accent(ui: &Ui) -> Srgba {
    Srgba::from(Tokens::of(ui).color.accent.solid)
}

fn ink(ui: &Ui) -> Srgba {
    Srgba::from(Content::Normal.color(&Tokens::of(ui)))
}

fn sunrise(ui: &Ui) -> GradientPreview {
    GradientPreview { stops: vec![(0.0, accent(ui)), (1.0, ink(ui))] }
}

// --- chip ---

/// A chip with the five tabs: No Color, Solid (the picker), Gradient (no editor body), and fake
/// Pattern and Picture editors. The line below names the kind.
fn show_chip(ui: &mut Ui, args: &mut ChipArgs) {
    let own = PaintStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let textures = fake_textures(ui);
    let paint = shared(ui, "paint-chip", || Paint::Solid(accent(ui)));
    let mut paint = lock(&paint);
    let fallback = accent(ui);
    let first_pattern = preview(own.bubbles.clone(), &textures.patterns[0]);
    let first_picture = preview(own.harbour.clone(), &textures.pictures[0]);
    let patterns = [(own.bubbles.as_str(), &textures.patterns[0]), (own.checks.as_str(), &textures.patterns[1])];
    let pictures = [(own.harbour.as_str(), &textures.pictures[0]), (own.meadow.as_str(), &textures.pictures[1])];
    let chip = PaintChip::new("paint-kinds", own.fill.clone(), &paint, fallback, &numbers)
        .none_allowed(args.no_color)
        .enabled(!args.disabled)
        .gradient_editor(sunrise(ui), |_| None)
        .pattern_editor(first_pattern, |ui| pick_image(ui, &patterns))
        .picture_editor(first_picture, |ui| pick_image(ui, &pictures));
    let edited = chip.show(ui).event.and_then(|event| event.value().cloned());
    if let Some(value) = edited {
        *paint = value;
    }
    let kinds = (PaintLabels::of(ui), PaintKindLabels::of(ui));
    let kind = match FillKind::of(&paint) {
        FillKind::None => kinds.0.no_color,
        FillKind::Solid => kinds.0.solid,
        FillKind::Gradient => kinds.0.gradient,
        FillKind::Pattern => kinds.1.pattern,
        FillKind::Picture => kinds.1.picture,
    };
    ui.add(Text::new(craft_i18n::fmt(&own.kind, &[("kind", &kind)])).content(Content::Muted));
}

// --- list ---

/// What the list story keeps: the paints, their bindings, the selection and the last action.
struct Kept {
    paints: Vec<(String, Paint)>,
    rows: Vec<PaintRow>,
    selected: Option<usize>,
    note: String,
}

fn fresh(ui: &Ui) -> Kept {
    let own = PaintStoryLabels::of(ui);
    let textures = fake_textures(ui);
    let brand = accent(ui);
    Kept {
        paints: vec![
            (own.brand.clone(), Paint::Solid(brand)),
            (own.shadow.clone(), Paint::Solid(Srgba { a: 0.5, ..ink(ui) })),
            (own.sunrise.clone(), Paint::Gradient(sunrise(ui))),
            (own.bubbles.clone(), Paint::Pattern(preview(own.bubbles.clone(), &textures.patterns[0]))),
            (own.harbour.clone(), Paint::Picture(preview(own.harbour.clone(), &textures.pictures[0]))),
        ],
        rows: vec![PaintRow::linked(PaintLink { name: own.brand_style.clone(), kind: PaintLinkKind::Style })],
        selected: Some(0),
        note: String::new(),
    }
}

fn show_list(ui: &mut Ui, args: &mut ListArgs) {
    let words = StoryLabels::of(ui);
    let own = PaintStoryLabels::of(ui);
    let kept = shared(ui, "paint-list", || fresh(ui));
    let mut kept = lock(&kept);
    let mut options = if args.actions { PaintListOptions::SELECTION_COLORS } else { PaintListOptions::default() };
    options.add = args.add && args.actions;
    options.remove = args.remove && args.actions;
    let response =
        PaintList::new("paint-list-story", own.fill.clone(), &kept.paints).selected(kept.selected).options(options).rows(&kept.rows).show(ui);
    kept.selected = response.selected.or(kept.selected);
    let act = |note: &str, row: usize, kept: &mut Kept| {
        kept.note = craft_i18n::fmt(&own.action, &[("action", note), ("row", &(row + 1).to_string())]);
    };
    for event in response.events {
        match event {
            PaintListEvent::Edit { row, event } => {
                act(&own.edited, row, &mut kept);
                if let (Some(value), Some(slot)) = (event.value(), kept.paints.get_mut(row)) {
                    slot.1 = value.clone();
                }
            }
            PaintListEvent::Hex { row, color } => {
                act(&own.hex, row, &mut kept);
                if let Some(slot) = kept.paints.get_mut(row) {
                    slot.1 = Paint::Solid(color);
                }
            }
            PaintListEvent::Opacity { row, event } => {
                act(&own.opacity, row, &mut kept);
                if let (Some(alpha), Some((_, Paint::Solid(color)))) = (event.value(), kept.paints.get_mut(row)) {
                    color.a = *alpha;
                }
            }
            PaintListEvent::Bind { row } => {
                act(&own.bound, row, &mut kept);
                if kept.rows.len() <= row {
                    kept.rows.resize(row + 1, PaintRow::default());
                }
                kept.rows[row] = PaintRow::linked(PaintLink { name: own.brand_style.clone(), kind: PaintLinkKind::Style });
            }
            PaintListEvent::Detach { row } => {
                act(&own.detached, row, &mut kept);
                if let Some(slot) = kept.rows.get_mut(row) {
                    *slot = PaintRow::default();
                }
            }
            PaintListEvent::SelectUsers { row } => act(&own.selected_users, row, &mut kept),
            PaintListEvent::Add => {
                let color = accent(ui);
                kept.paints.push((own.fill.clone(), Paint::Solid(color)));
                let row = kept.paints.len() - 1;
                act(&own.added, row, &mut kept);
            }
            PaintListEvent::Remove { row } => {
                act(&own.removed, row, &mut kept);
                if row < kept.paints.len() {
                    kept.paints.remove(row);
                }
                if row < kept.rows.len() {
                    kept.rows.remove(row);
                }
                kept.selected = kept.selected.filter(|s| *s < kept.paints.len());
            }
            PaintListEvent::Rename { row, name } => {
                act(&own.renamed, row, &mut kept);
                if let Some(slot) = kept.paints.get_mut(row) {
                    slot.0 = name;
                }
            }
        }
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            *kept = fresh(ui);
        }
        let line = if kept.note.is_empty() { words.no_event.clone() } else { kept.note.clone() };
        ui.add(Text::new(line).content(Content::Muted));
    });
}
