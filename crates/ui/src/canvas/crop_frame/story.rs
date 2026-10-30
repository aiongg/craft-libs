//! The Crop frame's stories: the Classic frame with its shield and guides, a held ratio, and
//! Straighten.

use egui::Ui;
use kurbo::{Rect, Size};

use super::{CropBox, CropFrame, CropLabels, CropNames};
use crate::canvas::CanvasView;
use crate::canvas::story::{DOC, ViewArgs, document, document_with};
use crate::canvas::{CropGuide, CropShield, RulerUnits};
use crate::edit::EditEvent;
use crate::localized::{Labels as _, LocaleNumbers};
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

#[derive(StoryArgs, Debug, PartialEq)]
struct ImageModeArgs {
    auto_center: bool,
    /// The frame's turn in degrees; the view turns the other way.
    #[arg(range = -45.0..=45.0)]
    angle: f64,
    /// One line: the story clears it once the line has turned the frame.
    armed: bool,
}

impl Default for ImageModeArgs {
    fn default() -> Self {
        Self { auto_center: true, angle: 0.0, armed: false }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/crop-frame/upright", Level::Primitives, "Crop frame", "Upright", UprightArgs::default(), show_upright)
            .on(Surface::Canvas),
        typed_story!("primitives/crop-frame/ratio", Level::Primitives, "Crop frame", "Ratio", RatioArgs::default(), show_ratio).on(Surface::Canvas),
        typed_story!("primitives/crop-frame/straighten", Level::Primitives, "Crop frame", "Straighten", StraightenArgs::default(), show_straighten)
            .on(Surface::Canvas),
        typed_story!("primitives/crop-frame/image-mode", Level::Primitives, "Crop frame", "Image mode", ImageModeArgs::default(), show_image_mode)
            .on(Surface::Canvas),
    ]
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
    let r = configure(CropFrame::new("story-crop-frame", &frame, CropNames::of(ui))).units(&px, &px).show(ui, &canvas.response, &canvas.view);
    keep_event(&mut kept, &r.event);
    if r.apply || r.discard {
        kept = Kept { frame: CropBox::upright(START), before: None };
    }
    (r, kept, key)
}

/// Applies the frame's event to what the story keeps; a cancel puts back the frame at the Begin.
fn keep_event(kept: &mut Kept, event: &Option<EditEvent<CropBox>>) {
    match event {
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
}

/// The Classic frame over the document: drag inside to move, an edge or corner to resize, outside
/// to turn (⇧ in steps), X to swap, Enter or a double-click to apply, Esc to discard. The guide
/// and the shield's opacity are args.
fn show_upright(ui: &mut Ui, args: &mut UprightArgs) {
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let shield = CropShield { opacity: args.shield, ..CropShield::default() }.fill(canvas_colour, false);
    let (guide, readout, labels) = (args.guide.guide(), args.readout, CropLabels::of(ui));
    let (_, kept, key) = frame_on_canvas(ui, args.view, |f| {
        let f = f.shield(shield).guide(guide);
        if readout { f.readout(labels) } else { f }
    });
    ui.data_mut(|d| d.insert_temp(key, kept));
}

/// A frame whose gestures all hold the width over height of the `ratio` arg.
fn show_ratio(ui: &mut Ui, args: &mut RatioArgs) {
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let shield = CropShield::default().fill(canvas_colour, false);
    let (ratio, labels) = (args.ratio.ratio(), CropLabels::of(ui));
    let (_, kept, key) = frame_on_canvas(ui, args.view, |f| f.shield(shield).guide(Some((CropGuide::Thirds, 0))).ratio(Some(ratio)).readout(labels));
    ui.data_mut(|d| d.insert_temp(key, kept));
}

/// With `armed` on, a drag draws a line along a horizon or a wall; its release turns the frame to
/// level it and fits the frame to the document (`CropBox::fitted`) as one step, and the story
/// clears `armed`.
fn show_straighten(ui: &mut Ui, args: &mut StraightenArgs) {
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let shield = CropShield::default().fill(canvas_colour, false);
    let (armed, labels) = (args.armed, CropLabels::of(ui));
    let (r, mut kept, key) = frame_on_canvas(ui, args.view, |f| f.shield(shield).straighten(armed).readout(labels));
    if let Some(angle) = r.straightened {
        if let Some(fitted) = kept.frame.fitted(Size::new(DOC.width(), DOC.height()), angle) {
            kept.frame = fitted;
        }
        args.armed = false;
    }
    ui.data_mut(|d| d.insert_temp(key, kept));
}

/// What the Image-mode story keeps: the frame and the view that moves with it, and the args the
/// view was last synced for.
#[derive(Clone, Copy)]
struct ImageKept {
    kept: Kept,
    view: CanvasView,
    synced: (f64, bool),
}

/// The frame stays upright on screen and the gestures move and turn the image behind it: drag
/// inside to move the image, outside to turn it, a handle to resize. The story keeps its view
/// across passes and syncs it to the frame when the `angle` or `auto_center` arg changes, and
/// after a Straighten.
fn show_image_mode(ui: &mut Ui, args: &mut ImageModeArgs) {
    let canvas_colour = Tokens::of(ui).color.surface.canvas.to_egui();
    let shield = CropShield::default().fill(canvas_colour, false);
    let key = ui.scope_id().with("crop-frame-image-story");
    let saved: Option<ImageKept> = ui.data(|d| d.get_temp(key));
    let canvas = document_with(ui, ViewArgs::default(), "Document", 0.0, |rect, ppp| {
        let base = saved.map_or_else(|| CanvasView::new(rect, DOC.center(), 1.0 / ppp), |s| s.view);
        CanvasView { rect, ..base }
    });
    let mut view = canvas.view;
    let fresh = CropBox { rect: START, angle: args.angle };
    let mut s = saved.unwrap_or(ImageKept { kept: Kept { frame: fresh, before: None }, view, synced: (f64::NAN, false) });
    if s.synced != (args.angle, args.auto_center) {
        s.kept = Kept { frame: fresh, before: None };
        CropFrame::sync_view(&mut view, &s.kept.frame);
        s.synced = (args.angle, args.auto_center);
    }
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let frame = s.kept.frame;
    let r = CropFrame::new("story-crop-frame", &frame, CropNames::of(ui))
        .units(&px, &px)
        .shield(shield)
        .straighten(args.armed)
        .auto_center(args.auto_center)
        .readout(CropLabels::of(ui))
        .show_image(ui, &canvas.response, &mut view);
    keep_event(&mut s.kept, &r.event);
    if let Some(angle) = r.straightened {
        if let Some(fitted) = s.kept.frame.fitted(Size::new(DOC.width(), DOC.height()), angle) {
            s.kept.frame = fitted;
            CropFrame::sync_view(&mut view, &fitted);
        }
        args.armed = false;
    }
    if r.apply || r.discard {
        s.kept = Kept { frame: fresh, before: None };
        CropFrame::sync_view(&mut view, &s.kept.frame);
    }
    s.view = view;
    ui.data_mut(|d| d.insert_temp(key, s));
}
