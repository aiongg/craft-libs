//! [`GradientPalette`]: the grid or list of gradient presets, in groups, that picks one.

use std::ops::Range;

use craft_theme::ControlSize;
use egui::accesskit::{Action, Role};
use egui::emath::GuiRounding as _;
use egui::epaint::RectShape;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    AsIdSalt, CornerRadius, CursorIcon, EventFilter, Id, IdSalt, Key, Modifiers, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2,
    vec2,
};

use super::ramp::GradientRamp;
use super::stops::{Sampler, default_sampler};
use super::value::{GradientStops, StopRows};
use crate::behavior::{GridKey, GridNav};
use crate::controls::{ListboxStyle, Select, Swatch, TooltipExt as _};
use crate::direction;
use crate::edit::{EditEvent, EditResponse};
use crate::presets::PresetStyle;
use crate::primitives::{Content, FocusRing, Gap, Interactive, Text};
use crate::theme::{Tokens, TokensOf, role_format};
use craft_theme::TypeRole;

/// Room round a grid cell for the selection ring and the focus ring, as [`Swatch`] palettes
/// leave: cells stand twice this apart.
const ROOM: f32 = 3.0;

/// How a [`GradientPalette`] lays its presets out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PaletteLayout {
    /// Gradient cells wrapped to the width, a run of rows under each group's label.
    #[default]
    Grid,
    /// A row per preset: the gradient, then its name.
    List,
}

/// A named run of presets.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientGroup {
    pub name: String,
    pub items: Vec<GradientItem>,
}

impl GradientGroup {
    pub fn new(name: impl Into<String>, items: Vec<GradientItem>) -> Self {
        Self { name: name.into(), items }
    }
}

/// A named gradient preset.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientItem {
    pub name: String,
    pub stops: GradientStops,
}

impl GradientItem {
    pub fn new(name: impl Into<String>, stops: GradientStops) -> Self {
        Self { name: name.into(), stops }
    }
}

/// The preset a pick names: a recent gradient by its place in the list the host passed, or an
/// item of a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GradientRef {
    Recent(usize),
    Item { group: usize, item: usize },
}

/// A palette of gradient presets in groups, with an optional Recent group first:
///
/// ```
/// use craft_theme::Srgba;
/// use craft_ui::composites::{ColorStop, GradientGroup, GradientItem, GradientPalette, GradientStops};
/// # egui::__run_test_ui(|ui| {
/// let stops = GradientStops::new(vec![ColorStop::new(0.0, Srgba::rgb(0.0, 0.0, 0.0)), ColorStop::new(1.0, Srgba::rgb(1.0, 1.0, 1.0))]);
/// let groups = [GradientGroup::new("Basics", vec![GradientItem::new("Black, White", stops)])];
/// let picked = GradientPalette::new("presets", "Gradient presets", &groups).show(ui);
/// if let Some(pick) = picked.event.as_ref().and_then(|e| e.value()) {
///     // copy the preset's stops into the host's gradient
///     let _ = pick;
/// }
/// # });
/// ```
///
/// - A click, Space or Enter on a cell or row is one [`Set`](crate::edit::EditEvent::Set) of its
///   [`GradientRef`], also when it is the selected one. The palette keeps no pick state; [`selected`](Self::selected) only draws a
///   ring (Grid) or the chosen fill (List).
/// - The name is the tooltip and the accessible name. Recent entries are named "{label} N" and
///   have no tooltip.
/// - One Tab stop: the selected cell, else the first. In Grid the arrows move the focus without
///   picking: Left and Right along a row and on to the next or previous row (mirrored right to
///   left; the gradients are not), Up and Down to the nearest cell in the row above or below
///   across group labels, Home and End to the first and last. In List, Up, Down, Home and End
///   move and Left and Right do nothing. A focus move scrolls the cell into view.
/// - The area scrolls past [`max_height`](Self::max_height).
/// - AccessKit: Grid is a `Group` named `label` with `Button` cells; List is a `ListBox` of
///   `ListBoxOption`s with `selected` on the chosen one. Cells carry their place and the set's
///   size. A screen reader's Focus action on a cell moves the palette's stop to it.
#[must_use = "show the palette with `show`"]
pub struct GradientPalette<'a> {
    id_salt: IdSalt,
    label: String,
    groups: &'a [GradientGroup],
    recent: &'a [GradientStops],
    recent_label: String,
    layout: PaletteLayout,
    rows: StopRows,
    sampler: Option<&'a Sampler>,
    selected: Option<GradientRef>,
    max_height: Option<f32>,
    enabled: bool,
}

