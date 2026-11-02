//! Stories of the Warp options-bar group (task ST-TR2): not registered. Registering them (task
//! RG2) adds `warp::fields_stories` to the crate's `register!` list, `WarpFields` to `COMPONENTS`
//! and [`WarpFieldsStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The real group in an options bar, over a fake style list. Every case keeps its own values and
//! applies the edits the group reports, as the draw library's `TransformBar` host does. Every
//! string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/warp-fields/group` | The group with a preset and with Custom; an armed split; the type-layer variant (Split and Grid disabled, no Custom) |

use craft_ui::localized::Labels as _;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;
use egui::Ui;

use super::{WarpFieldValues, WarpFields, WarpFieldsEdit, WarpFieldsOutput, WarpGridChoice, WarpSplit, WarpStyleEntry};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["WarpFields"];

craft_ui::words! {
    /// The captions and the fake style list of the Warp group story, English by default.
    #[non_exhaustive]
    pub(crate) struct WarpFieldsStoryLabels {
        preset: "warpFieldsStory.preset" = "A preset style";
        custom: "warpFieldsStory.custom" = "Custom";
        split: "warpFieldsStory.split" = "An armed split";
        on_type: "warpFieldsStory.onType" = "On a type layer";
        none: "warpFieldsStory.none" = "None";
        arc: "warpFieldsStory.arc" = "Arc";
        bulge: "warpFieldsStory.bulge" = "Bulge";
        flag: "warpFieldsStory.flag" = "Flag";
        wave: "warpFieldsStory.wave" = "Wave";
    }
}

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("composites/warp-fields/group", Level::Composites, "WarpFields", "Group", GroupArgs { bend: 50.0 }, show_group)]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct GroupArgs {
    /// Bend of the preset cases, in percent.
    #[arg(range = -100.0..=100.0)]
    bend: f64,
}

/// What a case starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Preset,
    Custom,
    Split,
    OnType,
}

/// A case's values and armed split, kept between passes.
#[derive(Clone)]
struct Fields {
    kind: Kind,
    bend: u64,
    values: WarpFieldValues,
    split: Option<WarpSplit>,
}

impl Fields {
    fn new(kind: Kind, bend: f64) -> Self {
        let custom = kind == Kind::Custom;
        let values = WarpFieldValues {
            style: if custom { "custom" } else { "arc" }.to_owned(),
            vertical: false,
            bend,
            horizontal: 0.0,
            vertical_distortion: 0.0,
            grid: if custom { WarpGridChoice::Custom } else { WarpGridChoice::Default },
        };
        Self { kind, bend: bend.to_bits(), values, split: (kind == Kind::Split).then_some(WarpSplit::Cross) }
    }

    fn load(ui: &Ui, salt: &'static str, kind: Kind, bend: f64) -> Self {
        let id = ui.make_persistent_id(("warp-fields-story", salt));
        ui.data(|d| d.get_temp::<Fields>(id)).filter(|f| f.kind == kind && f.bend == bend.to_bits()).unwrap_or_else(|| Self::new(kind, bend))
    }

    fn store(self, ui: &Ui, salt: &'static str) {
        let id = ui.make_persistent_id(("warp-fields-story", salt));
        ui.data_mut(|d| d.insert_temp(id, self));
    }

    /// What the app does with the group's edits: the fields take them over.
    fn apply(&mut self, output: WarpFieldsOutput) {
        for edit in output.edits {
            match edit {
                WarpFieldsEdit::Split(split) => self.split = split,
                WarpFieldsEdit::Grid(n) => {
                    self.values.grid = match n {
                        3 => WarpGridChoice::Three,
                        4 => WarpGridChoice::Four,
                        5 => WarpGridChoice::Five,
                        _ => WarpGridChoice::Default,
                    };
                }
                WarpFieldsEdit::Style(key) => self.values.style = key,
                WarpFieldsEdit::Orientation { vertical } => self.values.vertical = vertical,
                WarpFieldsEdit::Bend(event) => self.values.bend = event.value().copied().unwrap_or(self.values.bend),
                WarpFieldsEdit::Horizontal(event) => self.values.horizontal = event.value().copied().unwrap_or(self.values.horizontal),
                WarpFieldsEdit::Vertical(event) => {
                    self.values.vertical_distortion = event.value().copied().unwrap_or(self.values.vertical_distortion);
                }
            }
        }
    }
}

/// The fake style list: None, four presets and Custom.
fn styles(labels: &WarpFieldsStoryLabels) -> Vec<WarpStyleEntry> {
    let entry = |key: &str, label: &String, preset: bool, custom: bool| WarpStyleEntry { key: key.to_owned(), label: label.clone(), preset, custom };
    vec![
        entry("none", &labels.none, false, false),
        entry("arc", &labels.arc, true, false),
        entry("bulge", &labels.bulge, true, false),
        entry("flag", &labels.flag, true, false),
        entry("wave", &labels.wave, true, false),
        entry("custom", &labels.custom, false, true),
    ]
}

fn show_group(ui: &mut Ui, args: &mut GroupArgs) {
    let labels = WarpFieldsStoryLabels::of(ui);
    let styles = styles(&labels);
    let cases = [
        (Kind::Preset, "preset", &labels.preset),
        (Kind::Custom, "custom", &labels.custom),
        (Kind::Split, "split", &labels.split),
        (Kind::OnType, "on-type", &labels.on_type),
    ];
    for (kind, salt, text) in cases {
        ui.add(Text::new(text).content(Content::Muted));
        let mut fields = Fields::load(ui, salt, kind, args.bend);
        let mut output = WarpFieldsOutput::default();
        OptionsBar::new(salt).label(text.as_str()).show(ui, |slots| {
            WarpFields::new("group", &styles, fields.values.clone())
                .split(fields.split)
                .on_type(kind == Kind::OnType)
                .output(&mut output)
                .add_to(slots);
        });
        fields.apply(output);
        fields.store(ui, salt);
    }
}
