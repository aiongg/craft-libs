//! Stories of [`PresetBrowser`]: the six panels it replaces, each with a host that applies the
//! events to its own library.

use craft_theme::Srgba;
use egui::ecolor::Hsva;
use egui::emath::GuiRounding as _;
use egui::epaint::{Mesh, Shape};
use egui::{Color32, Frame, Painter, Rect, Sense, Ui, vec2};

use super::{
    CellShape, DefaultOpen, FooterEvent, PresetBrowser, PresetEvent, PresetFooter, PresetLabels, PresetReorder, PresetRow, PresetState, PresetView,
};
use crate::behavior::{DropPosition, ListDrag, Move};
use crate::controls::checkbox::Checkbox;
use crate::controls::slider;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Preset browser"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/preset-browser/swatches",
            Level::Composites,
            "Preset browser",
            "Swatches",
            PresetArgs::new(ViewArg::Grid, 28.0),
            show_swatches
        ),
        typed_story!(
            "composites/preset-browser/gradients",
            Level::Composites,
            "Preset browser",
            "Gradients",
            PresetArgs::new(ViewArg::Grid, 64.0),
            show_gradients
        ),
        typed_story!(
            "composites/preset-browser/brushes",
            Level::Composites,
            "Preset browser",
            "Brushes",
            PresetArgs::new(ViewArg::List, 32.0),
            show_brushes
        ),
        typed_story!(
            "composites/preset-browser/tool-presets",
            Level::Composites,
            "Preset browser",
            "Tool presets",
            PresetArgs::new(ViewArg::List, 32.0),
            show_tools
        ),
        typed_story!(
            "composites/preset-browser/reorder",
            Level::Composites,
            "Preset browser",
            "Reorder",
            PresetArgs::new(ViewArg::Grid, 36.0),
            show_reorder
        ),
        typed_story!(
            "composites/preset-browser/popover-grid",
            Level::Composites,
            "Preset browser",
            "Popover grid",
            PresetArgs::new(ViewArg::Grid, 40.0),
            show_popover
        ),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ViewArg {
    Grid,
    List,
}

impl ViewArg {
    fn view(self) -> PresetView {
        match self {
            ViewArg::Grid => PresetView::Grid,
            ViewArg::List => PresetView::List,
        }
    }

    fn of(view: PresetView) -> Self {
        match view {
            PresetView::List => ViewArg::List,
            _ => ViewArg::Grid,
        }
    }
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct PresetArgs {
    view: ViewArg,
    /// The thumbnail width in points.
    #[arg(range = 16.0..=96.0)]
    size: f32,
}

impl PresetArgs {
    fn new(view: ViewArg, size: f32) -> Self {
        Self { view, size }
    }
}

/// The host's library: the rows, the view settings, the selection and a line for the last event.
#[derive(Clone)]
struct Library {
    rows: Vec<PresetRow<u32>>,
    state: PresetState<u32>,
    selected: Option<u32>,
    last: String,
    /// The args last copied into the state, so a change on either side wins once.
    applied: Option<PresetArgs>,
    next: u32,
}

impl Library {
    fn new(rows: Vec<PresetRow<u32>>, args: &PresetArgs) -> Self {
        let next = rows.iter().map(|r| r.key).max().map_or(1, |k| k + 1);
        let mut state = PresetState::new();
        state.view = args.view.view();
        state.thumb = args.size;
        Self { rows, state, selected: None, last: String::new(), applied: Some(args.clone()), next }
    }

    /// The args and the state follow each other: whichever changed since the last frame wins.
    fn sync(&mut self, args: &mut PresetArgs) {
        if self.applied.as_ref() != Some(args) {
            self.state.view = args.view.view();
            self.state.thumb = args.size;
        } else {
            args.view = ViewArg::of(self.state.view);
            args.size = self.state.thumb;
        }
        self.applied = Some(args.clone());
    }

    fn at(&self, key: u32) -> Option<usize> {
        self.rows.iter().position(|r| r.key == key)
    }

    /// The end of the subtree that starts at `at`.
    fn end(&self, at: usize) -> usize {
        super::model::children_end(&self.rows, at)
    }

    /// Takes the rows of `key` out with their children.
    fn take(&mut self, key: u32) -> Vec<PresetRow<u32>> {
        let Some(at) = self.at(key) else { return Vec::new() };
        let end = self.end(at);
        self.rows.drain(at..end).collect()
    }

