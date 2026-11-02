//! Stories of the Brush Preset picker, its cards and chip (task ST-BR1; task RG2 registers them
//! and adds `BrushPresetPicker`, `BrushPresetCards` and `BrushPresetChip` to the components).
//! Fake presets in three groups; their tips and strokes are drawn with the painter, so every
//! state shows without an engine. The preset names and the notes are story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/brush-preset-picker/open` | The picker open on 40 fake presets in three groups: part boxes (gear menu), scale, search; Rename and Delete act on the fake library |
//! | `composites/brush-preset-picker/sampled-tip` | A sampled tip: no Hardness slider |
//! | `composites/brush-preset-picker/empty` | An empty library |
//! | `composites/brush-preset-cards/parts` | The card in all seven part combinations at one of the scales 0.15, 0.7, 1 and 2 |
//! | `controls/brush-preset-chip/states` | Closed, open, a four-digit size |

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Ui, vec2};

use super::{
    BrushCardLabels, BrushCardLayout, BrushCardParts, BrushCardsState, BrushPick, BrushPickerLabels, BrushPickerState,
    BrushPickerValues, BrushPresetCards, BrushPresetChip, BrushPresetPicker,
};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::popover::PopoverAnchor;
use craft_ui::presets::{PresetEvent, PresetRow};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, Surface, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["BrushPresetPicker", "BrushPresetCards", "BrushPresetChip"];

craft_ui::words! {
    /// The captions and fake names of the brush preset stories, English by default.
    #[non_exhaustive]
    pub(crate) struct BrushPresetStoryLabels {
        group_basic: "brushPresetStory.groupBasic" = "Basic";
        group_chalk: "brushPresetStory.groupChalk" = "Chalk and Charcoal";
        group_effects: "brushPresetStory.groupEffects" = "Special Effects";
        /// A fake preset's name; `{n}` is its number.
        preset: "brushPresetStory.preset" = "Brush {n}";
        /// The note under the picker after a pick; `{name}` is the preset.
        picked: "brushPresetStory.picked" = "Picked {name}";
        /// The note after the picker closed a pick (a double-click).
        activated: "brushPresetStory.activated" = "Activated {name}";
        renamed: "brushPresetStory.renamed" = "Renamed to {name}";
        deleted: "brushPresetStory.deleted" = "Deleted {name}";
        open_settings: "brushPresetStory.openSettings" = "Brush Settings asked for";
        new_preset: "brushPresetStory.newPreset" = "New Brush Preset asked for";
        import: "brushPresetStory.import" = "Import Brushes asked for";
        nothing: "brushPresetStory.nothing" = "No preset picked";
        /// The chip captions.
        chip_closed: "brushPresetStory.chipClosed" = "Closed";
        chip_open: "brushPresetStory.chipOpen" = "Open";
        chip_wide: "brushPresetStory.chipWide" = "Four digits";
        /// Joins the parts a card shows in a caption.
        parts_join: "brushPresetStory.partsJoin" = ", ";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/brush-preset-picker/open",
            Level::Composites,
            "BrushPresetPicker",
            "Open",
            PickerArgs { diameter: 30.0, hardness: 80.0, import: true },
            show_open
        ),
        typed_story!(
            "composites/brush-preset-picker/sampled-tip",
            Level::Composites,
            "BrushPresetPicker",
            "Sampled tip",
            SampledArgs { diameter: 120.0 },
            show_sampled
        ),
        typed_story!(
            "composites/brush-preset-picker/empty",
            Level::Composites,
            "BrushPresetPicker",
            "Empty",
            PickerArgs { diameter: 30.0, hardness: 80.0, import: true },
            show_empty
        ),
        typed_story!(
            "composites/brush-preset-cards/parts",
            Level::Composites,
            "BrushPresetCards",
            "Parts",
            CardsArgs { scale: ScaleArg::One },
            show_cards
        ),
        typed_story!("controls/brush-preset-chip/states", Level::Controls, "BrushPresetChip", "States", ChipArgs { diameter: 30.0 }, show_chip)
            .on(Surface::Chrome),
    ]
}

// --- args ---

#[derive(StoryArgs, Debug, PartialEq)]
struct PickerArgs {
    /// The brush diameter in pixels.
    #[arg(range = 1.0..=2500.0)]
    diameter: f64,
    /// The hardness in percent.
    #[arg(range = 0.0..=100.0)]
    hardness: f64,
    /// Shows Import Brushes… in the gear menu.
    import: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SampledArgs {
    #[arg(range = 1.0..=2500.0)]
    diameter: f64,
}

/// The card scales of the spec's story.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ScaleArg {
    #[choice(name = "0.15")]
    Min,
    #[choice(name = "0.7")]
    Compact,
    #[choice(name = "1")]
    One,
    #[choice(name = "2")]
    Max,
}

