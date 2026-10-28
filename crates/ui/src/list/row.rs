//! [`ListRow`]: one row of a list.

use craft_theme::{Stroke as Hairline, ThumbSizes, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{Color32, CornerRadius, Id, Label, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

use super::rename;
use super::toggle::{self, Cell, ToggleColumn, ToggleEdit};
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::{self, Icon, IconSize};
use crate::primitives::{FocusRing, Interactive};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, role_format};

/// The thumbnail setting of a list (PhotoCraft's Layers Panel Options), which the host stores.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThumbSize {
    /// No thumbnail: rows are `list.row_height` high.
    #[default]
    None,
    /// `list.thumb.sm`.
    Small,
    /// `list.thumb.md`.
    Medium,
    /// `list.thumb.lg`.
    Large,
}

impl ThumbSize {
    /// Every setting.
    pub const ALL: [ThumbSize; 4] = [ThumbSize::None, ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large];

    /// The setting's name, for settings and stories.
    pub const fn name(self) -> &'static str {
        match self {
            ThumbSize::None => "None",
            ThumbSize::Small => "Small",
            ThumbSize::Medium => "Medium",
            ThumbSize::Large => "Large",
        }
    }
}

/// Where a drag would drop, drawn on the row that is the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropMark {
    /// A line along the row's top edge.
    Before,
    /// A line along the row's bottom edge.
    After,
    /// An outline inside the row.
    Into,
}

/// How a rename ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenameEnd {
    /// Enter.
    Enter,
    /// Tab: the list renames the next row.
    Tab,
    /// Shift+Tab: the list renames the previous row.
    ShiftTab,
    /// The focus went elsewhere.
    Blur,
    /// Esc: nothing is renamed.
    Escape,
    /// The row left the list: the text typed is committed.
    Removed,
}

/// What a row reports besides pointer input, which is on its response.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum RowEvent<K> {
    /// An edit of a toggle column; see [`ToggleEdit`].
    Toggle(EditEvent<ToggleEdit<K>>),
    /// A rename ended with text other than the label; the text may be empty and the host decides
    /// what that means. Reported before the [`RenameEnded`](RowEvent::RenameEnded) of the same
    /// frame.
    Renamed {
        /// The row.
        row: K,
        /// The new text.
        text: String,
    },
    /// The rename field closed, by commit or by Esc; the host stops passing `renaming(true)`.
    RenameEnded {
        /// The row.
        row: K,
        /// How it ended.
        end: RenameEnd,
    },
}

/// A trailing item of a row.
pub struct TrailingItem<'a> {
    width: f32,
    optional: bool,
    add: Box<dyn FnOnce(&mut Ui) + 'a>,
}

type Thumbnail<'a> = Box<dyn FnOnce(&Painter, Rect) + 'a>;

/// What [`ListRow::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the row's events"]
pub struct RowResponse<K> {
    /// The row: clicks, double clicks and drags, secondary clicks for a context menu. Over a
    /// toggle cell or a trailing widget, the cell or widget takes the input.
    pub response: Response,
    /// What the row reported this frame, in order.
    pub events: Vec<RowEvent<K>>,
    /// Where the label (and detail) is drawn, the full height of the row: a double click on it
    /// starts a rename.
    pub label: Rect,
}

