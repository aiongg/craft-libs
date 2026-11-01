//! Stories of the Crop options bar and its menus (task ST-TO1; task RG2 registers them and their
//! components).
//!
//! Every story fills an `OptionsBar` with `CropOptionsBar` (or shows one menu) over settings kept in
//! the story's state, with a fake 2400 × 1500 px document at 300 ppi and two fake saved presets, and
//! shows the bar's last event in a status line. Sizes are args with defaults; the strings that
//! have a story word use it, the rest are plain.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/crop-options/bar` | The whole bar in a narrow and a wide width, with saved presets and the New and Delete entries |
//! | `patterns/crop-options/overlay-menu` | The overlay menu with its shortcuts |
//! | `patterns/crop-options/gear-menu` | The gear popover, Classic Mode greying two rows, the Custom colour request |

#![expect(dead_code, reason = "registered by task RG2")]

use egui::{Ui, vec2};
use kurbo::Size;

use super::stories_tool::presets;
use super::{
    CropBarEvent, CropBarLabels, CropBarOutput, CropDocKey, CropDocument, CropGuide, CropOptionsBar, CropOptionsMenu, CropOverlayMenu, CropSettings,
    GuideShow, ShieldColor,
};
use craft_ui::localized::Labels as _;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Crop options bar", "Crop overlay menu", "Crop options menu"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/crop-options/bar",
            Level::Patterns,
            "Crop options bar",
            "Bar",
            BarArgs { narrow: 520.0, wide: 1200.0, document: true, saved_presets: true },
            show_bar
        )
        .on(Surface::Chrome),
        typed_story!(
            "patterns/crop-options/overlay-menu",
            Level::Patterns,
            "Crop overlay menu",
            "Overlay menu",
            OverlayArgs { guide: GuideArg::Thirds, show: ShowArg::Auto },
            show_overlay_menu
        )
        .on(Surface::Chrome),
        typed_story!(
            "patterns/crop-options/gear-menu",
            Level::Patterns,
            "Crop options menu",
            "Gear menu",
            GearArgs { classic_mode: true, shield: true },
            show_gear_menu
        )
        .on(Surface::Chrome),
    ]
}

/// The fake document the bar's Original Ratio and Front Image read.
fn document() -> CropDocument {
    CropDocument::new(CropDocKey(1), Size::new(2400.0, 1500.0)).resolution(300.0)
}

fn state_key(ui: &Ui, name: &str) -> egui::Id {
    ui.make_persistent_id(("craft_draw::crop::bar_story", name))
}

fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

// --- bar ---

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    /// The narrow bar's width in points.
    #[arg(range = 320.0..=800.0)]
    narrow: f32,
    /// The wide bar's width in points.
    #[arg(range = 800.0..=1800.0)]
    wide: f32,
    /// A document is open: Original Ratio and Front Image are enabled.
    document: bool,
    /// The two saved presets are in the list, so Delete Crop Preset… is enabled.
    #[arg(name = "saved presets")]
    saved_presets: bool,
}

/// What the bar story keeps: the settings both bars edit, the Straighten button and the last event.
#[derive(Clone, Debug)]
struct BarState {
    settings: CropSettings,
    armed: bool,
    last: Option<String>,
}

fn event_text(event: CropBarEvent, settings: &CropSettings, labels: &CropBarLabels) -> String {
    match event {
        CropBarEvent::Ratio { editing } => format!("Ratio{}", if editing { " (editing)" } else { "" }),
        CropBarEvent::Straighten => "Straighten".to_owned(),
        CropBarEvent::Commit => "Commit".to_owned(),
        CropBarEvent::Cancel => "Cancel".to_owned(),
        CropBarEvent::NewPreset => match settings.preset_name(labels) {
            Some(name) => format!("New Crop Preset: {name}"),
            None => "New Crop Preset".to_owned(),
        },
        CropBarEvent::DeletePreset => "Delete Crop Preset".to_owned(),
        CropBarEvent::PickShieldColor => "Crop Shield colour requested".to_owned(),
    }
}

/// The same bar twice, at the narrow and the wide width. Both edit the same settings; the bar
/// scrolls sideways when it does not fit.
fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let words = StoryLabels::of(ui);
    let key = state_key(ui, "bar");
    let mut state: BarState = ui.data(|d| d.get_temp(key)).unwrap_or(BarState { settings: CropSettings::default(), armed: false, last: None });
    let labels = CropBarLabels::of(ui);
    let (document, users) = (document(), presets());
    for (name, width) in [("narrow", args.narrow), ("wide", args.wide)] {
        let mut out = CropBarOutput::default();
        ui.allocate_ui(vec2(width, 0.0), |ui| {
            ui.set_max_width(width);
            OptionsBar::new(("crop-bar-story", name)).label(name).show(ui, |bar| {
                let mut crop = CropOptionsBar::new(("crop", name), &mut state.settings).straighten(state.armed).output(&mut out);
                if args.document {
                    crop = crop.document(&document);
                }
                if args.saved_presets {
                    crop = crop.user_presets(&users);
                }
                crop.add_to(bar);
            });
        });
        for event in out.take() {
            if event == CropBarEvent::Straighten {
                state.armed = !state.armed;
            }
            state.last = Some(event_text(event, &state.settings, &labels));
        }
    }
    let text = match &state.last {
        Some(event) => craft_i18n::fmt(&words.last_event, &[("event", event)]),
        None => words.no_event.clone(),
    };
    status(ui, text);
    ui.data_mut(|d| d.insert_temp(key, state));
}

