//! `ToolRail`: the rail of tool slots, with hidden tools, the last-used tool per slot and one or
//! two columns, drawn with the `rail.*` tokens.

use egui::accesskit::{Orientation, Role};
use egui::{Align, EventFilter, Frame, Id, IdSalt, Key, Layout, Margin, Modifiers, Rect, Response, ScrollArea, Sense, Ui, UiBuilder, pos2, vec2};

use super::flyout::{FlyoutButton, FlyoutItem};
use crate::controls::{IconButton, IconButtonStyle};
use crate::direction;
use crate::icons::Icon;
use crate::panes::Edge;
use crate::story::{Args, Level, Story, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use egui::AsIdSalt;

/// Names a slot of the rail. The host numbers its slots; [`LastUsed`] remembers a tool under it.
pub type SlotId = u32;

/// One tool of a slot.
#[derive(Clone, Debug, PartialEq)]
pub struct RailTool<T> {
    pub id: T,
    pub icon: Icon,
    /// The tool's name: tooltip, flyout row and accessible name. Translate it in the app.
    pub label: String,
    /// The key that selects the tool ("B"), as the host formats it for the tooltip and the flyout.
    pub shortcut: Option<String>,
    /// `false`: drawn disabled and not selectable.
    pub enabled: bool,
}

impl<T> RailTool<T> {
    /// An enabled tool without a shortcut.
    pub fn new(id: T, icon: Icon, label: impl Into<String>) -> Self {
        Self { id, icon, label: label.into(), shortcut: None, enabled: true }
    }

    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        let shortcut = shortcut.into();
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// A group of tools that share one button; its flyout lists them.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolSlot<T> {
    pub id: SlotId,
    pub tools: Vec<RailTool<T>>,
}

impl<T> ToolSlot<T> {
    pub fn new(id: SlotId, tools: Vec<RailTool<T>>) -> Self {
        Self { id, tools }
    }
}

/// The tool each slot showed last. Plain data the host keeps with its workspace.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LastUsed<T> {
    tools: Vec<(SlotId, T)>,
}

impl<T> Default for LastUsed<T> {
    fn default() -> Self {
        Self { tools: Vec::new() }
    }
}

impl<T: PartialEq> LastUsed<T> {
    /// The tool remembered for `slot`.
    pub fn get(&self, slot: SlotId) -> Option<&T> {
        self.tools.iter().find(|(id, _)| *id == slot).map(|(_, tool)| tool)
    }

    /// Remembers `tool` for `slot`.
    pub fn set(&mut self, slot: SlotId, tool: T) {
        match self.tools.iter_mut().find(|(id, _)| *id == slot) {
            Some(entry) => entry.1 = tool,
            None => self.tools.push((slot, tool)),
        }
    }
}

/// What the footer closure of [`ToolRail::show`] is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RailFooter {
    /// The rail's columns: with two, the host lays its two mode buttons side by side.
    pub columns: usize,
}

/// The rail's width, from [`ToolRail::metrics`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RailMetrics {
    /// `rail.width`; in two columns `rail.width + rail.button + rail.gap`.
    pub width: f32,
    /// The space at each side of the buttons: `(rail.width - rail.button) / 2`.
    pub pad_x: f32,
}

/// What [`ToolRail::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolRailOutput<T> {
    /// The tool the host should make current: the shown tool of a clicked slot, or the tool chosen
    /// in a flyout.
    pub select: Option<T>,
    /// The shown tool of a slot that was double clicked; the host decides what that means.
    pub double_clicked: Option<T>,
    /// The column toggle was clicked; the host flips its column count.
    pub toggle_columns: bool,
    /// The Edit Toolbar button was clicked.
    pub edit_toolbar: bool,
    /// The slot whose flyout is open.
    pub flyout: Option<SlotId>,
    /// Where the rail was drawn.
    pub rect: Rect,
}