/// A row's sizes and colours, from [`ListRow::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListRowStyle {
    /// Height of a row without a thumbnail: `list.row_height`.
    pub row_height: f32,
    /// Thumbnail sides: `list.thumb`.
    pub thumb: ThumbSizes,
    /// Added to the thumbnail for the row's height: `list.thumb_pad`.
    pub thumb_pad: f32,
    /// Least height of a row with a thumbnail: `list.thumb_row_min`.
    pub thumb_row_min: f32,
    /// `list.pad_x`.
    pub pad_x: f32,
    /// `list.gap`.
    pub gap: f32,
    /// `list.indent`.
    pub indent: f32,
    /// `list.toggle_column`.
    pub toggle_column: f32,
    /// `list.min_label`.
    pub min_label: f32,
    /// `list.trailing_pad`.
    pub trailing_pad: f32,
    /// `list.radius`.
    pub radius: f32,
    /// `list.dividers`.
    pub dividers: bool,
    /// `list.drop_width`.
    pub drop_width: f32,
    /// Side of an icon: `size.icon.md`.
    pub icon_size: f32,
    /// Type role of the label: `list.label`.
    pub label_role: TypeRole,
    /// Type role of the detail line: `list.detail_role`.
    pub detail_role: TypeRole,
    /// Line height of the label and of the detail line.
    pub label_line: f32,
    pub detail_line: f32,
    /// `list.text`, `list.text.dimmed`, `list.text.selected` and `list.detail`.
    pub text: Color32,
    pub dimmed: Color32,
    pub selected_text: Color32,
    pub detail: Color32,
    /// `list.icon` and `list.icon.hover`.
    pub icon: Color32,
    pub icon_hover: Color32,
    /// `list.hover`, `list.selected` and `list.selected.hover`.
    pub hover: Color32,
    pub selected: Color32,
    pub selected_hover: Color32,
    /// The hairlines: `list.divider`, one physical pixel.
    pub divider: Stroke,
    /// `list.thumb.border`.
    pub thumb_border: Color32,
    /// `list.drop`.
    pub drop: Color32,
}

impl ListRowStyle {
    /// The height of a row with a thumbnail of `thumb` and, if `detail`, a detail line: the row
    /// height, or the thumbnail row, or the two lines of text, whichever is tallest.
    pub fn height(&self, thumb: ThumbSize, detail: bool) -> f32 {
        let base = match self.thumb_side(thumb) {
            0.0 => self.row_height,
            side => (side + self.thumb_pad).max(self.thumb_row_min),
        };
        if detail { base.max(self.label_line + self.detail_line + self.gap) } else { base }
    }

    /// The side of the thumbnail of `thumb`; 0 for none.
    pub fn thumb_side(&self, thumb: ThumbSize) -> f32 {
        match thumb {
            ThumbSize::None => 0.0,
            ThumbSize::Small => self.thumb.sm,
            ThumbSize::Medium => self.thumb.md,
            ThumbSize::Large => self.thumb.lg,
        }
    }
}

/// One row of a list.
///
/// ```
/// use craft_ui::list::{ListRow, ToggleColumn};
/// # egui::__run_test_ui(|ui| {
/// let row = ListRow::new("layer-1", "Layer 1")
///     .toggle(ToggleColumn::eye("Visibility", true))
///     .selected(true)
///     .show(ui);
/// if row.response.double_clicked() {
///     // The host starts a rename: `.renaming(true)` on the next frames.
/// }
/// # });
/// ```
///
/// - **Anatomy**, in reading order: [toggle columns](ListRow::toggle), the
///   [indent](ListRow::indent), a [thumbnail](ListRow::thumb) or [icon](ListRow::icon), the label
///   with an optional [detail](ListRow::detail) line under it, the trailing items. The row
///   mirrors in a right-to-left interface.
/// - **States** are inputs: [`selected`](ListRow::selected) (the neutral `state.selected` fill,
///   rounded in Comfortable, no border), [`focused`](ListRow::focused) (the focus ring, drawn
///   while the keyboard is the last input), [`dimmed`](ListRow::dimmed) (a hidden layer's faint
///   label), [`highlight`](ListRow::highlight) (the hover fill without the pointer),
///   [`drop_mark`](ListRow::drop_mark) and [`renaming`](ListRow::renaming).
/// - **Label.** One line, cut with an ellipsis, showing the whole text as a tooltip. Trailing
///   items are placed first, from the trailing edge inward; those marked
///   [optional](ListRow::optional_item) drop out, in the order given, before the label would
///   get narrower than `list.min_label`, and the indent shrinks before the label gets narrower
///   than that.
/// - **Rename.** While [`renaming`](ListRow::renaming), a [`TextField`](crate::controls::TextField)
///   with the whole label selected replaces the label and has the focus. Enter, Tab, Shift+Tab or
///   the focus leaving commit; Esc cancels; unchanged text renames nothing. A rename ends with
///   [`RowEvent::RenameEnded`]. The typed text is kept by the row, so the host changes the label
///   only on [`RowEvent::Renamed`].
/// - **AccessKit.** The row is a `ListBoxOption` (or the [role](ListRow::role) given) named by
///   its label, then its detail, with `selected` and `position_in_set`. It is not a Tab stop: the
///   list is the one focus stop. Each toggle cell is a button named "{column label} {row label}"
///   with its state, reachable by the pointer and the screen reader.
#[must_use = "show the row with `show`"]
pub struct ListRow<'a, K> {
    key: K,
    label: String,
    detail: Option<String>,
    columns: Vec<ToggleColumn>,
    icon: Option<Icon>,
    thumb: ThumbSize,
    thumbnail: Option<Thumbnail<'a>>,
    trailing: Vec<TrailingItem<'a>>,
    selected: bool,
    focused: bool,
    dimmed: bool,
    highlight: bool,
    renaming: bool,
    drop: Option<DropMark>,
    indent: f32,
    role: Role,
    position: Option<usize>,
    slot: Option<Rect>,
}

