//! [`ReferencePoint`]: the 3 × 3 picker of a box's nine points.

use egui::accesskit::Role;
use egui::{
    Color32, CornerRadius, EventFilter, Id, IdSalt, Key, Modifiers, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo,
    vec2,
};

use super::quad::Anchor9;
use crate::controls::TooltipExt as _;
use crate::edit::{EditEvent, EditResponse};
use crate::primitives::FocusRing;
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// The size of a [`ReferencePoint`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefSize {
    /// `ref_point.compact` points square, for an options bar.
    #[default]
    Compact,
    /// `ref_point.large` points square, for a dialog; with [`growth`](ReferencePoint::growth) arrows.
    Large,
}

/// A reference point picker's look, from [`ReferencePoint::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReferencePointStyle {
    /// Side of the whole picker: `ref_point.compact` or `ref_point.large`.
    pub side: f32,
    /// Side of an unpicked point's dot: `ref_point.dot` (the large picker draws it twice as wide).
    pub dot: f32,
    /// Side of the picked point's dot: `ref_point.dot` (compact) or twice that (large).
    pub picked_dot: f32,
    /// The picker's fill: `surface.sunken`.
    pub fill: Color32,
    /// 1 point of `line.control_edge`.
    pub border: Stroke,
    /// `shape.radius.control`.
    pub radius: CornerRadius,
    /// An unpicked point: `text.muted`.
    pub dot_color: Color32,
    /// The picked point: `accent.text`.
    pub picked: Color32,
    /// The cell under the pointer: `accent.soft`.
    pub hover: Color32,
    /// The growth arrows: `text.muted`.
    pub arrow: Color32,
}

/// The nine points of a box as a grid, to pick the reference point a transform or a canvas resize
/// works about:
/// `ReferencePoint::new("ref-point", Some(Anchor9::Center)).size(RefSize::Compact).show(ui)`.
///
/// ```
/// use craft_ui::canvas::quad::Anchor9;
/// use craft_ui::canvas::{RefSize, ReferencePoint};
/// # egui::__run_test_ui(|ui| {
/// let mut pivot = Some(Anchor9::Center);
/// let edit = ReferencePoint::new("ref-point", pivot).size(RefSize::Compact).show(ui);
/// if let Some(craft_ui::edit::EditEvent::Set { value, .. }) = edit.event {
///     pivot = value;
/// }
/// # });
/// ```
///
/// - **Look.** `Compact` is PhotoCraft's options-bar picker, `Large` its Canvas Size picker,
///   whose neighbours of the picked point show arrows in the directions the canvas grows
///   ([`growth`](Self::growth)). `None` (a reference point placed elsewhere) lights no point.
/// - **Pointer.** A click on a cell picks it at once ([`Set`](EditEvent::Set)).
/// - **Keyboard.** One Tab stop. The arrows move a keyboard cell over the grid and stop at its
///   edges; Space or Enter picks it. Arrows are document directions: they do not mirror in a
///   right-to-left interface, and neither does the grid.
/// - **AccessKit.** A `RadioGroup` named [`label`](Self::label) with nine `RadioButton` children
///   named by [`names`](Self::names); the keyboard cell is the group's active descendant while it
///   has focus, the picked one otherwise.
#[must_use = "show the picker with `show`"]
pub struct ReferencePoint {
    id_salt: IdSalt,
    picked: Option<Anchor9>,
    size: RefSize,
    growth: Option<(i8, i8)>,
    label: String,
    names: [String; 9],
    enabled: bool,
}

