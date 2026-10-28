//! The inspector: padding, hit areas and focus order drawn over the story canvas, and the
//! canvas's AccessKit tree in the end-side panel.
//!
//! It reads [`craft_ui::inspect`]: the primitives' records and the AccessKit tree of the
//! previous pass. Its colours are the gallery's own tokens, taken before the story's scope.

use std::collections::HashMap;
use std::sync::Arc;

use craft_theme::TypeRole;
use craft_ui::inspect::{self, AccessTree, Record};
use craft_ui::theme::{ToEgui, UiTokens};
use egui::accesskit::{Action, NodeId, Role};
use egui::{Color32, CornerRadius, Rect, RichText, ScrollArea, Stroke, StrokeKind, Ui};

/// Actions listed in the tree, in this order.
const ACTIONS: &[(Action, &str)] = &[
    (Action::Click, "click"),
    (Action::Focus, "focus"),
    (Action::SetValue, "set value"),
    (Action::Increment, "increment"),
    (Action::Decrement, "decrement"),
    (Action::Expand, "expand"),
    (Action::Collapse, "collapse"),
    (Action::ShowContextMenu, "context menu"),
    (Action::ScrollIntoView, "scroll into view"),
];

/// Which overlay layers show.
#[derive(Clone, Copy, Debug)]
struct Layers {
    padding: bool,
    hits: bool,
    focus: bool,
}

/// One story canvas the overlay drew over in a frame.
#[derive(Clone, Debug)]
pub struct Cell {
    /// "Canvas", or the compare cell's theme and column.
    name: String,
    /// The canvas `Ui`'s AccessKit node.
    node: NodeId,
    /// The labels of the focus-order badges, in order.
    pub badges: Vec<String>,
}

pub struct Inspector {
    layers: Layers,
    /// The gallery's tokens of this frame.
    tokens: Option<Arc<UiTokens>>,
    records: Vec<Record>,
    tree: Option<Arc<AccessTree>>,
    /// The node under the pointer in the tree panel.
    hovered: Option<NodeId>,
    /// The cells drawn this frame, and those of the last frame, which the panel lists (it is
    /// drawn before the canvas).
    cells: Vec<Cell>,
    shown: Vec<Cell>,
    /// The smallest hit area the Size preset allows, from the last cell drawn.
    min_target: f32,
}

impl Default for Inspector {
    fn default() -> Self {
        Self {
            layers: Layers { padding: true, hits: true, focus: true },
            tokens: None,
            records: Vec::new(),
            tree: None,
            hovered: None,
            cells: Vec::new(),
            shown: Vec::new(),
            min_target: 0.0,
        }
    }
}

impl Inspector {
    /// Reads the previous pass's records and tree; `tokens` are the gallery's.
    pub fn begin(&mut self, ctx: &egui::Context, tokens: Arc<UiTokens>) {
        self.tokens = Some(tokens);
        self.records = inspect::records(ctx);
        self.tree = inspect::tree(ctx);
        self.shown = std::mem::take(&mut self.cells);
    }

    /// The cells drawn in the last frame.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Draws the overlay over a story canvas at `rect`, whose `Ui` has AccessKit node `node`.
    /// `min_target` is the story's `size.control.sm`.
    pub fn overlay(&mut self, ui: &Ui, name: String, rect: Rect, node: NodeId, min_target: f32, rtl: bool) {
        let Some(t) = self.tokens.clone() else { return };
        let c = &t.color;
        let painter = ui.painter_at(rect);
        self.min_target = min_target;
        let mut badges = Vec::new();

        if self.layers.padding {
            let guide = c.overlay.guide.to_egui();
            let band = guide.gamma_multiply(0.25);
            for record in self.records.iter().filter(|r| r.layer == ui.layer_id() && rect.intersect(r.outer).is_positive()) {
                for part in band_parts(record.outer, record.content) {
                    painter.rect_filled(part, CornerRadius::ZERO, band);
                }
                painter.rect_stroke(record.content, CornerRadius::ZERO, Stroke::new(1.0, guide), StrokeKind::Inside);
            }
        }

        if let Some(tree) = self.tree.clone() {
            let halo = c.overlay.halo.to_egui();
            if self.layers.hits {
                let (normal, flagged) = (c.overlay.stroke.to_egui(), c.status.bad.solid.to_egui());
                // Text takes clicks for selection; its rect is not a target.
                let targets = tree.interactive(node).into_iter().filter(|id| tree.node(*id).is_some_and(|n| n.role() != Role::Label));
                for bounds in targets.filter_map(|id| tree.bounds(id)) {
                    let small = bounds.width() < min_target || bounds.height() < min_target;
                    let (color, width) = if small { (flagged, 2.0) } else { (normal, 1.0) };
                    painter.rect_stroke(bounds, CornerRadius::ZERO, Stroke::new(width + 2.0, halo), StrokeKind::Inside);
                    painter.rect_stroke(bounds, CornerRadius::ZERO, Stroke::new(width, color), StrokeKind::Inside);
                }
            }
            if let Some(bounds) = self.hovered.filter(|id| tree.walk(node).iter().any(|(n, _)| n == id)).and_then(|id| tree.bounds(id)) {
                let marquee = c.overlay.marquee.to_egui();
                painter.rect_filled(bounds, CornerRadius::ZERO, marquee.gamma_multiply(0.2));
                painter.rect_stroke(bounds, CornerRadius::ZERO, Stroke::new(2.0, marquee), StrokeKind::Outside);
            }
            let order = tree.focus_order(node);
            badges = order.iter().map(|id| label_of(&tree, *id)).collect();
            if self.layers.focus {
                let font = t.font(TypeRole::Caption);
                let (fill, text) = (c.status.info.solid.to_egui(), c.text.on_status.to_egui());
                for (i, bounds) in order.iter().enumerate().filter_map(|(i, id)| Some((i, tree.bounds(*id)?))) {
                    let centre = if rtl { bounds.right_top() } else { bounds.left_top() };
                    let galley = painter.layout_no_wrap((i + 1).to_string(), font.clone(), text);
                    let radius = (galley.size().max_elem() / 2.0 + 2.0).max(7.0);
                    painter.circle(centre, radius, fill, Stroke::new(1.0, halo));
                    painter.galley(centre - galley.size() / 2.0, galley, text);
                }
            }
        }
        self.cells.push(Cell { name, node, badges });
    }