/// The id of the row for `key` among rows shown in a `Ui` whose scope is `scope`: the base of the
/// row's state (the open rename, the toggle cells).
pub(super) fn row_id<K: std::hash::Hash + std::fmt::Debug>(scope: Id, key: &K) -> Id {
    scope.with(("list-row", key))
}

impl<'a, K: std::hash::Hash + std::fmt::Debug + Clone> ListRow<'a, K> {
    /// A row for `key`, which keeps its state (the open rename) across frames and names it in
    /// events, showing `label`.
    pub fn new(key: K, label: impl Into<String>) -> Self {
        Self {
            key,
            label: label.into(),
            detail: None,
            columns: Vec::new(),
            icon: None,
            thumb: ThumbSize::None,
            thumbnail: None,
            trailing: Vec::new(),
            selected: false,
            focused: false,
            dimmed: false,
            highlight: false,
            renaming: false,
            drop: None,
            indent: 0.0,
            role: Role::ListBoxOption,
            position: None,
            slot: None,
        }
    }

    /// The row's key.
    pub(super) fn key(&self) -> &K {
        &self.key
    }

    /// The row's label.
    pub(super) fn label(&self) -> &str {
        &self.label
    }

    /// The row's toggle columns.
    pub(super) fn columns(&self) -> &[ToggleColumn] {
        &self.columns
    }

    /// Shows the row in `rect`, which a list places, without allocating it in the `Ui`.
    pub(super) fn show_at(mut self, ui: &mut Ui, rect: Rect) -> RowResponse<K> {
        self.slot = Some(rect);
        self.show(ui)
    }