/// One preset as shown, in reading order.
struct Entry<'a> {
    key: GradientRef,
    name: String,
    tooltip: bool,
    stops: &'a GradientStops,
}

impl<'a> GradientPalette<'a> {
    /// The words the palette has no host string for, as the host's English default: the label
    /// to pass to [`recent`](Self::recent).
    pub const TEXTS: [&'static str; 1] = ["Recent"];

    /// A palette named `label` (its accessible name) of `groups`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, groups: &'a [GradientGroup]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            groups,
            recent: &[],
            recent_label: String::new(),
            layout: PaletteLayout::Grid,
            rows: StopRows::Split,
            sampler: None,
            selected: None,
            max_height: None,
            enabled: true,
        }
    }

    /// A first group of the gradients the host used last, newest first, headed `label`; its
    /// entries are named "{label} 1", "{label} 2" and have no tooltip.
    pub fn recent(mut self, stops: &'a [GradientStops], label: impl Into<String>) -> Self {
        self.recent = stops;
        self.recent_label = label.into();
        self
    }

    /// Grid or List. Default [`PaletteLayout::Grid`].
    pub fn layout(mut self, layout: PaletteLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The row layout, for the default sampler. Default [`StopRows::Split`].
    pub fn rows(mut self, rows: StopRows) -> Self {
        self.rows = rows;
        self
    }

    /// Samples the gradients instead of the default [`default_sampler`].
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// The preset to mark as the host's current one. None by default.
    pub fn selected(mut self, selected: Option<GradientRef>) -> Self {
        self.selected = selected;
        self
    }

    /// The height past which the area scrolls. Default `gradient_palette.max_height`.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Whether the palette takes input; a disabled one draws faded edges.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the palette and returns this frame's edit: a [`Set`](crate::edit::EditEvent::Set) of
    /// the preset picked. The response covers the area and is [changed](egui::Response::changed)
    /// with it.
    pub fn show(self, ui: &mut Ui) -> EditResponse<GradientRef> {
        let id = ui.make_persistent_id(self.id_salt);
        let tokens = Tokens::of(ui);
        let enabled = self.enabled && ui.is_enabled();
        let list = self.layout == PaletteLayout::List;

        let mut entries: Vec<Entry<'_>> = Vec::new();
        let mut shown: Vec<(&str, Range<usize>)> = Vec::new();
        if !self.recent.is_empty() {
            let start = entries.len();
            for (i, stops) in self.recent.iter().enumerate() {
                entries.push(Entry { key: GradientRef::Recent(i), name: format!("{} {}", self.recent_label, i + 1), tooltip: false, stops });
            }
            shown.push((self.recent_label.as_str(), start..entries.len()));
        }
        for (g, group) in self.groups.iter().enumerate().filter(|(_, g)| !g.items.is_empty()) {
            let start = entries.len();
            for (i, item) in group.items.iter().enumerate() {
                entries.push(Entry { key: GradientRef::Item { group: g, item: i }, name: item.name.clone(), tooltip: true, stops: &item.stops });
            }
            shown.push((group.name.as_str(), start..entries.len()));
        }
        let selected = self.selected.and_then(|s| entries.iter().position(|e| e.key == s));
        let ids: Vec<Id> = (0..entries.len()).map(|i| id.with(("preset", i))).collect();
        let total = entries.len();

        let role = if list { Role::ListBox } else { Role::Group };
        let builder = UiBuilder::new().scope_id(id).layout(direction::column(ui)).accessibility_role(role).accessibility_label(self.label.clone());
        let max_height = self.max_height.unwrap_or(tokens.component.gradient_palette.max_height);
        let inner = ui.scope_builder(builder, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, Gap::Inline.points(&tokens));
            ui.ctx().accesskit_node_builder(id, |node| node.set_size_of_set(total));
            ScrollArea::vertical()
                .id_salt("scroll")
                .max_height(max_height)
                .auto_shrink([false, true])
                .show(ui, |ui| self.cells(ui, id, &entries, &shown, &ids, selected, enabled))
                .inner
        });
        let picked = inner.inner;
        let mut response = inner.response;
        // Not a `Gesture::set`: picking the preset that is already selected still copies it, as
        // the host may have edited its gradient since.
        let event = picked.map(|i| {
            response.mark_changed();
            EditEvent::set(entries[i].key, ui.input(|i| i.modifiers))
        });
        EditResponse::new(response, event)
    }

    #[allow(clippy::too_many_arguments)]
    fn cells(
        &self,
        ui: &mut Ui,
        id: Id,
        entries: &[Entry<'_>],
        shown: &[(&str, Range<usize>)],
        ids: &[Id],
        selected: Option<usize>,
        enabled: bool,
    ) -> Option<usize> {
        let tokens = Tokens::of(ui);
        let t = tokens.component.gradient_palette;
        let list = self.layout == PaletteLayout::List;
        let rtl = direction::is_rtl(ui);
        let width = ui.available_width();
        let (slot, columns) = if list {
            (vec2(width, 0.0), 1)
        } else {
            let slot = vec2(t.cell_width + 2.0 * ROOM, t.cell_height + 2.0 * ROOM);
            (slot, GridNav::columns(width, 0.0, 0.0, slot.x))
        };
        let nav = if list {
            GridNav::wrapped(entries.len(), 1)
        } else {
            GridNav::ragged(shown.iter().flat_map(|(_, r)| {
                let n = r.len();
                (0..n.div_ceil(columns)).map(move |row| (n - row * columns).min(columns))
            }))
        }
        .rtl(rtl);
        let (stop, moved) = roving_stop(ui, id, ids, &nav, selected.unwrap_or(0), enabled, list);

        let swatch = Swatch::style(&tokens, ControlSize::Md);
        let ramp = GradientRamp::style(&tokens);
        let preset = PresetStyle::from_tokens(&tokens);
        let default = default_sampler(self.rows);
        let sampler: &Sampler = self.sampler.unwrap_or(&default);
        let listbox = Select::style(&tokens, ControlSize::Md).list;
        let mut picked = None;
        for (label, range) in shown {
            if !label.is_empty() {
                ui.with_layout(direction::column(ui), |ui| ui.add(Text::new(*label).role(TypeRole::Caption).content(Content::Muted)));
            }
            let height = if list { listbox.height.max(t.list_swatch_height + 4.0) } else { slot.y * range.len().div_ceil(columns) as f32 };
            let area = if list { ui.allocate_space(vec2(width, height * range.len() as f32)).1 } else { ui.allocate_space(vec2(width, height)).1 };
            for (k, i) in range.clone().enumerate() {
                let entry = &entries[i];
                let rect = if list {
                    Rect::from_min_size(area.min + vec2(0.0, k as f32 * height), vec2(width, height))
                } else {
                    let (row, col) = (k / columns, k % columns);
                    let x = if rtl { area.right() - (col + 1) as f32 * slot.x } else { area.left() + col as f32 * slot.x };
                    Rect::from_min_size(pos2(x, area.top() + row as f32 * slot.y), slot)
                };
                let cell = if list { rect } else { rect.shrink(ROOM) };
                let mut element = Interactive::with_id(ids[i], if list { Role::ListBoxOption } else { Role::Button }, entry.name.as_str())
                    .sense(if i == stop { Sense::click() } else { Sense::CLICK })
                    .state_layer(false)
                    .focus_ring(false)
                    .position_in_set(i)
                    .enabled(enabled);
                if list {
                    element = element.selected(selected == Some(i));
                }
                let response = element.interact_at(ui, cell).finish(ui);
                if i != stop && enabled {
                    ui.ctx().accesskit_node_builder(response.id, |node| node.add_action(Action::Focus));
                }
                if moved && i == stop {
                    ui.scroll_to_rect(rect, None);
                }
                if ui.is_rect_visible(rect) {
                    let colors = GradientRamp::colors(entry.stops, sampler);
                    if list {
                        paint_row(
                            ui,
                            &listbox,
                            &swatch.radius,
                            &ramp,
                            &colors,
                            (t.list_swatch_width, t.list_swatch_height),
                            &response,
                            selected == Some(i),
                            enabled,
                            &entry.name,
                            &tokens,
                        );
                    } else {
                        GradientRamp::paint_rounded(ui.painter(), cell, f32::from(swatch.radius.nw), &colors, &ramp);
                        ui.painter().extend(Swatch::outline(&swatch, cell, false, enabled, ui.pixels_per_point()));
                        if selected == Some(i) {
                            paint_selected(ui, &preset, swatch.radius, cell, enabled);
                        }
                        FocusRing::paint_radius(ui, cell, swatch.radius, &response);
                    }
                }
                let response = if enabled { response.on_hover_cursor(CursorIcon::PointingHand) } else { response };
                if entry.tooltip {
                    response.clone().tooltip(entry.name.as_str());
                }
                if enabled && response.clicked() {
                    picked = Some(i);
                }
            }
        }
        picked
    }
}