    /// The end-side panel: layer switches and the AccessKit tree of each cell.
    pub fn panel(&mut self, ui: &mut Ui) {
        ui.heading("Inspect");
        ui.checkbox(&mut self.layers.padding, "Padding");
        ui.checkbox(&mut self.layers.hits, "Hit areas");
        ui.checkbox(&mut self.layers.focus, "Focus order");
        if self.min_target > 0.0 {
            ui.label(RichText::new(format!("Hit areas under {} pt (size.control.sm) are flagged.", self.min_target)).small());
        }
        ui.separator();
        self.hovered = None;
        let Some(tree) = self.tree.clone() else {
            ui.label("Waiting for the AccessKit tree.");
            return;
        };
        let warn = self.tokens.as_ref().map_or(Color32::RED, |t| t.color.status.warn.fg.to_egui());
        let cells = self.shown.clone();
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, cell) in cells.iter().enumerate() {
                egui::CollapsingHeader::new(&cell.name).id_salt(("gallery.inspect.cell", i)).default_open(i == 0).show(ui, |ui| {
                    self.tree_rows(ui, &tree, cell.node, warn);
                });
            }
        });
    }

    /// One row per node under `root`, indented by depth; unlabelled containers are left out
    /// and their children shown in their place.
    fn tree_rows(&mut self, ui: &mut Ui, tree: &AccessTree, root: NodeId, warn: Color32) {
        let tab: HashMap<NodeId, usize> = tree.focus_order(root).into_iter().enumerate().map(|(i, id)| (id, i + 1)).collect();
        // Whether the node at each depth of the current path is shown.
        let mut path: Vec<bool> = Vec::new();
        for (id, depth) in tree.walk(root) {
            let Some(node) = tree.node(id) else { continue };
            path.truncate(depth);
            let shown = !(node.role() == Role::GenericContainer && node.label().is_none());
            let indent = path.iter().filter(|s| **s).count();
            path.push(shown);
            if !shown {
                continue;
            }
            let row = ui.horizontal(|ui| {
                ui.add_space(indent as f32 * 12.0);
                if let Some(n) = tab.get(&id) {
                    ui.label(RichText::new(format!("{n}.")).strong());
                }
                ui.label(RichText::new(format!("{:?}", node.role())).strong());
                // A text node's name is its value, shown below.
                if let Some(name) = tree.name(id).filter(|_| node.role() != Role::Label) {
                    ui.label(format!("\"{name}\""));
                }
                if let Some(value) = node.value() {
                    ui.label(RichText::new(format!("= {value}")).italics());
                }
                let actions: Vec<&str> = ACTIONS.iter().filter(|(a, _)| node.supports_action(*a)).map(|(_, name)| *name).collect();
                if !actions.is_empty() {
                    ui.label(RichText::new(actions.join(", ")).small());
                }
                if node.is_disabled() {
                    ui.label(RichText::new("disabled").small());
                }
                if inspect::is_interactive(node) && tree.name(id).is_none() {
                    ui.label(RichText::new("no label").color(warn).strong());
                }
            });
            if row.response.contains_pointer() {
                self.hovered = Some(id);
            }
        }
    }
}

/// The label of a node, or its role when it has none.
fn label_of(tree: &AccessTree, id: NodeId) -> String {
    tree.name(id).or_else(|| tree.node(id).map(|n| format!("{:?}", n.role()))).unwrap_or_default()
}

/// The band between `outer` and `content` as up to four rects.
fn band_parts(outer: Rect, content: Rect) -> impl Iterator<Item = Rect> {
    let inner = content.intersect(outer);
    [
        Rect::from_min_max(outer.min, egui::pos2(outer.max.x, inner.min.y)),
        Rect::from_min_max(egui::pos2(outer.min.x, inner.max.y), outer.max),
        Rect::from_min_max(egui::pos2(outer.min.x, inner.min.y), egui::pos2(inner.min.x, inner.max.y)),
        Rect::from_min_max(egui::pos2(inner.max.x, inner.min.y), egui::pos2(outer.max.x, inner.max.y)),
    ]
    .into_iter()
    .filter(|r| r.is_positive())
}

#[cfg(test)]
mod tests {
    use egui::vec2;

    use super::*;

    #[test]
    fn the_padding_band_is_the_outer_rect_less_the_content() {
        let outer = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(20.0, 10.0));
        let content = outer.shrink2(vec2(4.0, 2.0));
        let parts: Vec<Rect> = band_parts(outer, content).collect();
        let area: f32 = parts.iter().map(|r| r.area()).sum();
        assert_eq!(area, outer.area() - content.area());
        assert!(parts.iter().all(|p| !p.intersects(content.shrink(0.1))));
        assert_eq!(band_parts(outer, outer).count(), 0);
    }
}