    /// A second line under the label in `caption` and `text.faint` (PhotoCraft Studio's kind
    /// line).
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into()).filter(|d| !d.is_empty());
        self
    }

    /// Adds a toggle column at the reading start, after those added before it.
    pub fn toggle(mut self, column: ToggleColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// An icon before the label. A thumbnail takes its place.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A thumbnail of `size` before the label, framed in `list.thumb.border`; `paint` draws into
    /// the painter, clipped to the rect inside the frame. [`ThumbSize::None`] draws none.
    pub fn thumb(mut self, size: ThumbSize, paint: impl FnOnce(&Painter, Rect) + 'a) -> Self {
        self.thumb = size;
        self.thumbnail = Some(Box::new(paint));
        self
    }

    /// An item at the trailing end, `width` wide, that always shows. The first item added is the
    /// outermost.
    pub fn trailing_item(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.trailing.push(TrailingItem { width, optional: false, add: Box::new(add) });
        self
    }

    /// An item at the trailing end, `width` wide, that drops out when the label would get
    /// narrower than `list.min_label` with it.
    pub fn optional_item(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.trailing.push(TrailingItem { width, optional: true, add: Box::new(add) });
        self
    }

    /// The row is in the selection.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The row is the list's active row while the list has the focus: it draws the focus ring.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The label in `text.faint` (a hidden layer).
    pub fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }

    /// The hover look without the pointer (the canvas hovers the layer).
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// The rename field replaces the label.
    pub fn renaming(mut self, renaming: bool) -> Self {
        self.renaming = renaming;
        self
    }

    /// The drop marker of a drag in progress, if this row is its target.
    pub fn drop_mark(mut self, mark: Option<DropMark>) -> Self {
        self.drop = mark;
        self
    }

    /// Space before the icon or thumbnail, in points (a tree's levels and disclosure column). It
    /// shrinks before the label gets narrower than `list.min_label`.
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = indent.max(0.0);
        self
    }

    /// The AccessKit role: `ListBoxOption` by default.
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// The row's zero-based position among all of the list's rows, for AccessKit.
    pub fn position_in_set(mut self, index: usize) -> Self {
        self.position = Some(index);
        self
    }

    pub fn style(tokens: &UiTokens) -> ListRowStyle {
        let l = &tokens.component.list;
        ListRowStyle {
            row_height: l.row_height,
            thumb: l.thumb,
            thumb_pad: l.thumb_pad,
            thumb_row_min: l.thumb_row_min,
            pad_x: l.pad_x,
            gap: l.gap,
            indent: l.indent,
            toggle_column: l.toggle_column,
            min_label: l.min_label,
            trailing_pad: l.trailing_pad,
            radius: l.radius,
            dividers: l.dividers,
            drop_width: l.drop_width,
            icon_size: tokens.size.icon.md,
            label_role: l.label,
            detail_role: l.detail_role,
            label_line: tokens.typography.get(l.label).line_height,
            detail_line: tokens.typography.get(l.detail_role).line_height,
            text: l.text.to_egui(),
            dimmed: l.dimmed.to_egui(),
            selected_text: l.selected_text.to_egui(),
            detail: l.detail.to_egui(),
            icon: l.icon.to_egui(),
            icon_hover: l.icon_hover.to_egui(),
            hover: l.hover.to_egui(),
            selected: l.selected.to_egui(),
            selected_hover: l.selected_hover.to_egui(),
            divider: Stroke::new(Hairline::Hairline.width(1.0), l.divider.to_egui()),
            thumb_border: l.thumb_border.to_egui(),
            drop: l.drop.to_egui(),
        }
    }

    /// Shows the row across the available width.
    pub fn show(self, ui: &mut Ui) -> RowResponse<K> {
        let ListRow {
            key,
            label,
            detail,
            columns,
            icon,
            thumb,
            thumbnail,
            trailing,
            selected,
            focused,
            dimmed,
            highlight,
            renaming,
            drop,
            indent,
            role,
            position,
            slot,
        } = self;
        let tokens = Tokens::of(ui);
        let mut style = Self::style(&tokens);
        style.divider.width = Hairline::Hairline.width(ui.pixels_per_point());
        let height = style.height(thumb, detail.is_some());
        let rect = slot.unwrap_or_else(|| ui.allocate_space(vec2(ui.available_width(), height)).1);
        let row_id = row_id(ui.scope_id(), &key);
        let name = match &detail {
            Some(detail) => format!("{label}, {detail}"),
            None => label.clone(),
        };
        let mut element =
            Interactive::with_id(row_id, role, name).sense(Sense::CLICK | Sense::DRAG).selected(selected).state_layer(false).focus_ring(false);
        if let Some(position) = position {
            element = element.position_in_set(position);
        }
        let response = element.interact_at(ui, rect).finish(ui);

        let side = style.thumb_side(thumb);
        let leading = match (side > 0.0, icon) {
            (true, _) => Some(side),
            (false, Some(_)) => Some(style.icon_size),
            (false, None) => None,
        };
        let widths: Vec<(f32, bool)> = trailing.iter().map(|t| (t.width, t.optional)).collect();
        let geometry = Geometry::new(&style, rect, direction::is_rtl(ui), columns.len(), indent, leading, &widths);
        let visible = ui.is_rect_visible(rect);
        let hovered = ui.rect_contains_pointer(rect) && ui.is_enabled();
        if visible {
            paint_background(ui, &style, rect, selected, hovered || highlight, columns.len(), &geometry);
        }

        let mut events = Vec::new();
        for (index, column) in columns.iter().enumerate() {
            let cell = Cell { index, row_id, key: &key, row_label: &label };
            let at = geometry.span(rect, index as f32 * style.toggle_column, style.toggle_column);
            toggle::show(ui, &cell, column, at, &style, &mut events);
        }

        if let (Some(side), true) = (leading, visible) {
            let at = Rect::from_center_size(pos2(geometry.span(rect, geometry.leading_start, side).center().x, rect.center().y), vec2(side, side));
            let at = at.round_to_pixels(ui.pixels_per_point());
            if side > 0.0 && thumb != ThumbSize::None {
                paint_thumb(ui, &style, at, thumbnail);
            } else if let Some(icon) = icon {
                icons::paint_in(ui, at, icon, icons::active(ui.ctx()), style.icon_size, IconSize::Md.stroke(), style.icon);
            }
        }

        let band = geometry.label_band(rect);
        let text = if dimmed {
            style.dimmed
        } else if selected {
            style.selected_text
        } else {
            style.text
        };
        if renaming {
            rename::show(ui, row_id, &key, &label, band, &mut events);
        } else if visible {
            paint_label(ui, &tokens, &style, band, &label, detail.as_deref(), text);
        }
        for (item, at) in trailing.into_iter().zip(&geometry.items) {
            let Some((start, width)) = *at else { continue };
            let at = geometry.span(rect, start, width).round_to_pixels(ui.pixels_per_point());
            let mut child = ui.new_child(UiBuilder::new().max_rect(at).layout(direction::row(ui)));
            (item.add)(&mut child);
        }
        if let Some(mark) = drop {
            paint_drop_mark(ui, &style, rect, mark);
        }
        FocusRing::paint_if(ui, rect, corner_radius(style.radius), focused);
        RowResponse { response, events, label: band }
    }
}