/// The selection ring outside `cell`: the preset browser's, an accent ring and a halo outside it.
fn paint_selected(ui: &Ui, style: &PresetStyle, radius: CornerRadius, cell: Rect, enabled: bool) {
    let fade = |s: Stroke| if enabled { s } else { Stroke::new(s.width, s.color.gamma_multiply(0.5)) };
    let cell = cell.round_to_pixels(ui.pixels_per_point());
    let grown = |by: f32| CornerRadius {
        nw: radius.nw.saturating_add(by as u8),
        ne: radius.ne.saturating_add(by as u8),
        sw: radius.sw.saturating_add(by as u8),
        se: radius.se.saturating_add(by as u8),
    };
    let ring = style.ring_selected.width;
    ui.painter().add(RectShape::stroke(cell.expand(ring), grown(ring), fade(style.ring_halo), StrokeKind::Outside));
    ui.painter().add(RectShape::stroke(cell, radius, fade(style.ring_selected), StrokeKind::Outside));
}

/// A list row as a select's popup draws one: the chosen and the lit row filled, the keyboard
/// focus a ring inside the edge, the gradient swatch then the name.
#[allow(clippy::too_many_arguments)]
fn paint_row(
    ui: &Ui,
    s: &ListboxStyle,
    swatch_radius: &CornerRadius,
    ramp: &super::ramp::GradientRampStyle,
    colors: &[(f32, craft_theme::Srgba)],
    swatch: (f32, f32),
    response: &egui::Response,
    chosen: bool,
    enabled: bool,
    name: &str,
    tokens: &crate::theme::UiTokens,
) {
    let ppp = ui.pixels_per_point();
    let rect = response.rect.round_to_pixels(ppp);
    let lit = enabled && response.contains_pointer();
    let (fill, ink, border) = match () {
        () if chosen => (Some(s.chosen_fill), s.chosen_text, Some(s.chosen_border)),
        () if lit => (Some(s.active_fill), s.active_text, Some(s.active_border)),
        () => (None, if enabled { s.text } else { s.text_disabled }, None),
    };
    let painter = ui.painter();
    if let Some(fill) = fill.filter(|_| enabled) {
        painter.rect_filled(rect, s.radius, fill);
        if let Some(border) = border {
            painter.add(RectShape::stroke(rect, s.radius, Stroke::new(1.0, border), StrokeKind::Inside));
        }
    }
    if enabled && response.has_focus() && crate::primitives::FocusRing::visible(ui.ctx()) {
        painter.add(RectShape::stroke(rect, s.radius, s.focus, StrokeKind::Inside));
    }
    let forward = direction::forward(ui);
    let start = direction::start_x(ui, rect) + forward * s.pad_x;
    let (sw, sh) = swatch;
    let well = {
        let x = if forward < 0.0 { start - sw } else { start };
        Rect::from_min_size(pos2(x, rect.center().y - sh / 2.0), vec2(sw, sh)).round_to_pixels(ppp)
    };
    GradientRamp::paint_rounded(painter, well, f32::from(swatch_radius.nw), colors, ramp);
    let edge = Swatch::style(tokens, ControlSize::Md);
    painter.extend(Swatch::outline(&edge, well, false, enabled, ppp));
    let text_start = start + forward * (sw + s.gap);
    let room = (rect.width() - s.pad_x * 2.0 - sw - s.gap).max(0.0);
    let mut job = LayoutJob::single_section(name.to_owned(), role_format(tokens, TypeRole::Body, ink));
    job.wrap = TextWrapping::truncate_at_width(room);
    let galley = painter.layout_job(job);
    let x = if forward < 0.0 { text_start - galley.size().x } else { text_start };
    painter.galley(pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp), galley, ink);
}

