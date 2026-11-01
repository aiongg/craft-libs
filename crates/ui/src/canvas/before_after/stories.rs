//! Stories of the before and after view (task KS1): not registered. Registering them (task RG2)
//! adds `stories()` to the canvas stories, `COMPONENTS` to the component list and
//! [`BeforeAfterStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The two pictures are fakes drawn by the story (a gradient and the same gradient toned), so
//! every mode shows without a renderer. The host writes the `fraction` event's value back each
//! pass. The layout follows the gallery's interface direction, so the right-to-left layout shows
//! when the gallery is switched. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/before-after/modes` | The six modes over the two fakes, captions on and off |
//! | `primitives/before-after/divider` | A Split and a SplitStacked view with the fraction readout, the keys, `drag_anywhere` on and off, hold Backslash for the original |

#![expect(dead_code, reason = "registered by task RG2")]

use egui::{Color32, Key, Mesh, Painter, Rect, Shape, Ui, vec2};

use super::{BeforeAfter, BeforeAfterMode, BeforeAfterPane, BeforeAfterSide};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryChoice, Surface};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["BeforeAfter"];

crate::words! {
    /// The captions of the before and after stories, English by default.
    #[non_exhaustive]
    pub(crate) struct BeforeAfterStoryLabels {
        fraction: "beforeAfterStory.fraction" = "Divider at {n}%";
        shown: "beforeAfterStory.shown" = "Shown: {mode}";
        original: "beforeAfterStory.original" = "Original";
        after: "beforeAfterStory.after" = "After";
        side_by_side: "beforeAfterStory.sideBySide" = "Side by side";
        stacked: "beforeAfterStory.stacked" = "Stacked";
        split: "beforeAfterStory.split" = "Split";
        split_stacked: "beforeAfterStory.splitStacked" = "Split stacked";
        hold: "beforeAfterStory.hold" = "Hold Backslash to show the original.";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/before-after/modes",
            Level::Primitives,
            "BeforeAfter",
            "Modes",
            ModesArgs { mode: ModeArg::SideBySide, captions: true },
            show_modes
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/before-after/divider",
            Level::Primitives,
            "BeforeAfter",
            "Divider",
            DividerArgs { stacked: false, drag_anywhere: true, captions: false },
            show_divider
        )
        .on(Surface::Canvas),
    ]
}

/// The area the view gets and the picture inside it, in points: a story-local size.
const AREA: [f32; 2] = [520.0, 300.0];
const PICTURE: [f32; 2] = [200.0, 140.0];
/// Strength of the tone the "after" fake adds to the gradient.
const TONE: f32 = 0.45;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    After,
    Original,
    #[choice(name = "side by side")]
    SideBySide,
    Stacked,
    Split,
    #[choice(name = "split stacked")]
    SplitStacked,
}

impl ModeArg {
    fn mode(self) -> BeforeAfterMode {
        match self {
            ModeArg::After => BeforeAfterMode::After,
            ModeArg::Original => BeforeAfterMode::Original,
            ModeArg::SideBySide => BeforeAfterMode::SideBySide,
            ModeArg::Stacked => BeforeAfterMode::Stacked,
            ModeArg::Split => BeforeAfterMode::Split,
            ModeArg::SplitStacked => BeforeAfterMode::SplitStacked,
        }
    }
}