/// Where the parts of a row lie, as offsets from the reading start of the row.
struct Geometry {
    rtl: bool,
    leading_start: f32,
    label_start: f32,
    label_end: f32,
    /// Offset and width of each trailing item; `None` for one that dropped out.
    items: Vec<Option<(f32, f32)>>,
}

impl Geometry {
    fn new(style: &ListRowStyle, rect: Rect, rtl: bool, columns: usize, indent: f32, leading: Option<f32>, items: &[(f32, bool)]) -> Geometry {
        let base = columns as f32 * style.toggle_column + style.pad_x;
        let end = rect.width() - style.trailing_pad;
        let leading_width = leading.map_or(0.0, |side| side + style.gap);
        // Trailing items are placed first. Optional ones stay while the label keeps its least
        // width at the full indent; the indent then shrinks for what is left.
        let mut used: f32 = items.iter().filter(|(_, optional)| !optional).map(|(w, _)| w + style.gap).sum();
        let mut kept: Vec<bool> = items.iter().map(|(_, optional)| !optional).collect();
        for (i, (width, optional)) in items.iter().enumerate() {
            if *optional && end - (used + width + style.gap) - (base + indent + leading_width) >= style.min_label {
                kept[i] = true;
                used += width + style.gap;
            }
        }
        let label_end = end - used;
        let indent = indent.min((label_end - base - leading_width - style.min_label).max(0.0));
        let mut from_end = 0.0;
        let items = items
            .iter()
            .zip(&kept)
            .map(|((width, _), kept)| {
                kept.then(|| {
                    let start = end - from_end - width;
                    from_end += width + style.gap;
                    (start, *width)
                })
            })
            .collect();
        Geometry { rtl, leading_start: base + indent, label_start: base + indent + leading_width, label_end, items }
    }