/// The cell that is the palette's Tab stop this frame, moved by the arrow keys, Home and End
/// (which it takes) or a screen reader's Focus request, as [`GridNav::roving_stop`] does, over
/// the layout `nav` describes. Up and Down keep aiming at the column they started in. Returns the
/// stop and whether a key moved it.
fn roving_stop(ui: &Ui, id: Id, ids: &[Id], nav: &GridNav, resting: usize, enabled: bool, list: bool) -> (usize, bool) {
    if !enabled || ids.is_empty() {
        return (resting.min(ids.len().saturating_sub(1)), false);
    }
    let resting = resting.min(ids.len() - 1);
    let focused = ids.iter().position(|&i| ui.memory(|m| m.has_focus(i)));
    let mut stop = focused.unwrap_or(resting);
    if let Some(requested) = ids.iter().position(|&i| ui.input(|i2| i2.has_accesskit_action_request(i, Action::Focus))) {
        stop = requested;
    }
    let goal_key = id.with("goal");
    let mut moved = false;
    if let Some(from) = focused {
        ui.memory_mut(|m| m.set_focus_lock_filter(ids[from], EventFilter { horizontal_arrows: !list, vertical_arrows: true, ..Default::default() }));
        let mut goal = ui.data(|d| d.get_temp::<(usize, usize)>(goal_key)).filter(|&(at, _)| at == from).map(|(_, g)| g);
        let keys: &[(Key, GridKey)] = if list {
            &[(Key::ArrowDown, GridKey::Down), (Key::ArrowUp, GridKey::Up), (Key::Home, GridKey::Home), (Key::End, GridKey::End)]
        } else {
            &[
                (Key::ArrowRight, GridKey::Right),
                (Key::ArrowLeft, GridKey::Left),
                (Key::ArrowDown, GridKey::Down),
                (Key::ArrowUp, GridKey::Up),
                (Key::Home, GridKey::Home),
                (Key::End, GridKey::End),
            ]
        };
        ui.input_mut(|i| {
            for &(key, grid_key) in keys {
                if i.consume_key(Modifiers::NONE, key) {
                    let step = nav.step(stop, grid_key, goal);
                    stop = step.index;
                    goal = matches!(grid_key, GridKey::Up | GridKey::Down).then_some(step.goal);
                }
            }
        });
        moved = stop != from;
        ui.data_mut(|d| d.insert_temp(goal_key, (stop, goal.unwrap_or_else(|| nav.column_of(stop)))));
    }
    if focused.is_some_and(|f| f != stop) || (focused.is_none() && stop != resting) {
        ui.memory_mut(|m| m.request_focus(ids[stop]));
    }
    (stop, moved)
}
