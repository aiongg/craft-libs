//! Stories: an anchored picker with the controls a PhotoCraft picker holds, and a stroke-editor
//! popover with live edits and child selects.

use egui::{Id, Pos2, Sense, Ui, vec2};

use super::{Popover, PopoverAnchor, PopoverClose, PopoverSize};
use crate::controls::{Button, SearchField, Select, Slider};
use crate::direction::{self, Direction, HasDirection as _};
use crate::story::{Args, Level, Story};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Popover"];

const SIZES: &[&str] = &["resizable", "fit", "fixed"];
const ANCHORS: &[&str] = &["control", "point"];
const MODES: &[&str] = &["Normal", "Multiply", "Screen", "Overlay"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story::new(
            "patterns/popover/anchored",
            Level::Patterns,
            "Popover",
            "Anchored",
            || Args::new().with_choice("anchor", ANCHORS, 0).with_choice("size", SIZES, 0).with_bool("enter", false).with_bool("rtl", false),
            show_anchored,
        ),
        Story::new(
            "patterns/popover/stroke-editor",
            Level::Patterns,
            "Popover",
            "Stroke editor",
            || Args::new().with_bool("rtl", false),
            show_stroke_editor,
        ),
    ]
}

#[derive(Clone, Default)]
struct Demo {
    open: bool,
    at: Option<Pos2>,
    amount: f64,
    mode: usize,
    query: String,
    picked: String,
}

fn with_direction(ui: &mut Ui, args: &Args, add: impl FnOnce(&mut Ui)) {
    let ctx = ui.ctx().clone();
    let outer = ctx.direction();
    direction::apply(&ctx, if args.bool("rtl") { Direction::Rtl } else { Direction::Ltr });
    add(ui);
    direction::apply(&ctx, outer);
}

fn show_anchored(ui: &mut Ui, args: &mut Args) {
    let state_id = Id::unique("craft-ui-popover-story-anchored");
    let mut demo = ui.data_mut(|d| d.get_temp::<Demo>(state_id)).unwrap_or(Demo { amount: 40.0, ..Demo::default() });
    with_direction(ui, args, |ui| {
        let size = match args.choice("size") {
            "fit" => PopoverSize::Fit,
            "fixed" => PopoverSize::Fixed(vec2(240.0, 200.0)),
            _ => PopoverSize::Resizable { default: vec2(300.0, 260.0), min: vec2(230.0, 170.0) },
        };
        let close = PopoverClose::default().enter(args.bool("enter"));
        let (anchor, toggle) = if args.choice("anchor") == "point" {
            let canvas = ui.allocate_response(vec2(320.0, 80.0), Sense::click());
            if canvas.secondary_clicked() {
                demo.open = true;
                demo.at = canvas.interact_pointer_pos();
            }
            (PopoverAnchor::Point(demo.at.unwrap_or(canvas.rect.center())), None)
        } else {
            let chip = ui.add(Button::new("Brush"));
            if chip.clicked() {
                demo.open = !demo.open;
            }
            (PopoverAnchor::Control(chip.clone()), Some(chip.id))
        };
        let mut popover = Popover::new("story-anchored", anchor).size(size).close(close).label("Brush Preset picker");
        if let Some(toggle) = toggle {
            popover = popover.toggle(toggle);
        }
        let Demo { open, amount, mode, query, picked, .. } = &mut demo;
        popover.show(ui, open, |ui, pop| {
            if let Some(v) = Slider::new("size", "Size", *amount, 1.0..=100.0).show(ui).event.as_ref().and_then(|e| e.value()) {
                *amount = *v;
            }
            if let Some(v) = SearchField::new("search", "Search presets", query, "Clear search").show(ui).event.as_ref().and_then(|e| e.value()) {
                *query = v.clone();
            }
            egui::ScrollArea::vertical().max_height((pop.content_size().y - 90.0).max(40.0)).show(ui, |ui| {
                for name in ["Soft round", "Hard round", "Chalk", "Charcoal", "Ink"] {
                    if name.to_lowercase().contains(&query.to_lowercase()) && ui.add(Button::new(name)).clicked() {
                        *picked = name.to_owned();
                        pop.close();
                    }
                }
            });
            if let Some(v) = mode_select(ui, mode) {
                *mode = v;
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, demo));
}

fn mode_select(ui: &mut Ui, mode: &usize) -> Option<usize> {
    let mut value = *mode;
    let mut select = Select::new("mode", "Blend mode", &mut value);
    for (i, name) in MODES.iter().enumerate() {
        select = select.option(i, *name);
    }
    select.show(ui).event.as_ref().and_then(|e| e.value()).copied()
}

fn show_stroke_editor(ui: &mut Ui, args: &mut Args) {
    let state_id = Id::unique("craft-ui-popover-story-stroke");
    let mut demo = ui.data_mut(|d| d.get_temp::<Demo>(state_id)).unwrap_or(Demo { amount: 2.0, ..Demo::default() });
    with_direction(ui, args, |ui| {
        let chip = ui.add(Button::new("Stroke"));
        if chip.clicked() {
            demo.open = !demo.open;
        }
        let Demo { open, amount, mode, .. } = &mut demo;
        Popover::new("story-stroke", PopoverAnchor::Control(chip.clone()))
            .toggle(chip.id)
            .size(PopoverSize::Fixed(vec2(240.0, 120.0)))
            .label("Stroke")
            .show(ui, open, |ui, _| {
                if let Some(v) = Slider::new("width", "Width", *amount, 0.0..=40.0).show(ui).event.as_ref().and_then(|e| e.value()) {
                    *amount = *v;
                }
                if let Some(v) = mode_select(ui, mode) {
                    *mode = v;
                }
            });
    });
    ui.data_mut(|d| d.insert_temp(state_id, demo));
}
