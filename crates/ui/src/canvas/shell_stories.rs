//! Stories of the selection bar and the tool pill.

use egui::{Rect, Ui, vec2};

use super::{PillPlace, SelectionBar, ToolPill};
use crate::controls::IconButton;
use crate::icons::Icon;
use crate::patterns::rail::tool::story_slots;
use crate::story::{Level, Story, StoryArgs, Surface};
use crate::typed_story;

/// The components these stories cover.
pub const COMPONENTS: &[&str] = &["SelectionBar", "ToolPill"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/canvas/selection-bar", Level::Patterns, "SelectionBar", "Bar", BarArgs { above: true }, show_bar).on(Surface::Canvas),
        typed_story!("patterns/canvas/tool-pill", Level::Patterns, "ToolPill", "Pill", PillArgs { top: false }, show_pill).on(Surface::Canvas),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    /// Above the selection; off puts it below (the bar flips when it has no room).
    above: bool,
}

/// The bar over a rect selection in the middle of a canvas area.
fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let canvas = ui.allocate_space(vec2(ui.available_width().min(560.0), 300.0)).1;
    let selection = Rect::from_center_size(canvas.center(), vec2(180.0, 100.0));
    ui.painter().rect_stroke(selection, 0.0, ui.visuals().widgets.active.fg_stroke, egui::StrokeKind::Inside);
    SelectionBar::new("selection-bar-story").above(args.above).show(ui, canvas, Some(selection), |ui| {
        ui.add(IconButton::new(Icon::Copy, "Copy"));
        ui.add(IconButton::new(Icon::Trash, "Delete"));
    });
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PillArgs {
    /// At the top centre; off is the bottom centre.
    top: bool,
}

/// The pill at the bottom centre of a canvas area.
fn show_pill(ui: &mut Ui, args: &mut PillArgs) {
    let canvas = ui.allocate_space(vec2(ui.available_width().min(560.0), 300.0)).1;
    let key = ui.scope_id().with("tool-pill-story");
    let current = ui.data(|d| d.get_temp::<&'static str>(key)).unwrap_or("brush");
    let slots = story_slots();
    let place = if args.top { PillPlace::TopCenter } else { PillPlace::BottomCenter };
    let out = ToolPill::new("tool-pill-story").place(place).show(ui, canvas, &slots[..8], &current);
    if let Some(tool) = out.select {
        ui.data_mut(|d| d.insert_temp(key, tool));
    }
}
