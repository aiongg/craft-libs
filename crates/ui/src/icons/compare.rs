//! `specimens/icons/compare`: every icon in every candidate set, side by side.

use craft_theme::{Mode, TypeRole};
use egui::{Align2, Frame, Rect, Sense, Ui, vec2};

use super::{Icon, IconSet, IconSize, paint_in};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, scope};

const SETS: &[&str] = &["all", "lucide", "phosphor", "carbon", "tabler"];
const SIZES: &[&str] = &["16 and 20", "16", "20"];
const MODES: &[&str] = &["inherit", "light", "dark"];

const NAME_WIDTH: f32 = 176.0;
const CELL: f32 = 36.0;

/// The icon module's stories.
pub fn stories() -> Vec<Story> {
    vec![Story::new(
        "specimens/icons/compare",
        Level::Specimens,
        "Icons",
        "Compare",
        || Args::new().with_choice("set", SETS, 0).with_choice("size", SIZES, 0).with_choice("mode", MODES, 0),
        show,
    )]
}

fn show(ui: &mut Ui, args: &mut Args) {
    let mode = match args.choice("mode") {
        "light" => Some(Mode::Light),
        "dark" => Some(Mode::Dark),
        _ => None,
    };
    let filter = args.choice("set");
    let sets: Vec<IconSet> = IconSet::ALL.into_iter().filter(|set| filter == "all" || set.id() == filter).collect();
    let sizes: &[f32] = match args.choice("size") {
        "16" => &[16.0],
        "20" => &[20.0],
        _ => &[16.0, 20.0],
    };
    scope(ui, |o| o.mode = mode, |ui| table(ui, &sets, sizes));
}

fn table(ui: &mut Ui, sets: &[IconSet], sizes: &[f32]) {
    let tokens = Tokens::of(ui);
    let text = tokens.color.text.normal.to_egui();
    let muted = tokens.color.text.muted.to_egui();
    let label = tokens.font(TypeRole::Label);
    Frame::new().fill(tokens.color.surface.panel.to_egui()).inner_margin(12.0).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let width = NAME_WIDTH + CELL * (sets.len() * sizes.len()) as f32;
        let (header, _) = ui.allocate_exact_size(vec2(width, 2.0 * CELL), Sense::hover());
        let painter = ui.painter();
        for (index, set) in sets.iter().enumerate() {
            let left = header.left() + NAME_WIDTH + CELL * (index * sizes.len()) as f32;
            let top = Rect::from_min_size(egui::pos2(left, header.top()), vec2(CELL * sizes.len() as f32, CELL));
            painter.text(top.center_bottom(), Align2::CENTER_BOTTOM, short_title(*set), label.clone(), text);
            for (column, size) in sizes.iter().enumerate() {
                let x = left + CELL * (column as f32 + 0.5);
                painter.text(egui::pos2(x, header.bottom() - 4.0), Align2::CENTER_BOTTOM, format!("{size}"), label.clone(), muted);
            }
        }
        for icon in Icon::ALL {
            let (row, _) = ui.allocate_exact_size(vec2(width, CELL), Sense::hover());
            let name = if icon.directional() { format!("{} (mirrors)", icon.name()) } else { icon.name().to_owned() };
            ui.painter().text(row.left_center(), Align2::LEFT_CENTER, name, label.clone(), text);
            for (index, set) in sets.iter().enumerate() {
                for (column, size) in sizes.iter().enumerate() {
                    let left = row.left() + NAME_WIDTH + CELL * (index * sizes.len() + column) as f32;
                    let cell = Rect::from_min_size(egui::pos2(left, row.top()), vec2(CELL, CELL));
                    paint_in(ui, cell, icon, *set, *size, IconSize::Md.stroke(), text);
                }
            }
        }
        ui.add_space(8.0);
        let licences: Vec<String> = IconSet::ALL.iter().map(|set| format!("{}: {}", set.title(), set.licence())).collect();
        ui.label(egui::RichText::new(licences.join(" · ")).font(tokens.font(TypeRole::Caption)).color(muted));
    });
}

/// The set's name without its weight, to fit the column header.
fn short_title(set: IconSet) -> &'static str {
    let title = set.title();
    title.split(" (").next().unwrap_or(title)
}