/// The slots with their tools that are not hidden, each with its index in `slots`. A slot keeps
/// only its visible tools, so its flyout lists just those; a slot with none left is dropped.
pub fn visible_slots<'a, T: PartialEq>(slots: &'a [ToolSlot<T>], hidden: &[T]) -> Vec<(usize, Vec<&'a RailTool<T>>)> {
    slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| {
            let tools: Vec<&RailTool<T>> = slot.tools.iter().filter(|tool| !hidden.contains(&tool.id)).collect();
            (!tools.is_empty()).then_some((index, tools))
        })
        .collect()
}

/// The tool a slot shows: the current tool when the slot has it, else the one remembered in
/// `last_used` if it is still among the visible `tools` and enabled, else the first enabled tool,
/// else the first. `None` for an empty slot.
pub fn shown_tool<'a, T: PartialEq>(current: &T, slot: SlotId, tools: &[&'a RailTool<T>], last_used: &LastUsed<T>) -> Option<&'a RailTool<T>> {
    let find = |id: &T| tools.iter().copied().find(|tool| tool.id == *id);
    let stored = last_used.get(slot).and_then(find).filter(|tool| tool.enabled);
    find(current).or(stored).or_else(|| tools.iter().copied().find(|tool| tool.enabled)).or_else(|| tools.first().copied())
}

/// A move between the rail's buttons, which lie in rows of `columns` in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GridMove {
    Up,
    Down,
    Previous,
    Next,
}

/// The enabled button `movement` reaches from `index`, skipping disabled ones in the same
/// direction; `None` at the edge of the grid.
pub(super) fn grid_step(index: usize, enabled: &[bool], columns: usize, movement: GridMove) -> Option<usize> {
    let columns = columns.max(1);
    let mut at = index;
    loop {
        at = match movement {
            GridMove::Up => at.checked_sub(columns)?,
            GridMove::Down => at.checked_add(columns).filter(|at| *at < enabled.len())?,
            GridMove::Previous => at.checked_sub(1)?,
            GridMove::Next => at.checked_add(1).filter(|at| *at < enabled.len())?,
        };
        if enabled[at] {
            return Some(at);
        }
    }
}

/// The rail of tool slots, as a side panel at one edge of the window.
///
/// ```
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::{LastUsed, RailTool, ToolRail, ToolSlot};
/// # egui::__run_test_ui(|ui| {
/// let slots = [
///     ToolSlot::new(0, vec![RailTool::new("move", Icon::Move, "Move").shortcut("V")]),
///     ToolSlot::new(1, vec![RailTool::new("brush", Icon::Brush, "Brush").shortcut("B"), RailTool::new("pencil", Icon::Pencil, "Pencil")]),
/// ];
/// let mut last_used = LastUsed::default();
/// let out = ToolRail::new("tools").columns(1).show(ui, &slots, &"move", &mut last_used, |_, _| {});
/// if let Some(tool) = out.select {
///     // Make it the current tool.
/// }
/// # });
/// ```
///
/// - **Slots:** one [`FlyoutButton`] per slot, showing the slot's [shown tool](shown_tool); the
///   slot with the current tool is toggled on. A click selects the shown tool, a row of the flyout
///   selects that tool. The tooltip reads "Brush  (B)".
/// - **Width and columns:** [`ToolRail::metrics`]. With two columns the slots fill rows left to
///   right (right to left in a mirrored interface). The host owns the column count: the chevron
///   at the top reports [`toggle_columns`](ToolRailOutput::toggle_columns).
/// - **Layout:** the footer lies right under the last tool (or the Edit Toolbar button). Only
///   when the window is too short for both do the slots scroll, with the footer kept in view at
///   the bottom. Call it before the panel host, so the rail is the outermost panel on its edge.
/// - **Keyboard:** every button is a Tab stop. The arrow keys move between the slot buttons and
///   the Edit Toolbar button by the grid (Up and Down; Left and Right as well in two columns),
///   except while a flyout is open, whose menu then reads them.
/// - **AccessKit:** a vertical `Toolbar` named by [`label`](ToolRail::label); a slot is a `Button`
///   named by the shown tool, `selected`, with the shortcut as its description, and `has_popup`
///   menu with `expanded` when it has several tools.
#[must_use = "You should call .show"]
pub struct ToolRail<'a, T> {
    id_salt: IdSalt,
    edge: Edge,
    columns: usize,
    column_toggle: Option<String>,
    edit_toolbar: Option<String>,
    hidden: &'a [T],
    label: String,
    scroll_bar_label: String,
}