    /// The x of the offset `offset` from the row's reading start.
    fn edge(&self, rect: Rect, offset: f32) -> f32 {
        if self.rtl { rect.right() - offset } else { rect.left() + offset }
    }

    /// The part of the row `width` wide at `offset` from the reading start, the row's height.
    fn span(&self, rect: Rect, offset: f32, width: f32) -> Rect {
        let (a, b) = (self.edge(rect, offset), self.edge(rect, offset + width));
        Rect::from_x_y_ranges(a.min(b)..=a.max(b), rect.y_range())
    }

    fn label_band(&self, rect: Rect) -> Rect {
        self.span(rect, self.label_start, (self.label_end - self.label_start).max(0.0))
    }
}

fn paint_background(ui: &Ui, style: &ListRowStyle, rect: Rect, selected: bool, lit: bool, columns: usize, geometry: &Geometry) {
    let fill = match (selected, lit) {
        (true, true) => style.selected_hover,
        (true, false) => style.selected,
        (false, true) => style.hover,
        (false, false) => Color32::TRANSPARENT,
    };
    let painter = ui.painter();
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, corner_radius(style.radius), fill);
    }
    if style.dividers {
        let ppp = ui.pixels_per_point();
        painter.hline(rect.x_range(), (rect.bottom() - style.divider.width / 2.0).round_to_pixel_center(ppp), style.divider);
        for i in 1..=columns {
            painter.vline(geometry.edge(rect, i as f32 * style.toggle_column).round_to_pixel_center(ppp), rect.y_range(), style.divider);
        }
    }
}

/// The thumbnail's frame, with the host's painting inside it.
fn paint_thumb(ui: &Ui, style: &ListRowStyle, at: Rect, thumbnail: Option<Thumbnail<'_>>) {
    let inner = at.shrink(1.0);
    if let Some(paint) = thumbnail {
        paint(&ui.painter().with_clip_rect(inner), inner);
    }
    ui.painter().rect_stroke(at, 0.0, Stroke::new(1.0, style.thumb_border), StrokeKind::Inside);
}

/// The label, and the detail line under it, centred in `band` and cut with an ellipsis.
fn paint_label(ui: &mut Ui, tokens: &UiTokens, style: &ListRowStyle, band: Rect, label: &str, detail: Option<&str>, color: Color32) {
    if band.width() < 1.0 {
        return;
    }
    let total = style.label_line + detail.map_or(0.0, |_| style.detail_line);
    let top = band.center().y - total / 2.0;
    let line = |ui: &mut Ui, top: f32, height: f32, text: &str, role: TypeRole, color: Color32| {
        let at = Rect::from_x_y_ranges(band.x_range(), top..=top + height);
        let format = role_format(tokens, role, color);
        ui.scope_builder(UiBuilder::new().max_rect(at).layout(direction::row(ui)), |ui| {
            ui.add(Label::new(egui::text::LayoutJob::single_section(text.to_owned(), format)).truncate().selectable(false));
        });
    };
    line(ui, top, style.label_line, label, style.label_role, color);
    if let Some(detail) = detail {
        line(ui, top + style.label_line, style.detail_line, detail, style.detail_role, style.detail);
    }
}