impl ReferencePoint {
    /// A picker showing `picked`; `id_salt` keeps its keyboard cell.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, picked: Option<Anchor9>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            picked,
            size: RefSize::Compact,
            growth: None,
            label: "Reference point".into(),
            names: Anchor9::ALL.map(|a| ENGLISH[a as usize].into()),
            enabled: true,
        }
    }

    /// The picker's size.
    pub fn size(mut self, size: RefSize) -> Self {
        self.size = size;
        self
    }

    /// Shows arrows in the cells next to the picked point, pointing away from it: each axis grows
    /// when its sign is not negative and shrinks when it is (PhotoCraft's Canvas Size).
    pub fn growth(mut self, sign_x: i8, sign_y: i8) -> Self {
        self.growth = Some((sign_x, sign_y));
        self
    }

    /// The group's accessible name, translated ("Reference point" by default).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The names of the nine points in [`Anchor9::ALL`] order, translated ("Top left" to "Bottom
    /// right" by default). They are also the tooltip of the cell under the pointer.
    pub fn names(mut self, names: [String; 9]) -> Self {
        self.names = names;
        self
    }

    /// Whether the picker takes input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn style(tokens: &UiTokens, size: RefSize) -> ReferencePointStyle {
        let (r, c) = (&tokens.component.ref_point, &tokens.color);
        let (side, scale) = match size {
            RefSize::Compact => (r.compact, 1.0),
            RefSize::Large => (r.large, 2.0),
        };
        ReferencePointStyle {
            side,
            dot: r.dot,
            picked_dot: r.dot * scale,
            fill: c.surface.sunken.to_egui(),
            border: Stroke::new(1.0, c.line.control_edge.to_egui()),
            radius: tokens.radius.control,
            dot_color: c.text.muted.to_egui(),
            picked: c.accent.text.to_egui(),
            hover: c.accent.soft.to_egui(),
            arrow: c.text.muted.to_egui(),
        }
    }

    /// Shows the picker and returns this frame's pick: a [`Set`](EditEvent::Set) of the point.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Option<Anchor9>> {
        let tokens = Tokens::of(ui);
        let style = ReferencePoint::style(&tokens, self.size);
        let enabled = self.enabled && ui.is_enabled();
        let id = ui.make_persistent_id(self.id_salt);
        let sense = if enabled { Sense::click() | Sense::FOCUSABLE } else { Sense::hover() };
        let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(style.side), sense);
        let response_id = response.id;
        let cell = style.side / 3.0;
        let cell_rect = |i: usize| Rect::from_min_size(rect.min + vec2((i % 3) as f32 * cell, (i / 3) as f32 * cell), Vec2::splat(cell));
        let picked = self.picked.map(|a| a as usize);

        let memory = id.with("keyboard cell");
        let mut keyboard: Option<usize> = ui.data(|d| d.get_temp(memory));
        let focused = enabled && response.has_focus();
        let mut chosen = None;
        if !focused {
            keyboard = None;
        }
        if focused {
            let from = keyboard.or(picked).unwrap_or(Anchor9::Center as usize);
            let mut at = from;
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(response_id, EventFilter { horizontal_arrows: true, vertical_arrows: true, ..EventFilter::default() });
            });
            let pick = ui.input_mut(|i| {
                let (row, column) = (at / 3, at % 3);
                if i.consume_key(Modifiers::NONE, Key::ArrowLeft) && column > 0 {
                    at -= 1;
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowRight) && column < 2 {
                    at += 1;
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) && row > 0 {
                    at -= 3;
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) && row < 2 {
                    at += 3;
                }
                i.consume_key(Modifiers::NONE, Key::Space) || i.consume_key(Modifiers::NONE, Key::Enter)
            });
            keyboard = Some(at);
            if pick {
                chosen = Some(at);
            }
        }
        if enabled
            && response.clicked()
            && let Some(at) = response.interact_pointer_pos()
        {
            let (column, row) = ((((at.x - rect.min.x) / cell) as usize).min(2), (((at.y - rect.min.y) / cell) as usize).min(2));
            let index = row * 3 + column;
            keyboard = Some(index);
            chosen = Some(index);
        }
        ui.data_mut(|d| match keyboard {
            Some(k) => {
                d.insert_temp(memory, k);
            }
            None => {
                d.remove_temp::<usize>(memory);
            }
        });

        let hovered = enabled.then(|| response.hover_pos()).flatten().and_then(|p| cell_at(rect, cell, p));
        let shown = chosen.or(picked);
        self.paint(ui, &style, rect, cell, shown, hovered, enabled);
        if focused && FocusRing::visible(ui.ctx()) {
            ui.painter().add(FocusRing::shape(&tokens, crate::primitives::Corner::Control, cell_rect(keyboard.unwrap_or(4))));
        }
        let active = if focused { keyboard.or(shown) } else { shown };
        self.nodes(ui, response_id, &cell_rect, shown, active, enabled);
        response.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, enabled, &self.label));
        if let Some(i) = hovered {
            response = response.tooltip(&self.names[i]);
        }
        let event = chosen.map(|i| {
            response.mark_changed();
            EditEvent::set(Some(Anchor9::ALL[i]), ui.input(|i| i.modifiers))
        });
        EditResponse::new(response, event)
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(&self, ui: &Ui, style: &ReferencePointStyle, rect: Rect, cell: f32, shown: Option<usize>, hovered: Option<usize>, enabled: bool) {
        let painter = ui.painter();
        painter.rect(rect, style.radius, style.fill, style.border, StrokeKind::Inside);
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
        for i in 0..9 {
            let centre = rect.min + vec2((i % 3) as f32 + 0.5, (i / 3) as f32 + 0.5) * cell;
            if hovered == Some(i) && shown != Some(i) {
                painter.rect_filled(Rect::from_center_size(centre, Vec2::splat(cell)).shrink(0.5), 0.0, style.hover);
            }
            let (side, color) = if shown == Some(i) { (style.picked_dot, style.picked) } else { (style.dot, style.dot_color) };
            painter.rect_filled(Rect::from_center_size(centre, Vec2::splat(side)), 0.5, fade(color));
            if let (Some((sx, sy)), Some(from)) = (self.growth, shown)
                && i != from
            {
                let (dx, dy) = ((i % 3) as i32 - (from % 3) as i32, (i / 3) as i32 - (from / 3) as i32);
                if dx.abs() <= 1 && dy.abs() <= 1 {
                    let dir = vec2(dx as f32 * if sx < 0 { -1.0 } else { 1.0 }, dy as f32 * if sy < 0 { -1.0 } else { 1.0 });
                    paint_arrow(painter, centre, dir.normalized(), cell / 24.0, fade(style.arrow));
                }
            }
        }
    }

    /// The nine radio buttons in the group's AccessKit node.
    fn nodes(&self, ui: &mut Ui, group: Id, cell_rect: &dyn Fn(usize) -> Rect, shown: Option<usize>, active: Option<usize>, enabled: bool) {
        let nodes = ui.new_child(UiBuilder::new().max_rect(cell_rect(0).union(cell_rect(8))).accessibility_parent(group));
        for (i, name) in self.names.iter().enumerate() {
            let response = nodes.interact(cell_rect(i), group.with(("cell", i)), Sense::hover());
            response.widget_info(|| WidgetInfo::selected(Role::RadioButton, enabled, shown == Some(i), name));
        }
        let active = active.map(|i| group.with(("cell", i)).accesskit_id());
        ui.ctx().accesskit_node_builder(group, |node| match active {
            Some(id) => node.set_active_descendant(id),
            None => node.clear_active_descendant(),
        });
    }
}

