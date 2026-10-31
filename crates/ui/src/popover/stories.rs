//! Stories: an anchored picker with the controls a PhotoCraft picker holds, and a stroke-editor
//! popover with live edits and child selects.

use egui::{Pos2, Sense, Ui, vec2};

use super::{Popover, PopoverAnchor, PopoverClose, PopoverSize};
use crate::controls::{Button, SearchField, Select, Slider};
use crate::direction::{self, Direction, HasDirection as _};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Popover"];

const MODES: &[&str] = &["Normal", "Multiply", "Screen", "Overlay"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum AnchorArg {
    Control,
    Point,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SizeArg {
    Resizable,
    Fit,
    Fixed,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct AnchoredArgs {
    anchor: AnchorArg,
    size: SizeArg,
    enter: bool,
    rtl: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StrokeEditorArgs {
    rtl: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/popover/anchored",
            Level::Patterns,
            "Popover",
            "Anchored",
            AnchoredArgs { anchor: AnchorArg::Control, size: SizeArg::Resizable, enter: false, rtl: false },
            show_anchored
        ),
        typed_story!(
            "patterns/popover/stroke-editor",
            Level::Patterns,
            "Popover",
            "Stroke editor",
            StrokeEditorArgs { rtl: false },
            show_stroke_editor
        ),
        typed_story!(
            "patterns/popover/own-close-rule",
            Level::Patterns,
            "Popover",
            "Own close rule",
            StrokeEditorArgs { rtl: false },
            show_own_close_rule
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

fn with_direction(ui: &mut Ui, rtl: bool, add: impl FnOnce(&mut Ui)) {
    let ctx = ui.ctx().clone();
    let outer = ctx.direction();
    direction::apply(&ctx, if rtl { Direction::Rtl } else { Direction::Ltr });
    add(ui);
    direction::apply(&ctx, outer);
}

fn show_anchored(ui: &mut Ui, args: &mut AnchoredArgs) {
    let state_id = ui.make_persistent_id("craft-ui-popover-story-anchored");
    let mut demo = ui.data_mut(|d| d.get_temp::<Demo>(state_id)).unwrap_or(Demo { amount: 40.0, ..Demo::default() });
    with_direction(ui, args.rtl, |ui| {
        let size = match args.size {
            SizeArg::Fit => PopoverSize::Fit,
            SizeArg::Fixed => PopoverSize::Fixed(vec2(240.0, 200.0)),
            SizeArg::Resizable => PopoverSize::Resizable { default: vec2(300.0, 260.0), min: vec2(230.0, 170.0) },
        };
        let close = PopoverClose::default().enter(args.enter);
        let (anchor, toggle) = if args.anchor == AnchorArg::Point {
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

fn show_stroke_editor(ui: &mut Ui, args: &mut StrokeEditorArgs) {
    let state_id = ui.make_persistent_id("craft-ui-popover-story-stroke");
    let mut demo = ui.data_mut(|d| d.get_temp::<Demo>(state_id)).unwrap_or(Demo { amount: 2.0, ..Demo::default() });
    with_direction(ui, args.rtl, |ui| {
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

/// An always-open editor that closes by the host's own rule: `PopoverCtx::pressed_outside`, which
/// counts the Select's list as inside.
fn show_own_close_rule(ui: &mut Ui, args: &mut StrokeEditorArgs) {
    let state_id = ui.make_persistent_id("craft-ui-popover-story-own-rule");
    let mut demo = ui.data_mut(|d| d.get_temp::<Demo>(state_id)).unwrap_or(Demo { amount: 2.0, ..Demo::default() });
    with_direction(ui, args.rtl, |ui| {
        let chip = ui.add(Button::new("Stroke"));
        if chip.clicked() {
            demo.open = true;
        }
        let Demo { open, amount, mode, .. } = &mut demo;
        let mut close = false;
        Popover::new("story-own-rule", PopoverAnchor::Control(chip.clone()))
            .close(PopoverClose::default().outside(false).focus_left(false))
            .size(PopoverSize::Fixed(vec2(240.0, 120.0)))
            .label("Stroke")
            .show(ui, open, |ui, pop| {
                if let Some(v) = Slider::new("width", "Width", *amount, 0.0..=40.0).show(ui).event.as_ref().and_then(|e| e.value()) {
                    *amount = *v;
                }
                if let Some(v) = mode_select(ui, mode) {
                    *mode = v;
                }
                close = pop.pressed_outside() && !chip.contains_pointer();
            });
        if close {
            *open = false;
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, demo));
}