impl ScaleArg {
    fn scale(self) -> f32 {
        match self {
            ScaleArg::Min => 0.15,
            ScaleArg::Compact => 0.7,
            ScaleArg::One => 1.0,
            ScaleArg::Max => 2.0,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CardsArgs {
    /// The card scale.
    scale: ScaleArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ChipArgs {
    /// The diameter shown on the first two chips.
    #[arg(range = 1.0..=2500.0)]
    diameter: f64,
}

// --- fake brushes ---

/// The fake library has 40 presets in three groups.
const PRESETS: u32 = 40;
/// Group keys sit above every preset key.
const GROUP_BASE: u32 = 1000;

/// The tip shapes the painter draws, by preset.
#[derive(Clone, Copy, PartialEq)]
enum Tip {
    Hard,
    Soft,
    Star,
    Sampled,
}

fn tip_of(key: u32) -> Tip {
    [Tip::Hard, Tip::Soft, Tip::Star, Tip::Sampled][(key % 4) as usize]
}

/// A fake preset's diameter in pixels.
fn diameter_of(key: u32) -> f64 {
    f64::from(5 + (key * 37) % 200)
}

fn ink(ui: &Ui) -> Color32 {
    Tokens::of(ui).color.text.normal.to_egui()
}

/// The tip of `tip` inside `rect`.
fn paint_tip(painter: &Painter, rect: Rect, tip: Tip, color: Color32) {
    let (centre, radius) = (rect.center(), rect.width().min(rect.height()) * 0.45);
    match tip {
        Tip::Hard => {
            painter.circle_filled(centre, radius, color);
        }
        Tip::Soft => {
            const RINGS: u8 = 6;
            for i in 0..RINGS {
                let t = f32::from(i) / f32::from(RINGS);
                painter.circle_filled(centre, radius * (1.0 - t), color.gamma_multiply(0.18));
            }
        }
        Tip::Star => {
            let points = (0..10)
                .map(|i| {
                    let angle = std::f32::consts::TAU * i as f32 / 10.0 - std::f32::consts::FRAC_PI_2;
                    let r = if i % 2 == 0 { radius } else { radius * 0.45 };
                    centre + vec2(angle.cos(), angle.sin()) * r
                })
                .collect();
            painter.add(Shape::closed_line(points, Stroke::new((radius * 0.15).max(1.0), color)));
        }
        Tip::Sampled => {
            for (x, y) in [(-1.0, -1.0), (1.0, -0.5), (0.0, 0.0), (-0.6, 0.9), (0.8, 1.0)] {
                painter.circle_filled(centre + vec2(x, y) * radius * 0.55, radius * 0.22, color);
            }
        }
    }
}

/// The stroke of preset `key` as a wave through `rect`, thicker for the soft tips.
fn paint_stroke(painter: &Painter, rect: Rect, key: u32, color: Color32) {
    let width = rect.height() * (0.12 + 0.07 * (key % 4) as f32);
    let amp = (rect.height() * 0.5 - width * 0.5).max(0.0) * 0.8;
    let points: Vec<Pos2> = (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            Pos2::new(rect.left() + rect.width() * t, rect.center().y - amp * (t * std::f32::consts::TAU).sin())
        })
        .collect();
    let alpha = if tip_of(key) == Tip::Soft { 0.5 } else { 1.0 };
    painter.add(Shape::line(points, Stroke::new(width, color.gamma_multiply(alpha))));
}

/// The fake library the picker stories edit: the key and name of each preset in its group.
#[derive(Clone, Debug)]
struct Library {
    groups: Vec<Group>,
}

/// A group's key, name and presets (key and name).
type Group = (u32, String, Vec<(u32, String)>);

impl Library {
    fn new(words: &BrushPresetStoryLabels, count: u32) -> Self {
        let names = [&words.group_basic, &words.group_chalk, &words.group_effects];
        let mut groups: Vec<Group> = names.iter().zip(0u32..).map(|(name, i)| (GROUP_BASE + i, (*name).clone(), Vec::new())).collect();
        for key in 0..count {
            let group = (key * 3 / PRESETS.max(1)).min(2) as usize;
            groups[group].2.push((key, craft_i18n::fmt(&words.preset, &[("n", &(key + 1).to_string())])));
        }
        Self { groups }
    }

    fn rows(&self, labels: &BrushCardLabels, numbers: &LocaleNumbers) -> Vec<PresetRow<u32>> {
        let mut rows = Vec::new();
        for (key, name, presets) in &self.groups {
            rows.push(PresetRow::group(*key, name.clone(), 0).count(presets.len()));
            for (preset, label) in presets {
                rows.push(BrushPresetCards::row(*preset, label.clone(), 1, diameter_of(*preset), labels, numbers));
            }
        }
        rows
    }

    fn name_of(&self, key: u32) -> String {
        self.groups.iter().flat_map(|g| g.2.iter()).find(|p| p.0 == key).map(|p| p.1.clone()).unwrap_or_default()
    }

    fn rename(&mut self, key: u32, text: &str) {
        for preset in self.groups.iter_mut().flat_map(|g| g.2.iter_mut()).filter(|p| p.0 == key) {
            preset.1 = text.to_owned();
        }
    }

    fn delete(&mut self, key: u32) {
        for group in &mut self.groups {
            group.2.retain(|p| p.0 != key);
        }
    }
}

// --- picker ---

/// What a picker story keeps between passes.
#[derive(Clone, Debug)]
struct Kept {
    state: BrushPickerState<u32>,
    library: Library,
    open: bool,
    current: Option<u32>,
    note: String,
}

/// The chip, the picker below it and a note of the last pick. `hardness` is `None` for a sampled
/// tip. Edits to the size and hardness go back into the story's args.
fn show_picker(ui: &mut Ui, id: &str, count: u32, diameter: &mut f64, hardness: Option<&mut f64>, import: bool) {
    let words = BrushPresetStoryLabels::of(ui);
    let card_labels = BrushCardLabels::of(ui);
    let labels = BrushPickerLabels::of(ui);
    let in_overview = craft_ui::story::in_overview(ui.ctx());
    let kept = shared(ui, id, || Kept {
        state: BrushPickerState::new(),
        library: Library::new(&words, count),
        // An Overview tile starts closed: an open popover is a layer of its own that the tile cannot
        // clip, and its outside press would swallow clicks on the other tiles.
        open: !in_overview,
        current: None,
        note: String::new(),
    });
    let mut kept = lock(&kept);
    let kept = &mut *kept;
    let color = ink(ui);
    let chip = BrushPresetChip::new((id, "chip")).diameter(*diameter).open(kept.open).labels(labels.clone()).show(ui, |p, r| {
        paint_tip(p, r, Tip::Hard, color);
    });
    let rows = kept.library.rows(&card_labels, &LocaleNumbers::of(ui));
    let current = kept.current;
    let mut open = kept.open;
    let out = BrushPresetPicker::new((id, "picker"), PopoverAnchor::Control(chip.response.clone()), &mut kept.state, &rows)
        .toggle(chip.response.id)
        .current(current.as_ref())
        .values(BrushPickerValues { diameter: *diameter, hardness: hardness.as_deref().map(|h| *h / 100.0) })
        .import(import)
        .labels(labels)
        .card_labels(card_labels)
        .tip(move |p, r, k| paint_tip(p, r, tip_of(*k), color))
        .stroke(move |p, r, k| paint_stroke(p, r, *k, color))
        .diameter_of(|k| Some(diameter_of(*k)))
        .show(ui, &mut open);
    if chip.clicked {
        open = !open;
    }
    kept.open = open;
    if let Some(value) = out.diameter.as_ref().and_then(|e| e.value()) {
        *diameter = *value;
    }
    if let (Some(h), Some(value)) = (hardness, out.hardness.as_ref().and_then(|e| e.value())) {
        *h = (*value * 100.0).clamp(0.0, 100.0);
    }
    for pick in out.picks {
        match pick {
            BrushPick::List(PresetEvent::Select { key, .. }) => {
                kept.current = Some(key);
                kept.note = craft_i18n::fmt(&words.picked, &[("name", &kept.library.name_of(key))]);
            }
            BrushPick::List(PresetEvent::Activate { key, .. }) => {
                kept.current = Some(key);
                kept.note = craft_i18n::fmt(&words.activated, &[("name", &kept.library.name_of(key))]);
            }
            BrushPick::List(PresetEvent::Rename { key, text }) => {
                kept.library.rename(key, &text);
                kept.note = craft_i18n::fmt(&words.renamed, &[("name", &text)]);
            }
            BrushPick::List(PresetEvent::Delete(key)) => {
                kept.note = craft_i18n::fmt(&words.deleted, &[("name", &kept.library.name_of(key))]);
                kept.library.delete(key);
                if kept.current == Some(key) {
                    kept.current = None;
                }
            }
            BrushPick::OpenSettings => kept.note = words.open_settings.clone(),
            BrushPick::NewPreset => kept.note = words.new_preset.clone(),
            BrushPick::Import => kept.note = words.import.clone(),
            _ => {}
        }
    }
    let note = if kept.note.is_empty() { words.nothing.clone() } else { kept.note.clone() };
    ui.add(Text::new(note).content(Content::Muted));
}

fn show_open(ui: &mut Ui, args: &mut PickerArgs) {
    let PickerArgs { diameter, hardness, import } = args;
    show_picker(ui, "open", PRESETS, diameter, Some(hardness), *import);
}

fn show_sampled(ui: &mut Ui, args: &mut SampledArgs) {
    show_picker(ui, "sampled", PRESETS, &mut args.diameter, None, true);
}

fn show_empty(ui: &mut Ui, args: &mut PickerArgs) {
    let PickerArgs { diameter, hardness, import } = args;
    show_picker(ui, "empty", 0, diameter, Some(hardness), *import);
}

// --- cards ---

/// The seven non-empty combinations of parts, as bits (name, stroke, tip).
fn combos() -> impl Iterator<Item = BrushCardParts> {
    (1u8..8).map(|bits| BrushCardParts { name: bits & 1 != 0, stroke: bits & 2 != 0, tip: bits & 4 != 0 })
}

/// Every parts combination as its own card list of three presets, at the chosen scale.
fn show_cards(ui: &mut Ui, args: &mut CardsArgs) {
    let words = BrushPresetStoryLabels::of(ui);
    let labels = BrushPickerLabels::of(ui);
    let card_labels = BrushCardLabels::of(ui);
    let color = ink(ui);
    let scale = args.scale.scale();
    let numbers = LocaleNumbers::of(ui);
    let rows: Vec<PresetRow<u32>> = (0..3)
        .map(|k| BrushPresetCards::row(k, craft_i18n::fmt(&words.preset, &[("n", &(k + 1).to_string())]), 0, diameter_of(k), &card_labels, &numbers))
        .collect();
    let style = BrushPresetCards::<u32>::style(&Tokens::of(ui));
    for (i, parts) in combos().enumerate() {
        let mut names = Vec::new();
        if parts.tip {
            names.push(labels.part_tip.as_str());
        }
        if parts.stroke {
            names.push(labels.part_stroke.as_str());
        }
        if parts.name {
            names.push(labels.part_name.as_str());
        }
        ui.add(Text::new(names.join(&words.parts_join)).content(Content::Muted));
        let kept = shared(ui, &format!("cards-{i}"), BrushCardsState::<u32>::default);
        let mut state = lock(&kept);
        state.parts = parts;
        state.scale = scale;
        let height = BrushCardLayout::cell_size(parts, scale, ui.available_width(), &style).y + 2.0 * style.inset;
        let _events = BrushPresetCards::new(("brush-cards", i), &mut *state, &rows)
            .height(height)
            .labels(card_labels.clone())
            .tip(move |p, r, k| paint_tip(p, r, tip_of(*k), color))
            .stroke(move |p, r, k| paint_stroke(p, r, *k, color))
            .diameter_of(|k| Some(diameter_of(*k)))
            .show(ui);
    }
}

// --- chip ---

/// A closed chip, an open one and a four-digit size, side by side.
fn show_chip(ui: &mut Ui, args: &mut ChipArgs) {
    let words = BrushPresetStoryLabels::of(ui);
    let labels = BrushPickerLabels::of(ui);
    let color = ink(ui);
    ui.horizontal_top(|ui| {
        for (i, (caption, open, diameter)) in
            [(&words.chip_closed, false, args.diameter), (&words.chip_open, true, args.diameter), (&words.chip_wide, false, 1234.0)]
                .into_iter()
                .enumerate()
        {
            ui.vertical(|ui| {
                ui.add(Text::new(caption.clone()).content(Content::Muted));
                BrushPresetChip::new(("chip", i)).diameter(diameter).open(open).labels(labels.clone()).show(ui, |p, r| {
                    paint_tip(p, r, tip_of(i as u32), color);
                });
            });
        }
    });
}