fn paint_drop_mark(ui: &Ui, style: &ListRowStyle, rect: Rect, mark: DropMark) {
    let width = style.drop_width;
    match mark {
        DropMark::Before => {
            ui.painter().rect_filled(Rect::from_min_size(rect.min, vec2(rect.width(), width)), 0.0, style.drop);
        }
        DropMark::After => {
            ui.painter().rect_filled(Rect::from_min_size(pos2(rect.left(), rect.bottom() - width), vec2(rect.width(), width)), 0.0, style.drop);
        }
        DropMark::Into => {
            let radius: CornerRadius = corner_radius(style.radius);
            ui.painter().rect_stroke(rect, radius, Stroke::new(width, style.drop), StrokeKind::Inside);
        }
    }
}

const THUMB_NAMES: &[&str] = &["None", "Small", "Medium", "Large"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "composites/list-row/layers",
        level: Level::Composites,
        component: "ListRow",
        name: "Layers",
        args: || {
            Args::new()
                .with_choice("thumbnails", THUMB_NAMES, 0)
                .with_bool("detail", false)
                .with_bool("highlight second row", false)
                .with_number("width", 280.0, 60.0, 480.0)
        },
        show: show_layers,
    }]
}

/// The state of the story's layers panel.
#[derive(Clone, Debug)]
struct Layers {
    rows: Vec<Layer>,
    selected: Option<usize>,
    renaming: Option<usize>,
}

#[derive(Clone, Debug)]
struct Layer {
    name: String,
    visible: bool,
    locked: bool,
}

fn layer(name: &str, locked: bool) -> Layer {
    Layer { name: name.to_owned(), visible: true, locked }
}

impl Default for Layers {
    fn default() -> Self {
        let rows = vec![
            layer("Headline", false),
            layer("Photo with a name too long for the panel to show whole", false),
            layer("Shadow", false),
            layer("Background", true),
        ];
        Self { rows, selected: Some(0), renaming: None }
    }
}

/// A layers panel `width` wide: eye and lock columns, a thumbnail of the chosen size, a detail
/// line, an optional trailing blend label, a hidden (dimmed) layer once its eye is off, and
/// renaming by double click on the label. Drag down a column to sweep it.
pub(super) fn show_layers(ui: &mut Ui, args: &mut Args) {
    let width = args.number("width") as f32;
    let thumb = ThumbSize::ALL.into_iter().find(|t| t.name() == args.choice("thumbnails")).unwrap_or_default();
    let (detail, highlight) = (args.bool("detail"), args.bool("highlight second row"));
    let state_id = ui.scope_id().with("list-row-story");
    let mut layers = ui.data(|d| d.get_temp::<Layers>(state_id)).unwrap_or_default();
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = 0.0;
        for i in 0..layers.rows.len() {
            let row = &layers.rows[i];
            let tokens = Tokens::of(ui);
            let fill = tokens.color.surface.raised.to_egui();
            let mut item = ListRow::new(i, row.name.clone())
                .toggle(ToggleColumn::eye("Visibility", row.visible))
                .toggle(ToggleColumn::lock("Lock", row.locked))
                .thumb(thumb, move |painter, rect| {
                    painter.rect_filled(rect, 0.0, fill);
                })
                .optional_item(56.0, |ui| {
                    ui.label("Normal");
                })
                .selected(layers.selected == Some(i))
                .dimmed(!row.visible)
                .highlight(highlight && i == 1)
                .renaming(layers.renaming == Some(i))
                .position_in_set(i);
            if detail {
                item = item.detail("Raster layer");
            }
            let out = item.show(ui);
            if out.response.clicked() {
                layers.selected = Some(i);
            }
            let on_label = out.response.interact_pointer_pos().is_some_and(|p| out.label.contains(p));
            if out.response.double_clicked() && on_label {
                layers.renaming = Some(i);
            }
            for event in out.events {
                match event {
                    RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. }) => match value.column {
                        0 => layers.rows[value.row].visible = value.on,
                        _ => layers.rows[value.row].locked = value.on,
                    },
                    RowEvent::Renamed { row, text } if !text.is_empty() => layers.rows[row].name = text,
                    RowEvent::RenameEnded { .. } => layers.renaming = None,
                    _ => {}
                }
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, layers));
}