const ENGLISH: [&str; 9] = ["Top left", "Top", "Top right", "Left", "Center", "Right", "Bottom left", "Bottom", "Bottom right"];

fn cell_at(rect: Rect, cell: f32, p: Pos2) -> Option<usize> {
    rect.contains(p).then(|| {
        let (column, row) = ((((p.x - rect.min.x) / cell) as usize).min(2), (((p.y - rect.min.y) / cell) as usize).min(2));
        row * 3 + column
    })
}

/// An arrow centred on `centre` pointing along `dir`, drawn for a 24-point cell and scaled by `scale`.
fn paint_arrow(painter: &egui::Painter, centre: Pos2, dir: Vec2, scale: f32, color: Color32) {
    let (tip, tail) = (centre + dir * 6.0 * scale, centre - dir * 5.0 * scale);
    let side = vec2(-dir.y, dir.x) * 3.5 * scale;
    painter.line_segment([tail, tip], Stroke::new(1.3 * scale.max(0.5), color));
    painter.add(Shape::convex_polygon(
        vec![tip + dir * 1.5 * scale, tip - dir * 3.5 * scale + side, tip - dir * 3.5 * scale - side],
        color,
        Stroke::NONE,
    ));
}

/// The picker in both sizes: pick with the pointer or the arrow keys and Space; `growth` shows the
/// Canvas Size arrows.
fn show_story(ui: &mut Ui, args: &mut Args) {
    let key = ui.scope_id().with("ref-point-story");
    let mut picked: Option<Anchor9> = ui.data(|d| d.get_temp(key)).unwrap_or(Some(Anchor9::Center));
    let grow = if args.bool("shrink") { -1 } else { 1 };
    ui.horizontal(|ui| {
        let compact = ReferencePoint::new("compact", picked).show(ui);
        ui.add_space(24.0);
        let large = ReferencePoint::new("large", picked).size(RefSize::Large).growth(grow, grow).show(ui);
        for e in [compact, large] {
            if let Some(EditEvent::Set { value, .. }) = e.event {
                picked = value;
            }
        }
    });
    if args.bool("custom pivot") {
        ui.add_space(8.0);
        let _ = ReferencePoint::new("custom", None).show(ui);
    }
    ui.data_mut(|d| d.insert_temp(key, picked));
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ReferencePoint"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "primitives/reference-point/picker",
        level: Level::Primitives,
        component: "ReferencePoint",
        name: "Picker",
        args: || Args::new().with_bool("shrink", false).with_bool("custom pivot", false),
        show: show_story,
    }]
}

#[cfg(test)]
#[path = "ref_point_tests.rs"]
mod tests;