impl<'a, T: Clone + PartialEq> ToolRail<'a, T> {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            edge: Edge::Start,
            columns: 1,
            column_toggle: None,
            edit_toolbar: None,
            hidden: &[],
            label: "Tools".to_owned(),
            scroll_bar_label: "Scroll bar".to_owned(),
        }
    }

    /// The edge the rail sits at (default [`Edge::Start`]). The rail is vertical, so
    /// [`Edge::Bottom`] draws it at the end edge.
    pub fn edge(mut self, edge: Edge) -> Self {
        self.edge = edge;
        self
    }

    /// One or two columns of slots (default 1); the host owns the count.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.clamp(1, 2);
        self
    }

    /// Shows the column toggle at the top, named `label` (translated).
    pub fn column_toggle(mut self, label: impl Into<String>) -> Self {
        self.column_toggle = Some(label.into());
        self
    }

    /// Shows the Edit Toolbar button after the last slot, named `label` (translated).
    pub fn edit_toolbar(mut self, label: impl Into<String>) -> Self {
        self.edit_toolbar = Some(label.into());
        self
    }

    /// The tools hidden by the user's toolbar settings.
    pub fn hidden(mut self, hidden: &'a [T]) -> Self {
        self.hidden = hidden;
        self
    }

    /// The toolbar's translated accessible name (default "Tools").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The scroll bar's translated accessible name (default "Scroll bar").
    pub fn scroll_bar_label(mut self, label: impl Into<String>) -> Self {
        self.scroll_bar_label = label.into();
        self
    }

    /// The width of a rail with `columns` columns.
    pub fn metrics(tokens: &UiTokens, columns: usize) -> RailMetrics {
        let r = &tokens.component.rail;
        let width = if columns > 1 { r.width + r.button + r.gap } else { r.width };
        RailMetrics { width, pad_x: ((r.width - r.button) / 2.0).max(0.0) }
    }

    /// True when the rail sits against the left edge of the window.
    fn left(&self, ui: &Ui) -> bool {
        matches!((self.edge, direction::is_rtl(ui)), (Edge::Start, false) | (Edge::End | Edge::Bottom, true))
    }

    /// Draws the rail. `current` is the host's tool; `last_used` is updated with it. `footer`
    /// draws under the last tool (colour chips, Quick Mask, Screen Mode); it stays in view when the
    /// window is short. It must size to its content: its height is measured on one pass and used
    /// on the next, so a footer that fills the available height never settles.
    pub fn show(
        self,
        ui: &mut Ui,
        slots: &[ToolSlot<T>],
        current: &T,
        last_used: &mut LastUsed<T>,
        footer: impl FnOnce(&mut Ui, RailFooter),
    ) -> ToolRailOutput<T> {
        let left = self.left(ui);
        let tokens = Tokens::of(ui);
        let metrics = Self::metrics(&tokens, self.columns);
        let r = &tokens.component.rail;
        let layout = Grid {
            columns: self.columns,
            button: r.button,
            gap: r.gap,
            row_gap: r.row_gap,
            toggle_height: r.toggle_height,
            toggle_icon: r.toggle_icon,
            pad_x: metrics.pad_x,
            left,
        };
        let (fill, pad_y) = (r.fill.to_egui(), r.pad_y);
        let style = FlyoutButton::<T>::style(&tokens).icon_button;
        drop(tokens);

        let id = ui.make_persistent_id(self.id_salt);
        let panel = if left { egui::Panel::left(id.with("panel")) } else { egui::Panel::right(id.with("panel")) };
        let frame = Frame::NONE.fill(fill).inner_margin(Margin::symmetric(0, pad_y.round() as i8));
        let mut out =
            ToolRailOutput { select: None, double_clicked: None, toggle_columns: false, edit_toolbar: false, flyout: None, rect: Rect::NOTHING };
        let shown = panel.exact_size(metrics.width).resizable(false).show_separator_line(false).frame(frame).show(ui, |ui| {
            let toolbar = UiBuilder::new().id_salt("toolbar").accessibility_role(Role::Toolbar).accessibility_label(self.label.as_str());
            ui.scope_builder(toolbar, |ui| {
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| node.set_orientation(Orientation::Vertical));
                self.body(ui, id, &layout, style, slots, current, last_used, footer, &mut out);
            });
        });
        out.rect = shown.response.rect;
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn body(
        &self,
        ui: &mut Ui,
        rail: Id,
        grid: &Grid,
        style: IconButtonStyle,
        slots: &[ToolSlot<T>],
        current: &T,
        last_used: &mut LastUsed<T>,
        footer: impl FnOnce(&mut Ui, RailFooter),
        out: &mut ToolRailOutput<T>,
    ) {
        ui.spacing_mut().item_spacing = vec2(grid.gap, grid.row_gap);
        let rtl_icons = direction::is_rtl(ui);
        let center_x = ui.max_rect().center().x;
        if let Some(label) = &self.column_toggle {
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), grid.toggle_height), Sense::hover());
            let rect = Rect::from_center_size(rect.center(), vec2(grid.button, grid.toggle_height));
            let icon = chevron(grid.columns, grid.left, rtl_icons);
            let toggle = IconButtonStyle { icon: grid.toggle_icon, ..style };
            let response = IconButton::new(icon, label.as_str()).with_id(rail.with("column-toggle")).with_style(toggle).show_at(ui, rect);
            out.toggle_columns = response.clicked();
        }
        let visible = visible_slots(slots, self.hidden);
        let edit = usize::from(self.edit_toolbar.is_some());
        let rtl = direction::is_rtl(ui);
        // The footer lies right under the last button. Its height is the one measured on the last
        // pass; the slots scroll only when they and the footer do not fit.
        let footer_id = rail.with("footer-height");
        let footer_height = ui.ctx().data(|d| d.get_temp::<f32>(footer_id));
        let reserved = footer_height.map_or(0.0, |height| height + grid.row_gap);
        let scroll = ScrollArea::vertical()
            .id_salt(rail.with("slots"))
            .auto_shrink([false, true])
            .max_height((ui.available_height() - reserved).max(0.0))
            .vertical_scroll_bar_label(&self.scroll_bar_label);
        scroll.show(ui, |ui| {
            // Each button with whether it takes input, in reading order.
            let mut cells: Vec<(Id, bool)> = Vec::new();
            let mut row: Option<Rect> = None;
            for k in 0..visible.len() + edit {
                let (column, row_rect) = (k % grid.columns, row);
                let row_rect = match (column, row_rect) {
                    (0, _) | (_, None) => ui.allocate_exact_size(vec2(ui.available_width(), grid.button), Sense::hover()).0,
                    (_, Some(rect)) => rect,
                };
                row = Some(row_rect);
                let cell = grid.cell(row_rect.top(), center_x, column, rtl);
                let response = match visible.get(k) {
                    Some((index, tools)) => self.slot(ui, &slots[*index], tools, current, last_used, cell, out),
                    None => {
                        let label = self.edit_toolbar.as_deref().unwrap_or_default();
                        let response = IconButton::new(Icon::More, label).with_id(rail.with("edit-toolbar")).with_style(style).show_at(ui, cell);
                        out.edit_toolbar = response.clicked();
                        Some(response)
                    }
                };
                if let Some(response) = response {
                    if response.gained_focus() {
                        response.scroll_to_me(None);
                    }
                    cells.push((response.id, response.enabled()));
                }
            }
            if out.flyout.is_none() {
                Self::arrows(ui, grid.columns, &cells);
            }
        });
        let width = ui.max_rect().width();
        let inset = grid.pad_x.min(width / 2.0);
        let top = ui.cursor().top();
        let area = Rect::from_min_max(pos2(ui.max_rect().left() + inset, top), pos2(ui.max_rect().right() - inset, top.max(ui.max_rect().bottom())));
        let columns = grid.columns;
        let shown = ui
            .scope_builder(UiBuilder::new().id_salt("footer").max_rect(area).layout(direction::column(ui)), |ui| footer(ui, RailFooter { columns }));
        let measured = shown.response.rect.height();
        ui.ctx().data_mut(|d| d.insert_temp(footer_id, measured));
        if footer_height.is_none_or(|used| (used - measured).abs() > 0.5) {
            ui.ctx().request_discard("ToolRail footer height");
        }
    }

    /// One slot's button at `cell`. `None` for a slot without tools.
    #[allow(clippy::too_many_arguments)]
    fn slot(
        &self,
        ui: &mut Ui,
        slot: &ToolSlot<T>,
        tools: &[&RailTool<T>],
        current: &T,
        last_used: &mut LastUsed<T>,
        cell: Rect,
        out: &mut ToolRailOutput<T>,
    ) -> Option<Response> {
        let selected = tools.iter().any(|tool| tool.id == *current);
        if selected {
            last_used.set(slot.id, current.clone());
        }
        let shown = shown_tool(current, slot.id, tools, last_used)?;
        let items: Vec<FlyoutItem<T>> = tools
            .iter()
            .map(|tool| {
                let item = FlyoutItem::new(tool.id.clone(), tool.icon, tool.label.as_str()).enabled(tool.enabled);
                match &tool.shortcut {
                    Some(shortcut) => item.shortcut(shortcut.as_str()),
                    None => item,
                }
            })
            .collect();
        let builder = UiBuilder::new().id_salt(("tool-slot", slot.id)).max_rect(cell).layout(Layout::top_down(Align::Min));
        let flyout = ui
            .scope_builder(builder, |ui| {
                let mut button = FlyoutButton::new("button", shown.icon, shown.label.as_str())
                    .items(&items)
                    .current(&shown.id)
                    .selected(selected)
                    .enabled(tools.iter().any(|tool| tool.enabled));
                if let Some(shortcut) = &shown.shortcut {
                    button = button.shortcut(shortcut.as_str());
                }
                button.show(ui)
            })
            .inner;
        if let Some(tool) = flyout.chosen {
            out.select = Some(tool);
        } else if flyout.clicked && shown.enabled {
            out.select = Some(shown.id.clone());
        }
        if flyout.double_clicked {
            out.double_clicked = Some(shown.id.clone());
        }
        if flyout.open {
            out.flyout = Some(slot.id);
        }
        Some(flyout.response)
    }

    /// Moves the focus between the rail's buttons by the grid. The keys the focused button's
    /// move uses are kept from egui's geometric focus movement, so the focus stays in the rail.
    fn arrows(ui: &mut Ui, columns: usize, cells: &[(Id, bool)]) {
        let Some(index) = cells.iter().position(|(id, _)| ui.memory(|m| m.has_focus(*id))) else {
            return;
        };
        let two = columns > 1;
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(cells[index].0, EventFilter { vertical_arrows: true, horizontal_arrows: two, ..Default::default() })
        });
        let (previous, next) = if direction::is_rtl(ui) { (Key::ArrowRight, Key::ArrowLeft) } else { (Key::ArrowLeft, Key::ArrowRight) };
        let movement = ui.input_mut(|input| {
            if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                Some(GridMove::Down)
            } else if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                Some(GridMove::Up)
            } else if two && input.consume_key(Modifiers::NONE, previous) {
                Some(GridMove::Previous)
            } else if two && input.consume_key(Modifiers::NONE, next) {
                Some(GridMove::Next)
            } else {
                None
            }
        });
        let enabled: Vec<bool> = cells.iter().map(|(_, enabled)| *enabled).collect();
        if let Some(target) = movement.and_then(|movement| grid_step(index, &enabled, columns, movement)) {
            let filter = EventFilter { vertical_arrows: true, horizontal_arrows: two, ..Default::default() };
            ui.memory_mut(|m| m.request_focus_with_filter(cells[target].0, filter));
        }
    }
}

