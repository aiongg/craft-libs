//! `specimens/icons/compare`: every icon in every candidate set, side by side.

use craft_theme::{Mode, TypeRole};
use egui::{Align2, Frame, Rect, Sense, Ui, vec2};

use super::{Icon, IconSet, IconSize, paint_in};
use crate::story::{Level, Story, StoryArgs, StoryChoice, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, scope};
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SetArg {
    All,
    Lucide,
    Phosphor,
    Carbon,
    Tabler,
}

impl SetArg {
    /// The set to show, or `None` for every set.
    fn set(self) -> Option<IconSet> {
        match self {
            SetArg::All => None,
            SetArg::Lucide => Some(IconSet::Lucide),
            SetArg::Phosphor => Some(IconSet::Phosphor),
            SetArg::Carbon => Some(IconSet::Carbon),
            SetArg::Tabler => Some(IconSet::Tabler),
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SizesArg {
    #[choice(name = "16 and 20")]
    Both,
    #[choice(name = "16")]
    Sixteen,
    #[choice(name = "20")]
    Twenty,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    Inherit,
    Light,
    Dark,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CompareArgs {
    set: SetArg,
    size: SizesArg,
    mode: ModeArg,
}

const NAME_WIDTH: f32 = 176.0;
const CELL: f32 = 36.0;

/// The icon module's stories.
pub fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "specimens/icons/compare",
            Level::Specimens,
            "Icons",
            "Compare",
            CompareArgs { set: SetArg::All, size: SizesArg::Both, mode: ModeArg::Inherit },
            show
        )
        .on(Surface::Own),
    ]
}

fn show(ui: &mut Ui, args: &mut CompareArgs) {
    let mode = match args.mode {
        ModeArg::Light => Some(Mode::Light),
        ModeArg::Dark => Some(Mode::Dark),
        ModeArg::Inherit => None,
    };
    let sets: Vec<IconSet> = IconSet::ALL.into_iter().filter(|&set| args.set.set().is_none_or(|only| only == set)).collect();
    let sizes: &[f32] = match args.size {
        SizesArg::Sixteen => &[16.0],
        SizesArg::Twenty => &[20.0],
        SizesArg::Both => &[16.0, 20.0],
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
