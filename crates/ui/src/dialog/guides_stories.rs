//! Stories of the guide dialogs: New Guide, Edit Selected Guides and New Guide Layout. Each
//! opens on a [`DialogHost`](super::DialogHost) like the framework stories. The colour picker is
//! the story's: a request is answered at once with a fixed colour, so Custom… and the swatch
//! finish without an app. Guide Layout draws its live preview on a canvas beside the dialog.

use craft_theme::Rgba;
use egui::Ui;

use super::{GuideLayoutDialog, GuideLines, GuideRect, GuideUnits, NewGuideDialog};
use crate::canvas::story::{self as canvas_story, ViewArgs};
use crate::canvas::{Guide, GuideNames, Guides, Orientation, RulerUnits};
use crate::localized::units::LengthUnit;
use crate::localized::{Labels as _, LocaleNumbers};
use crate::story::drive;
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RulerArg {
    Pixels,
    Inches,
    Millimetres,
}

impl RulerArg {
    fn unit(self) -> LengthUnit {
        match self {
            RulerArg::Pixels => LengthUnit::Pixels,
            RulerArg::Inches => LengthUnit::Inches,
            RulerArg::Millimetres => LengthUnit::Millimetres,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct NewGuideArgs {
    open: bool,
    ruler: RulerArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct EditGuidesArgs {
    open: bool,
    ruler: RulerArg,
    #[arg(name = "horizontal guide")]
    horizontal: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct GuideLayoutArgs {
    open: bool,
    ruler: RulerArg,
    #[arg(name = "existing guides")]
    existing: bool,
    artboards: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/dialog/new-guide",
            Level::Patterns,
            "Dialog",
            "New guide",
            NewGuideArgs { open: true, ruler: RulerArg::Pixels },
            show_new_guide
        ),
        typed_story!(
            "patterns/dialog/edit-guides",
            Level::Patterns,
            "Dialog",
            "Edit selected guides",
            EditGuidesArgs { open: true, ruler: RulerArg::Pixels, horizontal: false },
            show_edit_guides
        ),
        typed_story!(
            "patterns/dialog/guide-layout",
            Level::Patterns,
            "Dialog",
            "New guide layout",
            GuideLayoutArgs { open: true, ruler: RulerArg::Pixels, existing: true, artboards: false },
            show_guide_layout
        ),
    ]
}

/// The story document: 640 × 400 pixels, as the canvas story draws it.
const DOCUMENT: GuideRect = GuideRect { x0: 0.0, y0: 0.0, x1: 640.0, y1: 400.0 };

fn units(ruler: RulerArg) -> GuideUnits {
    GuideUnits { ruler: ruler.unit(), width_px: DOCUMENT.x1, height_px: DOCUMENT.y1, pixels_per_inch: 96.0, points_per_inch: 72.0 }
}

/// The colour the story's picker answers with.
fn picked_color() -> craft_theme::Srgba {
    Rgba::rgb(0xe0, 0x6c, 0x1f).into()
}

/// New Guide: Orientation, Position in the ruler unit, Color with Custom… answered by the story.
fn show_new_guide(ui: &mut Ui, args: &mut NewGuideArgs) {
    let (ruler, words) = (args.ruler, StoryLabels::of(ui));
    drive(
        ui,
        "new-guide",
        &mut args.open,
        ruler,
        || NewGuideDialog::new(units(ruler)),
        |dialog| {
            if let Some(request) = dialog.pick_color() {
                dialog.set_color(&request.key, picked_color());
            }
        },
        |answer| {
            let value = format!("{:?} {:.2} px, {:?}", answer.orientation, answer.position_px, answer.color);
            craft_i18n::fmt(&words.result_ok, &[("value", &value)])
        },
    );
}

/// Edit Selected Guides: the same dialog over a guide that already exists.
fn show_edit_guides(ui: &mut Ui, args: &mut EditGuidesArgs) {
    let (ruler, horizontal, words) = (args.ruler, args.horizontal, StoryLabels::of(ui));
    drive(
        ui,
        "edit-guides",
        &mut args.open,
        (ruler, horizontal),
        || {
            let guide = if horizontal { Guide::new(7, Orientation::Horizontal, 150.0) } else { Guide::new(7, Orientation::Vertical, 240.0) };
            NewGuideDialog::edit(&guide, [0, 255, 255], units(ruler))
        },
        |dialog| {
            if let Some(request) = dialog.pick_color() {
                dialog.set_color(&request.key, picked_color());
            }
        },
        |answer| {
            let value = format!("{:?} {:.2} px, {:?}, edited {:?}", answer.orientation, answer.position_px, answer.color, answer.edited);
            craft_i18n::fmt(&words.result_ok, &[("value", &value)])
        },
    );
}

/// The two artboards the layout can target.
const ARTBOARDS: [GuideRect; 2] = [GuideRect { x0: 40.0, y0: 40.0, x1: 300.0, y1: 360.0 }, GuideRect { x0: 340.0, y0: 40.0, x1: 600.0, y1: 360.0 }];

/// New Guide Layout with its live preview drawn on the story canvas: the dialog's guides while
/// it is open, the document's own guides otherwise.
fn show_guide_layout(ui: &mut Ui, args: &mut GuideLayoutArgs) {
    let (ruler, existing, artboards, words) = (args.ruler, args.existing, args.artboards, StoryLabels::of(ui));
    let lines = if existing { GuideLines { vertical: vec![320.0], horizontal: vec![200.0] } } else { GuideLines::default() };
    let mut preview = None;
    drive(
        ui,
        "guide-layout",
        &mut args.open,
        (ruler, existing, artboards),
        || {
            let dialog = GuideLayoutDialog::new(DOCUMENT, lines.clone(), units(ruler), [0, 255, 255]);
            if artboards { dialog.artboards(ARTBOARDS.to_vec()) } else { dialog }
        },
        |dialog| {
            if let Some(request) = dialog.pick_color() {
                dialog.set_color(&request.key, picked_color());
            }
            preview = dialog.preview();
        },
        |answer| {
            let value = format!("{} + {}", answer.lines.all.vertical.len(), answer.lines.all.horizontal.len());
            craft_i18n::fmt(&words.result_ok, &[("value", &value)])
        },
    );
    let shown = match &preview {
        Some(preview) => (preview.lines.all.clone(), Some(preview.color)),
        None => (if existing { GuideLines { vertical: vec![320.0], horizontal: vec![200.0] } } else { GuideLines::default() }, None),
    };
    let canvas = canvas_story::document(ui, ViewArgs::default(), &StoryLabels::of(ui).document);
    let guides = shown.0.guides(1, shown.1.unwrap_or(egui::Color32::from_rgb(0, 255, 255)));
    let numbers = LocaleNumbers::of(ui);
    let units = RulerUnits::pixels((*numbers).clone());
    let _ = Guides::new("guide-layout-story", &guides, &units, &units, GuideNames::of(ui)).bounds(canvas_story::DOC).show(
        ui,
        &canvas.response,
        &canvas.view,
    );
}