/// The column toggle's icon. Widening (from one column) points away from the rail's edge, towards
/// the canvas; narrowing points back to the edge. Directional icons mirror in a right-to-left
/// interface, so the icon that draws pointing right there is the left one.
pub(super) fn chevron(columns: usize, left: bool, rtl: bool) -> Icon {
    let points_right = (columns == 1) == left;
    if points_right != rtl { Icon::ChevronsRight } else { Icon::ChevronsLeft }
}

/// The sizes the grid of buttons is laid out with.
struct Grid {
    columns: usize,
    button: f32,
    gap: f32,
    row_gap: f32,
    toggle_height: f32,
    toggle_icon: f32,
    pad_x: f32,
    /// The rail is against the left edge of the window.
    left: bool,
}

impl Grid {
    /// The rect of the button in `column` of the row at `top`: the buttons are centred as a group
    /// on the panel's full width (`center_x`, not what a solid scroll bar leaves), and the first column is at the reading start.
    fn cell(&self, top: f32, center_x: f32, column: usize, rtl: bool) -> Rect {
        let used = self.columns as f32 * self.button + (self.columns - 1) as f32 * self.gap;
        let start = center_x - used / 2.0;
        let offset = column as f32 * (self.button + self.gap);
        let x = if rtl { start + used - self.button - offset } else { start + offset };
        Rect::from_min_size(pos2(x, top), vec2(self.button, self.button))
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ToolRail"];

type StoryTool = &'static str;

fn story_slots() -> Vec<ToolSlot<StoryTool>> {
    let tool = |id, icon, label: &str, key: &str| RailTool::new(id, icon, label).shortcut(key);
    vec![
        ToolSlot::new(0, vec![tool("move", Icon::Move, "Move", "V")]),
        ToolSlot::new(
            1,
            vec![
                tool("marquee", Icon::MarqueeRect, "Rectangular Marquee", "M"),
                tool("ellipse", Icon::MarqueeEllipse, "Elliptical Marquee", "M"),
                RailTool::new("row", Icon::MarqueeRow, "Single Row Marquee"),
                RailTool::new("column", Icon::MarqueeColumn, "Single Column Marquee"),
            ],
        ),
        ToolSlot::new(2, vec![tool("lasso", Icon::Lasso, "Lasso", "L"), tool("magnetic", Icon::LassoMagnetic, "Magnetic Lasso", "L")]),
        ToolSlot::new(3, vec![tool("crop", Icon::Crop, "Crop", "C"), tool("perspective", Icon::PerspectiveCrop, "Perspective Crop", "C")]),
        ToolSlot::new(4, vec![tool("eyedropper", Icon::Eyedropper, "Eyedropper", "I"), tool("ruler", Icon::Ruler, "Ruler", "I")]),
        ToolSlot::new(5, vec![tool("brush", Icon::Brush, "Brush", "B"), tool("pencil", Icon::Pencil, "Pencil", "B")]),
        ToolSlot::new(6, vec![tool("stamp", Icon::Stamp, "Clone Stamp", "S")]),
        ToolSlot::new(
            7,
            vec![
                tool("eraser", Icon::Eraser, "Eraser", "E"),
                tool("background", Icon::EraserBackground, "Background Eraser", "E"),
                tool("magic", Icon::EraserMagic, "Magic Eraser", "E"),
            ],
        ),
        ToolSlot::new(8, vec![tool("gradient", Icon::Gradient, "Gradient", "G"), tool("bucket", Icon::PaintBucket, "Paint Bucket", "G")]),
        ToolSlot::new(9, vec![tool("pen", Icon::PenTool, "Pen", "P"), tool("freeform", Icon::PenFreeform, "Freeform Pen", "P")]),
        ToolSlot::new(10, vec![tool("hand", Icon::Hand, "Hand", "H")]),
    ]
}

const HIDDEN: &[&str] = &["none", "pencil", "stamp", "eraser tools"];

/// PhotoCraft's slot list. The column toggle reports in the line under the rail.
fn show_story(ui: &mut Ui, args: &mut Args) {
    let columns = args.number("columns").round() as usize;
    let height = args.number("height") as f32;
    let hidden: Vec<StoryTool> = match args.choice("hidden") {
        "pencil" => vec!["pencil"],
        "stamp" => vec!["stamp"],
        "eraser tools" => vec!["eraser", "background", "magic"],
        _ => Vec::new(),
    };
    let state = ui.make_persistent_id("tool-rail-story");
    let current = ui.data(|d| d.get_temp::<StoryTool>(state)).unwrap_or("brush");
    let mut last_used = ui.data(|d| d.get_temp::<LastUsed<StoryTool>>(state.with("last"))).unwrap_or_default();
    let mode = ui.data(|d| d.get_temp::<StoryTool>(state.with("mode"))).unwrap_or("standard");
    let slots = story_slots();
    let mut new_mode = None;
    let out = ui
        .allocate_ui(vec2(160.0, height), |ui| {
            ToolRail::new("story").columns(columns).column_toggle("Toolbar").edit_toolbar("Edit Toolbar…").hidden(&hidden).label("Tools").show(
                ui,
                &slots,
                &current,
                &mut last_used,
                |ui, footer| {
                    ui.with_layout(if footer.columns > 1 { direction::row(ui) } else { direction::column(ui) }, |ui| {
                        let style = FlyoutButton::<StoryTool>::style(&Tokens::of(ui)).icon_button;
                        ui.add(IconButton::new(Icon::LayerMask, "Quick Mask Mode").with_style(style).shortcut("Q"));
                        let modes = [
                            FlyoutItem::new("standard", Icon::AppWindow, "Standard Screen Mode"),
                            FlyoutItem::new("full", Icon::Maximize2, "Full Screen Mode"),
                        ];
                        let (icon, label) =
                            if mode == "full" { (Icon::Maximize2, "Full Screen Mode") } else { (Icon::AppWindow, "Standard Screen Mode") };
                        let button = FlyoutButton::new("screen-mode", icon, label).items(&modes).current(&mode).shortcut("F");
                        let response = button.show(ui);
                        new_mode = response.chosen.or_else(|| response.clicked.then_some(if mode == "full" { "standard" } else { "full" }));
                    });
                },
            )
        })
        .inner;
    if let Some(mode) = new_mode {
        ui.data_mut(|d| d.insert_temp(state.with("mode"), mode));
    }
    let mut note = ui.data(|d| d.get_temp::<String>(state.with("note"))).unwrap_or_else(|| "Nothing yet".to_owned());
    if let Some(tool) = out.select {
        ui.data_mut(|d| d.insert_temp(state, tool));
        note = format!("selected {tool}");
    } else if let Some(tool) = out.double_clicked {
        note = format!("double clicked {tool}");
    } else if out.toggle_columns {
        note = "toggle columns".to_owned();
    } else if out.edit_toolbar {
        note = "edit toolbar".to_owned();
    }
    ui.data_mut(|d| {
        d.insert_temp(state.with("last"), last_used);
        d.insert_temp(state.with("note"), note.clone());
    });
    ui.label(format!("{note}; flyout {}", out.flyout.map_or_else(|| "closed".to_owned(), |slot| format!("of slot {slot} open"))));
}

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_number("columns", 1.0, 1.0, 2.0).with_choice("hidden", HIDDEN, 0).with_number("height", 480.0, 160.0, 700.0);
    vec![Story::new("patterns/rail/tool", Level::Patterns, "ToolRail", "Tool rail", args, show_story).on(Surface::Chrome)]
}
