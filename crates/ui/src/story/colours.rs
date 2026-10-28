//! `foundations/colours`: every semantic colour token as a swatch.

use craft_theme::{ColorToken, Rgba, TypeRole};
use egui::{Align2, Sense, Stroke, StrokeKind, Ui, vec2};

use super::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf};

/// The groups of [`ColorTokens`](craft_theme::ColorTokens): the first segment of the token paths.
pub(super) const GROUPS: &[&str] = &["all", "surface", "text", "line", "accent", "status", "state", "overlay", "data", "caption", "edge"];

pub(super) fn stories() -> Vec<Story> {
    vec![Story {
        id: "foundations/colours/semantic",
        level: Level::Foundations,
        component: "Colours",
        name: "Semantic",
        args: || Args::new().with_choice("group", GROUPS, 0),
        show,
    }]
}

fn show(ui: &mut Ui, args: &mut Args) {
    let tokens = Tokens::of(ui);
    let group = args.choice("group");
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