    fn insert(&mut self, at: usize, mut block: Vec<PresetRow<u32>>, depth: usize) {
        let delta = depth as isize - block[0].depth as isize;
        for row in &mut block {
            row.depth = (row.depth as isize + delta).max(0) as usize;
        }
        self.rows.splice(at..at, block);
    }

    fn apply_move(&mut self, mv: &Move<u32>) {
        for &key in &mv.items {
            let block = self.take(key);
            let Some(target) = self.at(mv.target).filter(|_| !block.is_empty()) else { continue };
            let depth = self.rows[target].depth;
            match mv.position {
                DropPosition::Before => self.insert(target, block, depth),
                DropPosition::After => self.insert(self.end(target), block, depth),
                DropPosition::Into => self.insert(target + 1, block, depth + 1),
            }
        }
    }

    fn apply(&mut self, events: Vec<PresetEvent<u32>>) {
        for event in events {
            self.last = format!("{event:?}");
            match event {
                PresetEvent::Select { key, .. } => self.selected = Some(key),
                PresetEvent::Rename { key, text } => {
                    if let Some(at) = self.at(key) {
                        self.rows[at].label = text;
                    }
                }
                PresetEvent::Delete(key) => {
                    self.take(key);
                    self.selected = self.selected.filter(|&s| self.at(s).is_some());
                }
                PresetEvent::MoveTo { key, group } => {
                    let block = self.take(key);
                    if let (Some(at), false) = (self.at(group), block.is_empty()) {
                        let depth = self.rows[at].depth + 1;
                        self.insert(self.end(at), block, depth);
                    }
                }
                PresetEvent::Reorder(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => self.apply_move(&value),
                _ => {}
            }
        }
    }

    fn footer_events(&mut self, events: Vec<FooterEvent>) {
        for event in events {
            match event {
                FooterEvent::New => {
                    let depth = self.rows.last().map_or(0, |r| if r.is_group() { r.depth + 1 } else { r.depth });
                    self.rows.push(PresetRow::item(self.next, format!("New preset {}", self.next), depth));
                    self.next += 1;
                }
                FooterEvent::NewGroup => {
                    self.rows.push(PresetRow::group(self.next, format!("New group {}", self.next), 0));
                    self.next += 1;
                }
                FooterEvent::Delete => {
                    if let Some(key) = self.selected.take() {
                        self.take(key);
                    }
                }
                _ => {}
            }
            self.last = format!("{event:?}");
        }
    }
}

/// A colour from a key: even hues, some translucent when `fade` says so.
fn color_of(key: u32, fade: bool) -> Srgba {
    let hsva = Hsva::new((key as f32 * 0.137).fract(), 0.55 + 0.3 * ((key % 3) as f32 / 2.0), 0.85, 1.0);
    let [r, g, b, _] = hsva.to_rgba_unmultiplied();
    Srgba::new(r, g, b, if fade && key.is_multiple_of(5) { 0.5 } else { 1.0 })
}

fn paint_solid(painter: &Painter, rect: Rect, color: Srgba, tokens: &crate::theme::UiTokens) {
    paint_two(painter, rect, color, color, tokens);
}

/// A thumbnail's two-colour ramp, left to right, over the checkerboard where a colour is translucent.
fn paint_two(painter: &Painter, rect: Rect, a: Srgba, b: Srgba, tokens: &crate::theme::UiTokens) {
    let rect = rect.round_to_pixels(painter.ctx().pixels_per_point());
    if a.a < 1.0 || b.a < 1.0 {
        let swatch = &tokens.component.swatch;
        painter.extend(crate::controls::swatch::checkerboard(
            rect,
            egui::CornerRadius::ZERO,
            [swatch.checker_a.to_egui(), swatch.checker_b.to_egui()],
            swatch.checker_cell,
        ));
    }
    let (left, right): (Color32, Color32) = (a.into(), b.into());
    let mut mesh = Mesh::default();
    for (pos, color) in [(rect.left_top(), left), (rect.right_top(), right), (rect.right_bottom(), right), (rect.left_bottom(), left)] {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn status(ui: &mut Ui, lib: &Library) {
    let selected = lib.selected.map_or("none".to_owned(), |k| k.to_string());
    ui.add(Text::new(format!("Selected: {selected}. Last event: {}", lib.last)).content(Content::Muted));
}

fn swatch_rows() -> Vec<PresetRow<u32>> {
    let mut rows = Vec::new();
    for (group, (name, range)) in [("Default", 1..61), ("Pastels", 61..141), ("Recent", 141..201)].into_iter().enumerate() {
        rows.push(PresetRow::group(1000 + group as u32, name, 0));
        rows.extend(range.map(|n| PresetRow::item(n, format!("Swatch {n}"), 1)));
    }
    rows
}

fn show_swatches(ui: &mut Ui, args: &mut PresetArgs) {
    let start = Library::new(swatch_rows(), args);
    slider::with_values(ui, "preset-swatches", [start], |ui, state| {
        let lib = &mut state[0];
        lib.sync(args);
        ui.set_width(ui.available_width().min(320.0));
        let tokens = Tokens::of(ui);
        let Library { rows, state, selected, .. } = lib;
        let out = PresetBrowser::new("preset-swatches", "Swatches", state, rows)
            .selected(selected.as_ref())
            .flush(true)
            .max_height(200.0)
            .thumb_range(16.0..=48.0)
            .reorder(PresetReorder::items_and_groups())
            .show(ui, |painter, rect, info| paint_solid(painter, rect, color_of(*info.key, true), &tokens));
        lib.apply(out.events);
        let events = PresetFooter::new(&mut lib.state, &PresetLabels::of(ui)).thumb_range(16.0..=48.0).count(200).show(ui);
        lib.footer_events(events);
        status(ui, lib);
    });
}

fn gradient_rows() -> Vec<PresetRow<u32>> {
    let mut rows = vec![PresetRow::group(1000, "Basics", 0)];
    rows.extend((1..=12).map(|n| PresetRow::item(n, format!("Gradient {n}"), 1)));
    rows.push(PresetRow::group(1001, "Transparent", 0));
    rows.extend((13..=20).map(|n| PresetRow::item(n, format!("Gradient {n}"), 1)));
    rows
}

fn gradient_colors(key: u32) -> (Srgba, Srgba) {
    let (a, b) = (color_of(key, false), color_of(key * 7 + 3, false));
    if key > 12 { (a, Srgba::new(a.r, a.g, a.b, 0.0)) } else { (a, b) }
}

/// Cells of 32:18 over the checkerboard; a preset dropped on the mock canvas applies to it.
fn show_gradients(ui: &mut Ui, args: &mut PresetArgs) {
    let start = (Library::new(gradient_rows(), args), None::<u32>);
    slider::with_values(ui, "preset-gradients", [start], |ui, state| {
        let (lib, on_canvas) = &mut state[0];
        lib.sync(args);
        ui.set_width(ui.available_width().min(320.0));
        let tokens = Tokens::of(ui);
        let Library { rows, state, selected, .. } = lib;
        let out = PresetBrowser::new("preset-gradients", "Gradients", state, rows)
            .selected(selected.as_ref())
            .cell_shape(CellShape::Ratio(32.0 / 18.0))
            .max_height(180.0)
            .thumb_range(48.0..=96.0)
            .show(ui, |painter, rect, info| {
                let (a, b) = gradient_colors(*info.key);
                paint_two(painter, rect, a, b, &tokens);
            });
        lib.apply(out.events);
        let (canvas, _) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::hover());
        let response = ui.interact(canvas, ui.scope_id().with("preset-canvas"), Sense::hover());
        if let Some(drag) = response.dnd_release_payload::<ListDrag<u32>>() {
            *on_canvas = drag.items.first().copied();
        }
        let hovering = response.dnd_hover_payload::<ListDrag<u32>>().is_some();
        let painter = ui.painter_at(canvas);
        match *on_canvas {
            Some(key) => {
                let (a, b) = gradient_colors(key);
                paint_two(&painter, canvas, a, b, &tokens);
            }
            None => {
                painter.rect_filled(canvas, 2.0, tokens.component.swatch.checker_a.to_egui());
            }
        }
        if hovering {
            painter.rect_stroke(canvas, 2.0, ui.visuals().selection.stroke, egui::StrokeKind::Inside);
        }
        let name = on_canvas.map_or("none".to_owned(), |k| format!("Gradient {k}"));
        ui.add(Text::new(format!("Drop a gradient on the canvas above. Canvas: {name}")).content(Content::Muted));
        status(ui, lib);
    });
}

const BRUSH_FOLDERS: usize = 10;
const BRUSHES: u32 = 5_000;

fn brush(n: u32, depth: usize) -> PresetRow<u32> {
    PresetRow::item(n, format!("Brush {n}"), depth)
        .caption(format!("{}", 5 + n % 120))
        .preview(4.0, move |painter, rect| stroke_preview(painter, rect, n))
}

fn brush_rows() -> Vec<PresetRow<u32>> {
    let per = BRUSHES / BRUSH_FOLDERS as u32;
    let mut rows = Vec::with_capacity(BRUSHES as usize + 2 * BRUSH_FOLDERS);
    for group in 0..BRUSH_FOLDERS as u32 {
        rows.push(PresetRow::group(100_000 + group, format!("Brush set {}", group + 1), 0));
        let first = group * per + 1;
        if group == 0 {
            // The first set holds a folder, one level down.
            rows.push(PresetRow::group(200_000, "Dry media", 1));
            rows.extend((first..first + 20).map(|n| brush(n, 2)));
            rows.extend((first + 20..first + per).map(|n| brush(n, 1)));
        } else {
            rows.extend((first..first + per).map(|n| brush(n, 1)));
        }
    }
    rows
}

/// A wavy stroke whose width and frequency follow `n`, drawn in the text colour of the rect's ui.
fn stroke_preview(painter: &Painter, rect: Rect, n: u32) {
    let color = painter.ctx().global_style().visuals.text_color();
    let (amp, freq) = (rect.height() * 0.25, 1.0 + (n % 5) as f32);
    let points: Vec<_> = (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            egui::pos2(rect.left() + t * rect.width(), rect.center().y + amp * (t * freq * std::f32::consts::TAU).sin())
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.0 + (n % 4) as f32, color)));
}

/// Search, a folder inside a set, caption numbers and a preview band per row, 5,000 brushes.
fn show_brushes(ui: &mut Ui, args: &mut PresetArgs) {
    let start = Library::new(brush_rows(), args);
    slider::with_values(ui, "preset-brushes", [start], |ui, state| {
        let lib = &mut state[0];
        lib.sync(args);
        ui.set_width(ui.available_width().min(360.0));
        let Library { rows, state, selected, .. } = lib;
        let out = PresetBrowser::new("preset-brushes", "Brushes", state, rows)
            .selected(selected.as_ref())
            .toolbar(true)
            .default_open(DefaultOpen::First)
            .max_height(220.0)
            .thumb_range(24.0..=96.0)
            .reorder(PresetReorder::items_and_groups().nest(1))
            .show(ui, |painter, rect, info| stroke_preview(painter, rect, *info.key));
        lib.apply(out.events);
        let events = PresetFooter::new(&mut lib.state, &PresetLabels::of(ui)).thumb_range(24.0..=96.0).count(BRUSHES as usize).show(ui);
        lib.footer_events(events);
        status(ui, lib);
    });
}

const TOOLS: [(&str, Icon); 6] = [
    ("Brush 12 px", Icon::Brush),
    ("Pencil 3 px", Icon::Pencil),
    ("Eraser soft", Icon::Eraser),
    ("Dropper sample", Icon::Droplet),
    ("Frame square", Icon::Square),
    ("Frame round", Icon::Circle),
];

/// List only, an icon per row, no groups and a checkbox ahead of the slider in the footer.
fn show_tools(ui: &mut Ui, args: &mut PresetArgs) {
    let rows =
        (0..12).map(|n| PresetRow::item(n + 1, format!("{} {}", TOOLS[n as usize % 6].0, n / 6 + 1), 0).icon(TOOLS[n as usize % 6].1)).collect();
    let start = (Library::new(rows, args), true);
    slider::with_values(ui, "preset-tools", [start], |ui, state| {
        let (lib, current_only) = &mut state[0];
        lib.sync(args);
        ui.set_width(ui.available_width().min(320.0));
        let shown: Vec<PresetRow<u32>> = lib.rows.iter().filter(|r| !*current_only || r.key % 2 == 1).cloned().collect();
        let mut browser_state = lib.state.clone();
        let out = PresetBrowser::new("preset-tools", "Tool presets", &mut browser_state, &shown)
            .selected(lib.selected.as_ref())
            .view_lock(PresetView::List)
            .max_height(180.0)
            .show(ui, |_, _, _| {});
        lib.state = browser_state;
        lib.apply(out.events);
        let labels = PresetLabels::of(ui);
        let events = PresetFooter::new(&mut lib.state, &labels)
            .new_group(false)
            .can_delete(lib.selected.is_some())
            .leading(|ui| {
                let _ = Checkbox::bool("Current tool only", current_only).show(ui);
            })
            .show(ui);
        lib.footer_events(events);
        status(ui, lib);
    });
}

const LOCKED: u32 = 1002;

fn reorder_rows() -> Vec<PresetRow<u32>> {
    let mut rows = Vec::new();
    for (group, name) in [(1000, "Warm"), (1001, "Cool"), (LOCKED, "Locked")] {
        rows.push(PresetRow::group(group, name, 0));
        let first = (group - 1000) * 6 + 1;
        rows.extend((first..first + 6).map(|n| PresetRow::item(n, format!("Preset {n}"), 1)));
    }
    rows
}

/// Items and groups drag, Cmd/Ctrl+] and [ move the lead, the context menu has Move to, and the
/// Locked group refuses drops.
fn show_reorder(ui: &mut Ui, args: &mut PresetArgs) {
    let start = Library::new(reorder_rows(), args);
    slider::with_values(ui, "preset-reorder", [start], |ui, state| {
        let lib = &mut state[0];
        lib.sync(args);
        ui.set_width(ui.available_width().min(320.0));
        let tokens = Tokens::of(ui);
        let Library { rows, state, selected, .. } = lib;
        let out = PresetBrowser::new("preset-reorder", "Reorderable presets", state, rows)
            .selected(selected.as_ref())
            .max_height(220.0)
            .reorder(PresetReorder::items_and_groups().accepts(|mv, _| mv.target != LOCKED))
            .show(ui, |painter, rect, info| paint_solid(painter, rect, color_of(*info.key, false), &tokens));
        lib.apply(out.events);
        status(ui, lib);
    });
}

/// A grid without groups in a popup, as the pattern picker shows it.
fn show_popover(ui: &mut Ui, args: &mut PresetArgs) {
    let rows = (1..=30).map(|n| PresetRow::item(n, format!("Pattern {n}"), 0)).collect();
    let start = Library::new(rows, args);
    slider::with_values(ui, "preset-popover", [start], |ui, state| {
        let lib = &mut state[0];
        lib.sync(args);
        let Library { rows, state, selected, .. } = lib;
        let out = Frame::popup(ui.style())
            .show(ui, |ui| {
                ui.set_width(ui.available_width().min(220.0));
                PresetBrowser::new("preset-popover", "Patterns", state, rows)
                    .selected(selected.as_ref())
                    .view_lock(PresetView::Grid)
                    .thumb_range(24.0..=64.0)
                    .max_height(150.0)
                    .show(ui, |painter, rect, info| pattern(painter, rect, *info.key))
            })
            .inner;
        lib.apply(out.events);
        status(ui, lib);
    });
}

fn pattern(painter: &Painter, rect: Rect, n: u32) {
    let color = painter.ctx().global_style().visuals.text_color();
    let painter = painter.with_clip_rect(rect);
    painter.rect_filled(rect, 0.0, Color32::TRANSPARENT);
    let step = 3.0 + (n % 5) as f32 * 2.0;
    let stroke = egui::Stroke::new(1.0, color);
    let mut at = 0.0;
    while at < rect.width() + rect.height() {
        if n.is_multiple_of(2) {
            painter.line_segment([rect.left_top() + vec2(at, 0.0), rect.left_top() + vec2(at - rect.height(), rect.height())], stroke);
        } else {
            painter.circle_filled(rect.left_top() + vec2(at % rect.width(), (at / rect.width()).floor() * step), 1.2, color);
        }
        at += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(lib: &Library) -> Vec<(u32, usize)> {
        lib.rows.iter().map(|r| (r.key, r.depth)).collect()
    }

    #[test]
    fn a_move_carries_a_group_with_its_children_and_sets_the_depth() {
        let args = PresetArgs::new(ViewArg::Grid, 32.0);
        let mut lib = Library::new(
            vec![PresetRow::group(10, "A", 0), PresetRow::item(1, "1", 1), PresetRow::group(20, "B", 0), PresetRow::item(2, "2", 1)],
            &args,
        );
        lib.apply_move(&Move { items: vec![10], target: 20, position: DropPosition::After, copy: false });
        assert_eq!(keys(&lib), [(20, 0), (2, 1), (10, 0), (1, 1)]);
        lib.apply_move(&Move { items: vec![1], target: 20, position: DropPosition::Into, copy: false });
        assert_eq!(keys(&lib), [(20, 0), (1, 1), (2, 1), (10, 0)]);
    }
}