/// Paints a fake picture in `rect`: a diagonal gradient between two theme colours, toned toward
/// the accent for the "after" side.
fn paint_fake(painter: &Painter, tokens: &crate::theme::UiTokens, pane: &BeforeAfterPane) {
    let (from, to) = (tokens.color.surface.canvas.to_egui(), tokens.color.text.normal.to_egui());
    let tone = tokens.color.accent.solid.to_egui();
    let blend = |c: Color32| if pane.side == BeforeAfterSide::After { c.lerp_to_gamma(tone, TONE) } else { c };
    let corners = [(pane.image.left_top(), 0.0), (pane.image.right_top(), 0.5), (pane.image.right_bottom(), 1.0), (pane.image.left_bottom(), 0.5)];
    let mut mesh = Mesh::default();
    for (pos, t) in corners {
        mesh.colored_vertex(pos, blend(from.lerp_to_gamma(to, t)));
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn picture_in(area: Rect) -> Rect {
    Rect::from_center_size(area.center(), vec2(PICTURE[0], PICTURE[1]))
}

fn mode_name(mode: BeforeAfterMode, own: &BeforeAfterStoryLabels) -> &str {
    match mode {
        BeforeAfterMode::After => own.after.as_str(),
        BeforeAfterMode::Original => own.original.as_str(),
        BeforeAfterMode::SideBySide => own.side_by_side.as_str(),
        BeforeAfterMode::Stacked => own.stacked.as_str(),
        BeforeAfterMode::Split => own.split.as_str(),
        BeforeAfterMode::SplitStacked => own.split_stacked.as_str(),
    }
}

// --- modes ---

#[derive(StoryArgs, Debug, PartialEq)]
struct ModesArgs {
    mode: ModeArg,
    captions: bool,
}

/// One view in the chosen mode over the fakes. The divider modes can be dragged and keyed here too.
fn show_modes(ui: &mut Ui, args: &mut ModesArgs) {
    let own = BeforeAfterStoryLabels::of(ui);
    let key = ui.make_persistent_id("before-after-modes-story");
    let mut fraction: f32 = ui.data(|d| d.get_temp(key)).unwrap_or(0.5);
    let (area, _) = ui.allocate_exact_size(vec2(AREA[0], AREA[1]), egui::Sense::hover());
    let tokens = Tokens::of(ui);
    let view = BeforeAfter::new("before-after-modes-story", args.mode.mode()).fraction(fraction).captions(args.captions);
    // The picture sits in the middle of the after pane.
    let after = view.panes(ui, area)[1];
    let out = view.show(ui, area, picture_in(after), |painter, pane| paint_fake(painter, &tokens, pane));
    if let Some(event) = &out.fraction
        && let Some(value) = event.value()
    {
        fraction = *value;
    }
    let line = own.shown.replace("{mode}", mode_name(out.mode, &own));
    ui.add(Text::new(line).content(Content::Muted));
    ui.data_mut(|d| d.insert_temp(key, fraction));
}

// --- divider ---

#[derive(StoryArgs, Debug, PartialEq)]
struct DividerArgs {
    stacked: bool,
    #[arg(name = "drag anywhere")]
    drag_anywhere: bool,
    captions: bool,
}

/// A Split (or SplitStacked) view whose divider is dragged, keyed (arrows, Home, End) or clicked
/// to; Backslash held shows the original. The readout is the fraction the host stores.
fn show_divider(ui: &mut Ui, args: &mut DividerArgs) {
    let own = BeforeAfterStoryLabels::of(ui);
    let key = ui.make_persistent_id("before-after-divider-story");
    let mut fraction: f32 = ui.data(|d| d.get_temp(key)).unwrap_or(0.5);
    let (area, _) = ui.allocate_exact_size(vec2(AREA[0], AREA[1]), egui::Sense::hover());
    let tokens = Tokens::of(ui);
    let mode = if args.stacked { BeforeAfterMode::SplitStacked } else { BeforeAfterMode::Split };
    let view = BeforeAfter::new("before-after-divider-story", mode)
        .fraction(fraction)
        .captions(args.captions)
        .drag_anywhere(args.drag_anywhere)
        .original_while(Key::Backslash);
    let after = view.panes(ui, area)[1];
    let out = view.show(ui, area, picture_in(after), |painter, pane| paint_fake(painter, &tokens, pane));
    if let Some(event) = &out.fraction
        && let Some(value) = event.value()
    {
        fraction = *value;
    }
    let numbers = LocaleNumbers::of(ui);
    let percent = numbers.format(f64::from((fraction * 100.0).round()), 0..=0);
    ui.add(Text::new(own.fraction.replace("{n}", &percent)).content(Content::Muted));
    ui.add(Text::new(own.hold.clone()).content(Content::Muted));
    ui.data_mut(|d| d.insert_temp(key, fraction));
}
