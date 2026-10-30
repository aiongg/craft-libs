//! `foundations/colours`: every semantic colour token as a swatch.

use craft_theme::{ColorToken, Rgba, TypeRole};
use egui::{Align2, Sense, Stroke, StrokeKind, Ui, vec2};

use super::{Level, Story, StoryArgs, StoryChoice};
use crate::theme::{ToEgui, Tokens, TokensOf};
use crate::typed_story;

/// The groups of [`ColorTokens`](craft_theme::ColorTokens): the first segment of the token paths.
#[derive(StoryChoice, Clone, Copy)]
pub(super) enum Group {
    All,
    Surface,
    Text,
    Line,
    Accent,
    Status,
    State,
    Overlay,
    Data,
    Caption,
    Edge,
}

#[derive(StoryArgs)]
struct ColourArgs {
    group: Group,
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Colours"];

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!("foundations/colours/semantic", Level::Foundations, "Colours", "Semantic", ColourArgs { group: Group::All }, show)]
}

fn show(ui: &mut Ui, args: &mut ColourArgs) {
    let tokens = Tokens::of(ui);
    let group = args.group.name();
    let mut last = "";
    for token in ColorToken::ALL {
        let path = token.path();
        let section = path.split('.').next().unwrap_or(path);
        if group != "all" && section != group {
            continue;
        }
        if section != last {
            last = section;
            ui.add_space(8.0);
            ui.label(egui::RichText::new(section).font(tokens.font(TypeRole::BodyStrong)));
        }
        swatch(ui, path, tokens.color(*token));
    }
}

/// A row: the colour, its path and its hex value.
fn swatch(ui: &mut Ui, path: &str, colour: Rgba) {
    let tokens = Tokens::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(420.0), 24.0), Sense::hover());
    let painter = ui.painter();
    let chip = egui::Rect::from_min_size(rect.min, vec2(48.0, rect.height()));
    painter.rect_filled(chip, tokens.radius.control, colour.to_egui());
    painter.rect_stroke(chip, tokens.radius.control, Stroke::new(1.0, tokens.color.line.border.to_egui()), StrokeKind::Inside);
    let text = tokens.color.text.normal.to_egui();
    painter.text(chip.right_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, path, tokens.font(TypeRole::Label), text);
    let [r, g, b, a] = colour.0;
    let hex = if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("#{r:02x}{g:02x}{b:02x}{a:02x}") };
    painter.text(rect.right_center(), Align2::RIGHT_CENTER, hex, tokens.font(TypeRole::Mono), tokens.color.text.muted.to_egui());
}
