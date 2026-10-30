//! The Crop frame's stories: the Classic frame with its shield and guides, a held ratio, and
//! Straighten.

use egui::Ui;
use kurbo::{Rect, Size};

use super::{CropBox, CropFrame, CropLabels, CropNames};
use crate::canvas::story::{DOC, ViewArgs, document};
use crate::canvas::{CropGuide, CropShield, RulerUnits};
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::story::{Level, Story, StoryArgs, StoryChoice, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Crop frame"];

/// The frame before any gesture.
const START: Rect = Rect::new(120.0, 70.0, 420.0, 290.0);

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum GuideArg {
    None,
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
    fn guide(self) -> Option<(CropGuide, u8)> {
        let guide = match self {
            GuideArg::None => return None,
            GuideArg::Thirds => CropGuide::Thirds,
            GuideArg::Grid => CropGuide::Grid,
            GuideArg::Diagonal => CropGuide::Diagonal,
            GuideArg::Triangle => CropGuide::Triangle,
            GuideArg::GoldenRatio => CropGuide::GoldenRatio,
            GuideArg::GoldenSpiral => CropGuide::GoldenSpiral,
        };
        Some((guide, 0))
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RatioArg {
    #[choice(name = "1:1")]
    Square,
    #[choice(name = "4:3")]
    FourThree,
    #[choice(name = "3:2")]
    ThreeTwo,
    #[choice(name = "16:9")]
    SixteenNine,
    #[choice(name = "2:3")]
    TwoThree,
}

impl RatioArg {
    fn ratio(self) -> f64 {
        match self {
            RatioArg::Square => 1.0,
            RatioArg::FourThree => 4.0 / 3.0,
            RatioArg::ThreeTwo => 3.0 / 2.0,
            RatioArg::SixteenNine => 16.0 / 9.0,
            RatioArg::TwoThree => 2.0 / 3.0,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct UprightArgs {
    #[arg(flatten)]
    view: ViewArgs,
    guide: GuideArg,
    #[arg(range = 0.0..=1.0)]
    shield: f32,
    readout: bool,
}

impl Default for UprightArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), guide: GuideArg::Thirds, shield: 0.75, readout: true }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct RatioArgs {
    #[arg(flatten)]
    view: ViewArgs,
    ratio: RatioArg,
}

impl Default for RatioArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), ratio: RatioArg::ThreeTwo }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StraightenArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// One line: the story clears it once the line has turned the frame.
    armed: bool,
}

impl Default for StraightenArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), armed: true }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/crop-frame/upright", Level::Primitives, "Crop frame", "Upright", UprightArgs::default(), show_upright)
            .on(Surface::Canvas),
        typed_story!("primitives/crop-frame/ratio", Level::Primitives, "Crop frame", "Ratio", RatioArgs::default(), show_ratio).on(Surface::Canvas),
        typed_story!("primitives/crop-frame/straighten", Level::Primitives, "Crop frame", "Straighten", StraightenArgs::default(), show_straighten)
            .on(Surface::Canvas),
    ]
}

/// The English names the stories use.
fn english_names() -> CropNames {
    CropNames {
        group: "Crop frame".into(),
        whole: "Whole frame".into(),
        corners: ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"].map(String::from),
        edges: ["Top edge", "Right edge", "Bottom edge", "Left edge"].map(String::from),
    }
}

/// What a story keeps between frames: the frame, and the frame a running drag started from.
#[derive(Clone, Copy)]
struct Kept {
    frame: CropBox,
    before: Option<CropBox>,
}

/// Draws the story canvas and the frame over it, applies the frame's events to the kept frame
/// and returns the response with the frame as it is now.
fn frame_on_canvas(
    ui: &mut Ui,
    view: ViewArgs,
    configure: impl for<'a> FnOnce(CropFrame<'a>) -> CropFrame<'a>,
) -> (super::CropResponse, Kept, egui::Id) {
    let canvas = document(ui, view, "Document");
    let key = ui.scope_id().with("crop-frame-story");
    let mut kept: Kept = ui.data(|d| d.get_temp(key)).unwrap_or(Kept { frame: CropBox::upright(START), before: None });
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let frame = kept.frame;
    let r = configure(CropFrame::new("story-crop-frame", &frame, english_names())).units(&px, &px).show(ui, &canvas.response, &canvas.view);
    match &r.event {
        Some(EditEvent::Begin { .. }) => kept.before = Some(kept.frame),
        Some(EditEvent::Update { value, .. }) => kept.frame = *value,
        Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
            kept.before = None;
            kept.frame = *value;
        }
        Some(EditEvent::Cancel { .. }) => {
            if let Some(before) = kept.before.take() {
                kept.frame = before;
            }
        }
        None => {}
    }
    if r.apply || r.discard {
        kept = Kept { frame: CropBox::upright(START), before: None };
    }
    (r, kept, key)
}

fn labels() -> CropLabels {
    CropLabels::new("W:", "H:", "Angle:")
}

/// The Classic frame over the document: drag inside to move, an edge or corner to resize, outside
/// to turn (⇧ in steps), X to swap, Enter or a double-click to apply, Esc to discard. The guide
/// and the shield's opacity are args.
fn show_upright(ui: &mut Ui, args: &mut UprightArgs) {
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let shield = CropShield { opacity: args.shield, ..CropShield::default() }.fill(canvas_colour, false);
    let (guide, readout) = (args.guide.guide(), args.readout);
    let (_, kept, key) = frame_on_canvas(ui, args.view, |f| {
        let f = f.shield(shield).guide(guide);
        if readout { f.readout(labels()) } else { f }
    });
    ui.data_mut(|d| d.insert_temp(key, kept));
}

/// A frame whose gestures all hold the width over height of the `ratio` arg.
fn show_ratio(ui: &mut Ui, args: &mut RatioArgs) {
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let shield = CropShield::default().fill(canvas_colour, false);
    let ratio = args.ratio.ratio();
    let (_, kept, key) =
        frame_on_canvas(ui, args.view, |f| f.shield(shield).guide(Some((CropGuide::Thirds, 0))).ratio(Some(ratio)).readout(labels()));
    ui.data_mut(|d| d.insert_temp(key, kept));
}

/// With `armed` on, a drag draws a line along a horizon or a wall; its release turns the frame to
/// level it and fits the frame to the document (`CropBox::fitted`) as one step, and the story
/// clears `armed`.
fn show_straighten(ui: &mut Ui, args: &mut StraightenArgs) {
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let shield = CropShield::default().fill(canvas_colour, false);
    let armed = args.armed;
    let (r, mut kept, key) = frame_on_canvas(ui, args.view, |f| f.shield(shield).straighten(armed).readout(labels()));
    if let Some(angle) = r.straightened {
        if let Some(fitted) = kept.frame.fitted(Size::new(DOC.width(), DOC.height()), angle) {
            kept.frame = fitted;
        }
        args.armed = false;
    }
    ui.data_mut(|d| d.insert_temp(key, kept));
}