// --- overlay menu ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum GuideArg {
    #[choice(name = "Rule of Thirds")]
    Thirds,
    Grid,
    Diagonal,
    Triangle,
    #[choice(name = "Golden Ratio")]
    GoldenRatio,
    #[choice(name = "Golden Spiral")]
    GoldenSpiral,
}

impl GuideArg {
    fn guide(self) -> CropGuide {
        match self {
            GuideArg::Thirds => CropGuide::Thirds,
            GuideArg::Grid => CropGuide::Grid,
            GuideArg::Diagonal => CropGuide::Diagonal,
            GuideArg::Triangle => CropGuide::Triangle,
            GuideArg::GoldenRatio => CropGuide::GoldenRatio,
            GuideArg::GoldenSpiral => CropGuide::GoldenSpiral,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ShowArg {
    Auto,
    Always,
    Never,
}

impl ShowArg {
    fn show(self) -> GuideShow {
        match self {
            ShowArg::Auto => GuideShow::Auto,
            ShowArg::Always => GuideShow::Always,
            ShowArg::Never => GuideShow::Never,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct OverlayArgs {
    /// The overlay the menu starts with; a pick in the menu replaces it until the arg changes.
    guide: GuideArg,
    show: ShowArg,
}

/// The overlay menu's state: the settings it edits and the args they were last set from.
#[derive(Clone, Debug)]
struct OverlayState {
    settings: CropSettings,
    from: (GuideArg, ShowArg),
}

/// The overlay button and its menu: six overlays and three show rules as check rows, Cycle Overlay
/// (O) and Cycle Orientation (⇧O, enabled for an overlay with more than one orientation). The
/// status line shows what the settings hold after a pick.
fn show_overlay_menu(ui: &mut Ui, args: &mut OverlayArgs) {
    let key = state_key(ui, "overlay");
    let wanted = (args.guide, args.show);
    let mut state: OverlayState =
        ui.data(|d| d.get_temp(key)).unwrap_or_else(|| OverlayState { settings: CropSettings::default(), from: (GuideArg::Grid, ShowArg::Never) });
    if state.from != wanted {
        state.from = wanted;
        state.settings.set_guide(args.guide.guide());
        state.settings.guide_show = args.show.show();
    }
    let labels = CropBarLabels::of(ui);
    ui.horizontal(|ui| {
        CropOverlayMenu::new("crop-overlay-story", &mut state.settings).labels(&labels).show(ui);
        let settings = &state.settings;
        status(ui, format!("{} · {:?} · orientation {}", settings.guide.name(), settings.guide_show, settings.guide_orientation));
    });
    ui.data_mut(|d| d.insert_temp(key, state));
}

// --- gear menu ---

#[derive(StoryArgs, Debug, PartialEq)]
struct GearArgs {
    /// Classic Mode on greys Show Cropped Area and Auto Center Preview.
    #[arg(name = "classic mode")]
    classic_mode: bool,
    /// Enable Crop Shield; off greys the rows under it.
    shield: bool,
}

/// What the gear story keeps: the settings, the args they were last set from and the count of
/// colour requests, which picks the sample colour.
#[derive(Clone, Debug)]
struct GearState {
    settings: CropSettings,
    from: (bool, bool),
    requests: usize,
}

/// The sample colours a Custom… request answers with: the story stands in for the colour dialog.
const SAMPLE_COLOURS: [[u8; 3]; 3] = [[200, 60, 60], [60, 120, 200], [240, 190, 60]];

/// The gear button and its popover. Custom… and the swatch ask the app for a colour; the story
/// answers with the next sample colour.
fn show_gear_menu(ui: &mut Ui, args: &mut GearArgs) {
    let key = state_key(ui, "gear");
    let wanted = (args.classic_mode, args.shield);
    let mut state: GearState =
        ui.data(|d| d.get_temp(key)).unwrap_or_else(|| GearState { settings: CropSettings::default(), from: (!wanted.0, !wanted.1), requests: 0 });
    if state.from != wanted {
        state.from = wanted;
        state.settings.shield.classic_mode = args.classic_mode;
        state.settings.shield.enabled = args.shield;
    }
    let labels = CropBarLabels::of(ui);
    let words = StoryLabels::of(ui);
    let mut last = None;
    ui.horizontal(|ui| {
        let out = CropOptionsMenu::new("crop-gear-story", &mut state.settings.shield).labels(&labels).show(ui);
        if out.pick_color {
            state.settings.shield.custom_color = SAMPLE_COLOURS[state.requests % SAMPLE_COLOURS.len()];
            state.requests += 1;
            last = Some("Crop Shield colour requested".to_owned());
        }
        let [r, g, b] = state.settings.shield.custom_color;
        let colour = match state.settings.shield.color {
            ShieldColor::MatchCanvas => labels.match_canvas.clone(),
            ShieldColor::Custom => format!("#{r:02x}{g:02x}{b:02x}"),
        };
        status(ui, format!("{colour} · {}", last.clone().unwrap_or_else(|| words.no_event.clone())));
    });
    ui.data_mut(|d| d.insert_temp(key, state));
}
