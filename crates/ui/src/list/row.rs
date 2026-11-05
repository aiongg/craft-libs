//! [`ListRow`]: one row of a list.

use std::borrow::Cow;
use std::sync::Arc;

use craft_theme::{Stroke as Hairline, ThumbSizes, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{Color32, Id, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use super::rename;
use super::toggle::{self, Cell, ToggleColumn, ToggleEdit};
use super::tree::{self, Branch, OpenChange, TreeText};
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::{self, Icon, IconSize};
use crate::localized::SharedLabels as _;
use crate::localized::{Labels, Word, Words};
use crate::menu::{ContextMenu, MenuEntry, OpenAt};
use crate::primitives::{FocusRing, Interactive};
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, role_format};
use crate::typed_story;

/// The thumbnail setting of a list (PhotoCraft's Layers Panel Options), which the host stores
/// (it is serde, for a panel's `*Settings`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// A side in points, for a thumbnail that follows a slider (a preset browser's list view).
    Px(u16),
}

impl ThumbSize {
    /// Every setting.
    pub const ALL: [ThumbSize; 4] = [ThumbSize::None, ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large];

    /// The setting's English name, for settings and stories.
    pub const fn name(self) -> &'static str {
        self.word().english
    }

    /// The setting's name in the language of `words`.
    pub fn label(self, words: &Words) -> Cow<'static, str> {
        words.get(self.word())
    }

    const fn word(self) -> Word {
        let index = match self {
            ThumbSize::None => 0,
            ThumbSize::Small => 1,
            ThumbSize::Medium => 2,
            ThumbSize::Large => 3,
            ThumbSize::Px(_) => 4,
        };
        <ThumbSizeLabels as Labels>::WORDS[index]
    }
}

crate::words! {
    /// The names of the [`ThumbSize`] settings.
    pub struct ThumbSizeLabels {
        none: "list.thumbNone" = "None";
        small: "list.thumbSmall" = "Small";
        medium: "list.thumbMedium" = "Medium";
        large: "list.thumbLarge" = "Large";
        custom: "list.thumbCustom" = "Custom";
    }
}

crate::words! {
    /// The name of a row's "\u{2026}" menu button; `{row}` is the row's label.
    pub struct RowMenuLabels {
        more: "list.rowMenu" = "More ({row})";
    }
}

/// What a [`List::row_menu`](super::List::row_menu) entry runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RowCommand {
    /// Rename the row: the list opens the row's rename field in the pass the entry is chosen, so
    /// the host sees no event for it.
    Rename,
    /// An entry of the host's, numbered by the host.
    Other(usize),
}

/// The "\u{2026}" menu of a row, set by the list.
struct RowMenu {
    entries: Vec<MenuEntry<RowCommand>>,
    /// Whether [`RowCommand::Rename`] opens the row's rename field.
    can_rename: bool,
}

crate::words! {
    /// The description of a clipped row for a screen reader.
    pub(crate) struct ClippedLabels {
        clipped: "list.clipped" = "Clipped to the layer below";
    }
}

impl ClippedLabels {
    fn text(ui: &Ui) -> String {
        crate::localized::Words::of(ui).get(<Self as crate::localized::Labels>::WORDS[0]).into_owned()
    }
}

/// The description of a clipped row: "Clipped to the layer below", then the host's own.
fn clipped_description(ui: &Ui, description: Option<String>) -> String {
    let clipped = ClippedLabels::text(ui);
    description.map_or(clipped.clone(), |description| format!("{clipped}, {description}"))
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
    /// An entry of the row's menu ([`List::row_menu`](super::List::row_menu)) other than a
    /// rename was chosen.
    Menu {
        /// The row.
        row: K,
        /// The entry's command.
        command: RowCommand,
    },
    /// The row's menu chose [`RowCommand::Rename`] on a row that can be renamed: the row has
    /// opened its rename field in this pass. A [`List`](super::List) keeps the open rename.
    RenameStarted {
        /// The row.
        row: K,
    },
    /// The rename field closed, by commit or by Esc; the host stops passing `renaming(true)`.
    RenameEnded {
        /// The row.
        row: K,
        /// How it ended.
        end: RenameEnd,
    },
    /// A tree row's disclosure triangle was clicked, or a screen reader expanded or collapsed
    /// the row.
    Open(OpenChange<K>),
    /// The link glyph before an extra thumbnail was clicked ([`thumb_link`](ListRow::thumb_link)):
    /// `linked` is the state it asks for, the opposite of what the row showed.
    LinkToggled {
        /// The row.
        row: K,
        /// The extra thumbnail the glyph belongs to, counted from 0 in the order added.
        thumb: usize,
        /// The requested state.
        linked: bool,
    },
    /// A click landed on a thumbnail. The row's own response is clicked as well, so the host
    /// selects the row as for any click and also learns which thumbnail it hit (a layer's pixels
    /// or its mask become the target).
    ThumbClicked {
        /// The row.
        row: K,
        /// The thumbnail.
        which: ThumbTarget,
    },
    /// A double click landed on the label area of a row the [`List`](super::List) can't rename
    /// (one it renames starts the rename instead). Reported before the
    /// [`Activate`](super::ListEvent::Activate) the same click makes, so a host can tell a double
    /// click on the name from one elsewhere on the row (Layers' `ActivateOn::Name`).
    LabelDoubleClicked {
        /// The row.
        row: K,
    },
    /// A drag started on an extra thumbnail of a row with [`thumb_drags`](ListRow::thumb_drags):
    /// the thumbnail is dragged, not the row. A [`List`](super::List) starts no row drag for this
    /// press; the host sets its own egui drag payload in the same pass (a layer's mask, which only
    /// the footer's Delete takes).
    ThumbDrag {
        /// The row.
        row: K,
        /// The thumbnail the press started on: always an [`Extra`](ThumbTarget::Extra) one.
        which: ThumbTarget,
    },
}

/// One of a row's thumbnails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThumbTarget {
    /// The thumbnail ([`thumb`](ListRow::thumb)).
    First,
    /// An extra thumbnail ([`extra_thumb`](ListRow::extra_thumb)), counted from 0 in the order
    /// added.
    Extra(usize),
}

/// A trailing item of a row.
pub struct TrailingItem<'a> {
    width: f32,
    optional: bool,
    add: Box<dyn FnOnce(&mut Ui) + 'a>,
}

type Thumbnail<'a> = Box<dyn FnOnce(&Painter, Rect) + 'a>;

/// A thumbnail after the first (a mask), with the link glyph before it if it has one.
struct ExtraThumb<'a> {
    paint: Thumbnail<'a>,
    link: Option<(bool, String)>,
}

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
    /// The response of each toggle cell, in [`ListRow::toggle`] order: a host reads a secondary
    /// click (PhotoCraft's menu on the eye), the hover and the focus from it. Primary clicks and
    /// drags are the cell's [`RowEvent::Toggle`] edits.
    pub toggles: Vec<Response>,
    /// The least width the row needs: its required parts (toggle columns, disclosure, thumbnails
    /// and links, required trailing items) and `list.min_label` for the label. The same as
    /// [`ListRow::min_width`]. A narrower row is clipped at its trailing edge. A host sets its
    /// panel's minimum width from this.
    pub min_width: f32,
}

/// A row's sizes and colours, from [`ListRow::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListRowStyle {
    /// Height of a row without a thumbnail: `list.row_height`.
    pub row_height: f32,
    /// Height of a row in a dense list (a layer's sub-row, a compact panel): `list.row_height_dense`.
    pub row_height_dense: f32,
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
    /// Width of a tree row's disclosure column: `list.disclosure`.
    pub disclosure: f32,
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
    /// How far the second-thumb brackets stand outside the thumbnail: `list.bracket_outset`.
    pub bracket_outset: f32,
    /// Arm length of the second-thumb brackets: `list.bracket_arm`.
    pub bracket_arm: f32,
    /// Side of an icon: `size.icon.md`.
    pub icon_size: f32,
    /// Type role of the label: `list.label`.
    pub label_role: TypeRole,
    /// Type role of the label of a [strong](ListRow::strong) row: `list.label_strong`.
    pub label_strong_role: TypeRole,
    /// Type role of the detail line: `list.detail_role`.
    pub detail_role: TypeRole,
    /// Line height of the label and of the detail line.
    pub label_line: f32,
    pub detail_line: f32,
    /// `list.text`, `list.text.dimmed`, `list.text.selected` and `list.text.selected.dimmed`.
    pub text: Color32,
    pub dimmed: Color32,
    /// `list.text.muted`: the label of a [muted](ListRow::muted) row.
    pub muted: Color32,
    pub selected_text: Color32,
    pub selected_dimmed: Color32,
    /// `list.detail` and `list.detail.selected`.
    pub detail: Color32,
    pub selected_detail: Color32,
    /// `list.icon` and `list.icon.hover`.
    pub icon: Color32,
    pub icon_hover: Color32,
    /// `list.hover`, `list.selected` and `list.selected.hover`.
    pub hover: Color32,
    pub selected: Color32,
    pub selected_hover: Color32,
    /// `list.stripe`: behind every second row of a striped list, under the hover and selected fills.
    pub stripe: Color32,
    /// The hairlines: `list.divider`, one physical pixel.
    pub divider: Stroke,
    /// `list.thumb.border` and `list.thumb.border.selected`.
    pub thumb_border: Color32,
    pub thumb_border_selected: Color32,
    /// `list.thumb.radius`: the corners of the thumbnail's frame.
    pub thumb_radius: f32,
    /// The layer form's fills: `list.layer.hover` and `list.layer.selected`, and the 1 pt inside
    /// outline of the selected row, `list.layer.outline` (clear when the theme has none).
    pub layer_hover: Color32,
    pub layer_selected: Color32,
    pub layer_outline: Color32,
    /// `list.layer.radius`: the corners of the layer form's fills and outline.
    pub layer_radius: f32,
    /// `list.layer.text.selected`, `list.layer.text.selected.dimmed` and
    /// `list.layer.detail.selected`.
    pub layer_selected_text: Color32,
    pub layer_selected_dimmed: Color32,
    pub layer_selected_detail: Color32,
    /// `list.drop`.
    pub drop: Color32,
    /// The outline under the drop marker: `list.drop_halo_width` of `list.drop_halo` on each side.
    pub drop_halo: Stroke,
    /// The drag ghost: `list.ghost.fill`, a 1 pt `list.ghost.border`, `radius.control` corners.
    pub ghost_fill: Color32,
    pub ghost_border: Color32,
    pub ghost_radius: f32,
    /// Where the ghost starts from the pointer toward the reading end: `list.ghost_offset`.
    pub ghost_offset: Vec2,
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
            ThumbSize::Px(side) => f32::from(side),
        }
    }
}

/// One row of a list.
///
/// ```
/// use craft_ui::list::{ListRow, ToggleColumn};
/// # egui::__run_test_ui(|ui| {
/// let row = ListRow::new("layer-1", "Layer 1")
///     .toggle(ToggleColumn::eye("Visibility {row}", true))
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
/// - **Form.** A plain row (history, actions, swatches) has no dividers and the plain hover. The
///   [layer form](ListRow::layer) (layers, channels) adds the dividers, the accent tint on the
///   hover and the layer look of the selected row.
/// - **States** are inputs: [`selected`](ListRow::selected) (the `list.selected` fill,
///   rounded in Comfortable, no border), [`focused`](ListRow::focused) (the focus ring, drawn
///   while the keyboard is the last input), [`dimmed`](ListRow::dimmed) (a hidden layer's faint
///   label), [`highlight`](ListRow::highlight) (the hover fill without the pointer),
///   [`drop_mark`](ListRow::drop_mark) and [`renaming`](ListRow::renaming).
/// - **Label.** One line, cut with an ellipsis. The row shows a tooltip only when
///   [`tooltip`](ListRow::tooltip) sets one. Trailing
///   items are placed first, from the trailing edge inward; those marked
///   [optional](ListRow::optional_item) drop out, in the order given, before the label would
///   get narrower than `list.min_label`, and the indent shrinks before the label gets narrower
///   than that. The label is never narrower than `list.min_label`: a row too narrow for its
///   required parts and that label is clipped at the trailing edge, and
///   [`min_width`](ListRow::min_width) is the width it needs.
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
    description: Option<String>,
    tooltip: Option<egui::WidgetText>,
    accessible_name: Option<String>,
    columns: Vec<ToggleColumn>,
    icon: Option<Icon>,
    glyph: Option<Thumbnail<'a>>,
    glyph_width: Option<f32>,
    striped: bool,
    thumb: ThumbSize,
    thumbnail: Option<Thumbnail<'a>>,
    thumb_width: Option<f32>,
    extras: Vec<ExtraThumb<'a>>,
    target: Option<ThumbTarget>,
    thumb_drags: bool,
    trailing: Vec<TrailingItem<'a>>,
    selected: bool,
    focused: bool,
    dimmed: bool,
    muted: bool,
    strong: bool,
    selectable: bool,
    spacer: bool,
    highlight: bool,
    layer: bool,
    clipped: bool,
    italic: bool,
    renaming: bool,
    rule: bool,
    drop: Option<DropMark>,
    indent: f32,
    paint_depth: Option<usize>,
    indented_toggles: bool,
    rename_field_name: Option<String>,
    menu: Option<RowMenu>,
    role: Role,
    position: Option<usize>,
    slot: Option<Rect>,
    branch: Option<Branch>,
    tree_text: Option<Arc<TreeText>>,
}

fn with_tooltip(response: Response, tooltip: Option<egui::WidgetText>) -> Response {
    match tooltip {
        Some(tooltip) => response.on_hover_text(tooltip),
        None => response,
    }
}

/// The id of the row for `key` among rows shown in a `Ui` whose scope is `scope`: the base of the
/// row's state (the open rename, the toggle cells).
pub(super) fn row_id<K: std::hash::Hash + std::fmt::Debug>(scope: Id, key: &K) -> Id {
    scope.with(("list-row", key))
}

impl<'a, K: std::hash::Hash + std::fmt::Debug + Clone + Send + Sync + 'static> ListRow<'a, K> {
    /// A row for `key`, which keeps its state (the open rename) across frames and names it in
    /// events, showing `label`.
    pub fn new(key: K, label: impl Into<String>) -> Self {
        Self {
            key,
            label: label.into(),
            detail: None,
            description: None,
            tooltip: None,
            accessible_name: None,
            columns: Vec::new(),
            icon: None,
            glyph: None,
            glyph_width: None,
            striped: false,
            thumb: ThumbSize::None,
            thumbnail: None,
            thumb_width: None,
            extras: Vec::new(),
            target: None,
            thumb_drags: false,
            trailing: Vec::new(),
            selected: false,
            focused: false,
            dimmed: false,
            muted: false,
            strong: false,
            selectable: true,
            spacer: false,
            highlight: false,
            layer: false,
            clipped: false,
            italic: false,
            renaming: false,
            rule: false,
            drop: None,
            indent: 0.0,
            paint_depth: None,
            indented_toggles: false,
            rename_field_name: None,
            menu: None,
            role: Role::ListBoxOption,
            position: None,
            slot: None,
            branch: None,
            tree_text: None,
        }
    }

    /// Empty room in a list: a row that takes its height but draws nothing, answers no pointer
    /// and has no AccessKit node. Give its position [`DisplayOrder::is_section`] so that keys and
    /// clicks skip it, and its height through [`List::heights`].
    pub fn spacer(key: K) -> Self {
        let mut row = Self::new(key, String::new());
        row.spacer = true;
        row
    }

    /// Whether the row is a [spacer](ListRow::spacer).
    pub fn is_spacer(&self) -> bool {
        self.spacer
    }

    /// Whether keys and clicks select the row (default true). A row that is not selectable is
    /// still a stop for the lead: Up, Down, Home, End, Page Up and Page Down move the lead onto
    /// it without changing the selection, a click makes it the lead without selecting it, and
    /// Space on it selects nothing. Enter and a double click still activate it.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Whether the row is [selectable](ListRow::selectable).
    pub fn is_selectable(&self) -> bool {
        self.selectable
    }

    /// The label in the strong weight (`list.label_strong`), for a row that heads the others.
    pub fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    /// The label in `text.muted`: a secondary row that is still on. A [dimmed](ListRow::dimmed)
    /// row (`text.faint`) wins over it; a selected muted row takes the selected dimmed color.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// The row is clipped to the layer below: it is indented one step and shows the down-right
    /// clipping arrow before its thumbnail or icon. AccessKit describes it as clipped (the
    /// [`description`](ListRow::description), if given, follows).
    pub fn clipped(mut self, clipped: bool) -> Self {
        self.clipped = clipped;
        self
    }

    /// The label is drawn in italics (Photoshop's Background layer).
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Makes the row a tree row: indented by its depth, with a disclosure triangle when it is a
    /// group, and a `TreeItem` for AccessKit.
    pub(super) fn tree_text(mut self, text: Arc<TreeText>) -> Self {
        self.tree_text = Some(text);
        self
    }

    pub(super) fn branch(mut self, branch: Branch) -> Self {
        self.branch = Some(branch);
        self.role = Role::TreeItem;
        self
    }

    /// Takes the text typed into the open rename of the row `key` shown in `ui`'s scope, closing
    /// the rename; `None` when it has none. A host that starts another rename by a command
    /// calls this for the one that is open and applies the text as [`RowEvent::Renamed`] would
    /// (a [`List`](super::List) does this itself). The row then isn't shown with
    /// [`renaming`](ListRow::renaming) any more.
    pub fn take_rename_draft(ui: &Ui, key: &K) -> Option<String> {
        rename::take_draft(ui.ctx(), row_id(ui.scope_id(), key))
    }

    /// Takes the open rename of whichever row with this key type is renaming, closing it, and
    /// returns that row's key with the text typed; `None` when no rename is open. Only one rename
    /// is open at a time, since the field holds the keyboard focus. For a host command that runs
    /// with the [`Context`](egui::Context) only (a menu item): apply the text as
    /// [`RowEvent::Renamed`] would.
    pub fn take_open_rename_draft(ctx: &egui::Context) -> Option<(K, String)> {
        rename::take_open_draft(ctx)
    }

    /// The width the row needs in `ui`'s theme: its required parts and `list.min_label` for the
    /// label, whatever the indent (which shrinks). Equals [`RowResponse::min_width`] of the shown
    /// row; measures without showing.
    pub fn min_width(&self, ui: &Ui) -> f32 {
        self.min_width_with(ui, self.branch.is_some_and(|b| b.group))
    }

    /// [`min_width`](ListRow::min_width) of the row as a row of a tree [`List`](super::List) shows
    /// it, with the disclosure column when `has_children`. The indent shrinks before the label,
    /// so the depth does not change the width.
    pub fn min_width_in_tree(&self, ui: &Ui, has_children: bool) -> f32 {
        self.min_width_with(ui, has_children)
    }

    fn min_width_with(&self, ui: &Ui, group: bool) -> f32 {
        let style = Self::style(&Tokens::of(ui));
        let (column, leading) = self.extent(&style, group);
        let widths: Vec<(f32, bool)> = self.trailing.iter().map(|t| (t.width, t.optional)).collect();
        let ends = self.columns.iter().filter(|c| c.at_end).count();
        min_width(&style, (self.columns.len() - ends, ends, self.indented_toggles), column, leading, &widths)
    }

    /// The width of a tree row's disclosure and clipping columns, and of the leading part.
    fn extent(&self, style: &ListRowStyle, group: bool) -> (f32, Option<f32>) {
        let disclosure = if group { style.disclosure } else { 0.0 };
        let marker = if self.clipped { style.icon_size } else { 0.0 };
        (disclosure + marker, leading_width(style, self.thumb, self.icon_slot(style), self.thumb_width, &self.extras))
    }

    /// The width of the icon or glyph slot, `None` for a row with neither.
    fn icon_slot(&self, style: &ListRowStyle) -> Option<f32> {
        (self.icon.is_some() || self.glyph.is_some()).then(|| self.glyph_width.unwrap_or(style.icon_size))
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

    /// The row's AccessKit name when it differs from the drawn text (a row whose icon carries
    /// part of the meaning). Without it the name is the label, then the detail.
    pub fn accessible_name(mut self, name: impl Into<String>) -> Self {
        self.accessible_name = Some(name.into()).filter(|n| !n.is_empty());
        self
    }

    /// What a screen reader adds after the row's name; not drawn. Empty is none.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into()).filter(|d| !d.is_empty());
        self
    }

    /// A tooltip shown while the pointer rests on the row, whether or not the label is cut
    /// (PhotoCraft's file list shows the full path).
    pub fn tooltip(mut self, tooltip: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// A second line under the label in `caption` and `text.faint` (PhotoCraft Studio's kind
    /// line).
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into()).filter(|d| !d.is_empty());
        self
    }

    /// Adds a toggle column at the reading start, after those added before it; one
    /// [`at_end`](ToggleColumn::at_end) sits at the trailing end instead.
    pub fn toggle(mut self, column: ToggleColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// An icon before the label. A thumbnail takes its place.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.glyph = None;
        self.glyph_width = None;
        self
    }

    /// A glyph painted by `paint` before the label, in the icon's slot: the same place and size as
    /// [`icon`](ListRow::icon) (`size.icon.md` square, centred on the row), with no frame, for a
    /// status mark the host draws itself. `paint` draws into the painter, clipped to that rect. The
    /// row's width and AccessKit tree are those of a row with an icon. It replaces an icon, and an
    /// icon replaces it (the last one set wins). A thumbnail takes its place.
    pub fn glyph(mut self, paint: impl FnOnce(&Painter, Rect) + 'a) -> Self {
        self.glyph = Some(Box::new(paint));
        self.glyph_width = None;
        self.icon = None;
        self
    }

    /// Like [`glyph`](ListRow::glyph) in a slot `width` points wide and `size.icon.md` high, for a
    /// mark that is wider than an icon (a row number). The slot's width counts in the row's least
    /// width and the label starts after it; `paint` draws into that rect, clipped to it.
    pub fn glyph_sized(mut self, width: f32, paint: impl FnOnce(&Painter, Rect) + 'a) -> Self {
        self.glyph = Some(Box::new(paint));
        self.glyph_width = Some(width);
        self.icon = None;
        self
    }

    /// Fills the row with `list.stripe` under the hover and selected fills, for every second row
    /// of a striped list (the host counts the rows).
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// A thumbnail of `size` before the label, framed just outside in `list.thumb.border`
    /// (`list.thumb.border.selected` on the selected row); `paint` draws into the painter, clipped
    /// to the thumbnail's rect. [`ThumbSize::None`] draws none.
    pub fn thumb(mut self, size: ThumbSize, paint: impl FnOnce(&Painter, Rect) + 'a) -> Self {
        self.thumb = size;
        self.thumbnail = Some(Box::new(paint));
        self
    }

    /// A thumbnail `size` points wide and high, for a gradient or other strip that is not
    /// square: as [`thumb`](ListRow::thumb) with a [`ThumbSize::Px`] of the height, and the
    /// width given. The row's height follows the height as for any thumbnail.
    pub fn thumb_sized(mut self, size: Vec2, paint: impl FnOnce(&Painter, Rect) + 'a) -> Self {
        self.thumb_width = Some(size.x.max(0.0));
        self.thumb(ThumbSize::Px(size.y.round().clamp(0.0, f32::from(u16::MAX)) as u16), paint)
    }

    /// A thumbnail after the first and any earlier extra ones, of the same size and frame (a
    /// layer's mask, then its vector mask; the selected ring is the first thumbnail's); `paint`
    /// draws into the painter, clipped to its rect. Nothing is drawn without a first thumbnail.
    /// Extra thumbnails never drop out for width: the label is cut first. A link glyph can sit
    /// before each ([`thumb_link`](ListRow::thumb_link)).
    pub fn extra_thumb(mut self, paint: impl FnOnce(&Painter, Rect) + 'a) -> Self {
        self.extras.push(ExtraThumb { paint: Box::new(paint), link: None });
        self
    }

    /// A link glyph before the extra thumbnail added last ([`Icon::Link`] when `linked`, else
    /// [`Icon::Unlink`], dimmed), which reports [`RowEvent::LinkToggled`] when clicked. `name` is
    /// its accessible name and tooltip, with `{row}` standing for the row's label ("Unlink mask
    /// from {row}"); give each glyph of a row its own. It takes a `list.icon` wide slot; without
    /// it the thumbnails are a `list.gap` apart. The glyph is a Tab stop and shows the focus ring.
    /// Does nothing before an [`extra_thumb`](ListRow::extra_thumb).
    pub fn thumb_link(mut self, linked: bool, name: impl Into<String>) -> Self {
        if let Some(extra) = self.extras.last_mut() {
            extra.link = Some((linked, name.into()));
        }
        self
    }

    /// The thumbnail the row's editing is aimed at (a layer's pixels or a mask). It is framed with
    /// corner brackets whether or not the row is selected: the host passes it for the active row
    /// only. Without a call no thumbnail is marked. A click on a thumbnail reports [`RowEvent::ThumbClicked`].
    pub fn thumb_target(mut self, target: ThumbTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// A press that starts a drag on an [extra thumbnail](ListRow::extra_thumb) drags the
    /// thumbnail instead of the row (default false): the row reports [`RowEvent::ThumbDrag`] in the
    /// pass the drag starts, and a [`List`](super::List) starts no row drag for it. A drag from the
    /// first thumbnail, the label or anywhere else drags the row as before. Clicks on any
    /// thumbnail still report [`RowEvent::ThumbClicked`]. PhotoCraft's mask thumbnails: a drag
    /// from one carries the mask to the footer's Delete.
    pub fn thumb_drags(mut self, thumb_drags: bool) -> Self {
        self.thumb_drags = thumb_drags;
        self
    }

    /// The row's "\u{2026}" menu, drawn by the row at its trailing edge before the label, so a
    /// [`Rename`](RowCommand::Rename) starts the rename field in the pass it is chosen. A list sets
    /// it from [`List::row_menu`](super::List::row_menu); no menu for empty `entries`.
    pub(super) fn menu(mut self, ui: &Ui, entries: Vec<MenuEntry<RowCommand>>, can_rename: bool) -> Self {
        if entries.is_empty() {
            return self;
        }
        let width = Self::style(&Tokens::of(ui)).toggle_column;
        // The first item is the outermost.
        self.trailing.insert(0, TrailingItem { width, optional: false, add: Box::new(|_| {}) });
        self.menu = Some(RowMenu { entries, can_rename });
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

    /// The layer form of the row, for lists of layers and channels: dividers under the row and at
    /// each toggle column's edge (where `list.dividers` is on; plain rows have none), the hover
    /// fill with the accent tint, and the selected row in the layer look (`list.layer.*`: in themes
    /// whose PhotoCraft kind is not a Pro kind, the hover fill in a 1 pt outline).
    pub fn layer(mut self, layer: bool) -> Self {
        self.layer = layer;
        self
    }

    /// The rename field replaces the label.
    pub fn renaming(mut self, renaming: bool) -> Self {
        self.renaming = renaming;
        self
    }

    /// A `list.divider` hairline along the row's bottom edge, across its whole width, in a
    /// [plain](ListRow::layer) row too (default false): the History panel's rule under its last
    /// snapshot. A layer-form row already draws it where `list.dividers` is on, and draws it once.
    pub fn rule(mut self, rule: bool) -> Self {
        self.rule = rule;
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

    /// The tree depth the indent is painted at, when it differs from the depth the list has the
    /// row at: a filtered view lists a row under the ancestors shown (its depth for the keys and
    /// AccessKit) and keeps the indent of its true depth. Only a row of a tree list indents by
    /// depth; the depth counts as [`TreeRow`](super::TreeRow)'s does.
    pub fn paint_depth(mut self, depth: usize) -> Self {
        self.paint_depth = Some(depth);
        self
    }

    /// The accessible name of the rename field (default: the row's label): the field's purpose,
    /// as in History's "Name".
    pub fn rename_field_name(mut self, name: impl Into<String>) -> Self {
        self.rename_field_name = Some(name.into());
        self
    }

    /// Puts the [toggle columns](ListRow::toggle) after the [indent](ListRow::indent) instead of at
    /// the leading edge: a sub-row's eye sits under its parent's. The leading edge keeps its
    /// columns' width, empty, so the dividers and the parent's eye column run on through the row.
    /// A [sweep](ToggleEdit) over the column takes the row wherever the pointer crosses it between
    /// the leading edge and its cell, so one sweep runs down normal and indented rows alike.
    pub fn indented_toggles(mut self, indented: bool) -> Self {
        self.indented_toggles = indented;
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
            row_height_dense: l.row_height_dense,
            thumb: l.thumb,
            thumb_pad: l.thumb_pad,
            thumb_row_min: l.thumb_row_min,
            pad_x: l.pad_x,
            gap: l.gap,
            indent: l.indent,
            disclosure: l.disclosure,
            toggle_column: l.toggle_column,
            min_label: l.min_label,
            trailing_pad: l.trailing_pad,
            radius: l.radius,
            dividers: l.dividers,
            drop_width: l.drop_width,
            bracket_outset: l.bracket_outset,
            bracket_arm: l.bracket_arm,
            icon_size: tokens.size.icon.md,
            label_role: l.label,
            label_strong_role: l.label_strong,
            detail_role: l.detail_role,
            label_line: tokens.typography.get(l.label).line_height,
            detail_line: tokens.typography.get(l.detail_role).line_height,
            text: l.text.to_egui(),
            dimmed: l.dimmed.to_egui(),
            muted: l.muted.to_egui(),
            selected_text: l.selected_text.to_egui(),
            selected_dimmed: l.selected_dimmed.to_egui(),
            detail: l.detail.to_egui(),
            selected_detail: l.selected_detail.to_egui(),
            icon: l.icon.to_egui(),
            icon_hover: l.icon_hover.to_egui(),
            hover: l.hover.to_egui(),
            selected: l.selected.to_egui(),
            selected_hover: l.selected_hover.to_egui(),
            stripe: l.stripe.to_egui(),
            divider: Stroke::new(Hairline::Hairline.width(1.0), l.divider.to_egui()),
            thumb_border: l.thumb_border.to_egui(),
            thumb_border_selected: l.thumb_border_selected.to_egui(),
            thumb_radius: l.thumb_radius,
            layer_hover: l.layer_hover.to_egui(),
            layer_selected: l.layer_selected.to_egui(),
            layer_outline: l.layer_outline.to_egui(),
            layer_radius: l.layer_radius,
            layer_selected_text: l.layer_selected_text.to_egui(),
            layer_selected_dimmed: l.layer_selected_dimmed.to_egui(),
            layer_selected_detail: l.layer_selected_detail.to_egui(),
            drop: l.drop.to_egui(),
            drop_halo: Stroke::new(l.drop_halo_width, l.drop_halo.to_egui()),
            ghost_fill: l.ghost_fill.to_egui(),
            ghost_border: l.ghost_border.to_egui(),
            ghost_radius: tokens.shape.radius.control,
            ghost_offset: Vec2::from(l.ghost_offset),
        }
    }

    /// Shows the row across the available width.
    pub fn show(self, ui: &mut Ui) -> RowResponse<K> {
        let ListRow {
            key,
            label,
            detail,
            description,
            tooltip,
            accessible_name,
            columns,
            icon,
            glyph,
            glyph_width,
            striped,
            thumb,
            thumbnail,
            thumb_width,
            extras,
            target,
            thumb_drags,
            trailing,
            selected,
            focused,
            dimmed,
            muted,
            strong,
            selectable: _,
            spacer: _,
            highlight,
            layer,
            clipped,
            italic,
            renaming,
            rule,
            drop,
            indent,
            paint_depth,
            indented_toggles,
            rename_field_name,
            menu,
            role,
            position,
            slot,
            branch,
            tree_text,
        } = self;
        let tokens = Tokens::of(ui);
        let mut style = Self::style(&tokens);
        style.divider.width = Hairline::Hairline.width(ui.pixels_per_point());
        let height = style.height(thumb, detail.is_some());
        let indent = indent + if clipped { style.indent } else { 0.0 };
        let (indent, disclosure) = branch.map_or((indent, 0.0), |b| {
            let steps = Branch { depth: paint_depth.unwrap_or(b.depth), ..b }.steps();
            (indent + steps as f32 * style.indent, if b.group { style.disclosure } else { 0.0 })
        });
        // The clipping arrow takes a column of its own after the disclosure triangle.
        let marker = if clipped { style.icon_size } else { 0.0 };
        let column = disclosure + marker;
        let rect = slot.unwrap_or_else(|| ui.allocate_space(vec2(ui.available_width(), height)).1);
        let row_id = row_id(ui.scope_id(), &key);
        let name = match (accessible_name, &detail) {
            (Some(name), _) => name,
            (None, Some(detail)) => format!("{label}, {detail}"),
            (None, None) => label.clone(),
        };
        let mut element = Interactive::with_id(row_id, role, name.clone())
            .sense(Sense::CLICK | Sense::DRAG)
            .selected(selected)
            .state_layer(false)
            .focus_ring(false);
        if let Some(Branch { group: true, open, .. }) = branch {
            element = element.expanded(open);
        }
        if let Some(position) = position {
            element = element.position_in_set(position);
        }
        let response = element.interact_at(ui, rect).finish(ui);
        let response = with_tooltip(response, tooltip);
        let description = if clipped { Some(clipped_description(ui, description)) } else { description };
        if let Some(description) = description {
            ui.ctx().accesskit_node_builder(response.id, |node| node.set_description(description));
        }
        let mut events = Vec::new();
        if let Some(branch) = branch {
            tree::report(ui, &response, &branch, &key, &mut events);
        }

        let side = style.thumb_side(thumb);
        let first_width = (side > 0.0).then(|| thumb_width.unwrap_or(side));
        let icon_slot = (icon.is_some() || glyph.is_some()).then(|| glyph_width.unwrap_or(style.icon_size));
        let leading = leading_width(&style, thumb, icon_slot, thumb_width, &extras);
        let extras = if side > 0.0 && thumb != ThumbSize::None { extras } else { Vec::new() };
        let widths: Vec<(f32, bool)> = trailing.iter().map(|t| (t.width, t.optional)).collect();
        let ends = columns.iter().filter(|c| c.at_end).count();
        let counts = (columns.len() - ends, ends, indented_toggles);
        let min_width = min_width(&style, counts, column, leading, &widths);
        let geometry = Geometry::new(&style, rect, direction::is_rtl(ui), counts, (indent, column), leading, &widths);
        // Parts that don't fit stay in their place and the row's trailing edge cuts them.
        let outer_clip = ui.clip_rect();
        ui.set_clip_rect(outer_clip.intersect(rect));
        let visible = ui.is_rect_visible(rect);
        let hovered = ui.rect_contains_pointer(rect) && ui.is_enabled();
        if visible {
            paint_background(ui, &style, rect, Fill { selected, lit: hovered || highlight, layer, striped }, counts.0, &geometry);
            paint_rule(ui, &style, rect, (rule, layer));
        }

        let mut toggles = Vec::with_capacity(columns.len());
        for (index, column) in columns.iter().enumerate() {
            let cell = Cell { index, row_id, key: &key, row_label: &label, selected };
            if column.at_end {
                let at = end_cell(&geometry, &style, rect, &columns, index);
                toggles.push(toggle::show(ui, &cell, column, (at, at), &style, &mut events));
                continue;
            }
            let before = columns[..index].iter().filter(|c| !c.at_end).count();
            let offset = geometry.toggles_start + before as f32 * style.toggle_column;
            let at = geometry.span(rect, offset, style.toggle_column);
            // A sweep crosses an indented cell anywhere between the leading edge and the cell.
            let reach = if indented_toggles { geometry.span(rect, 0.0, offset + style.toggle_column) } else { at };
            toggles.push(toggle::show(ui, &cell, column, (at, reach), &style, &mut events));
        }

        if clipped && visible {
            let at = geometry.span(rect, geometry.column_start + disclosure, marker);
            paint_clip_arrow(ui, &style, at, geometry.rtl);
        }

        if let Some(branch) = branch.filter(|b| b.group) {
            let at = geometry.span(rect, geometry.column_start, disclosure);
            tree::show_disclosure(ui, &style, at, &branch, &tree_text.unwrap_or_else(|| TreeText::shared(ui)), row_id, &key, &label, &mut events);
        }

        if let (Some(across), true) = (leading, visible) {
            let high = if side > 0.0 { side } else { style.icon_size };
            let at =
                Rect::from_center_size(pos2(geometry.span(rect, geometry.leading_start, across).center().x, rect.center().y), vec2(across, high));
            let at = at.round_to_pixels(ui.pixels_per_point());
            if side > 0.0 && thumb != ThumbSize::None {
                let mut thumbs = Vec::new();
                if let (false, Some(first_width)) = (extras.is_empty(), first_width) {
                    let ppp = ui.pixels_per_point();
                    let part = |offset: f32, width: f32| {
                        let span = geometry.span(rect, geometry.leading_start + offset, width);
                        Rect::from_center_size(span.center(), vec2(width, high)).round_to_pixels(ppp)
                    };
                    let first = part(0.0, first_width);
                    paint_thumb(ui, &style, first, thumbnail, selected);
                    thumbs.push((ThumbTarget::First, first));
                    let mut offset = first_width;
                    for (index, extra) in extras.into_iter().enumerate() {
                        let gap = between(&style, &extra);
                        let other = part(offset + gap, first_width);
                        paint_thumb(ui, &style, other, Some(extra.paint), false);
                        thumbs.push((ThumbTarget::Extra(index), other));
                        if let Some((linked, name)) = &extra.link {
                            let slot = geometry.span(rect, geometry.leading_start + offset, gap);
                            let slot = Rect::from_center_size(slot.center(), vec2(gap, high));
                            show_link(ui, &style, slot, (row_id, index), &key, &label, (*linked, name), &mut events);
                        }
                        offset += gap + first_width;
                    }
                } else {
                    paint_thumb(ui, &style, at, thumbnail, selected);
                    thumbs.push((ThumbTarget::First, at));
                }
                if let Some((_, r)) = thumbs.iter().find(|(which, _)| Some(*which) == target) {
                    paint_brackets(ui.painter(), *r, bracket_color(&style, selected, layer), &style);
                }
                if let Some(pos) = response.clicked().then(|| response.interact_pointer_pos()).flatten()
                    && let Some((which, _)) = thumbs.iter().find(|(_, r)| r.contains(pos))
                {
                    events.push(RowEvent::ThumbClicked { row: key.clone(), which: *which });
                }
                report_thumb_drag(ui, (thumb_drags, &response), &key, &thumbs, &mut events);
            } else {
                paint_icon_slot(ui, &style, at, glyph, icon);
            }
        }

        let mut renaming = renaming;
        let has_menu = menu.is_some();
        if let (Some(menu), Some(Some((start, width)))) = (menu, geometry.items.first()) {
            let at = geometry.span(rect, *start, *width).round_to_pixels(ui.pixels_per_point());
            match show_menu(ui, &style, at, (&menu, &label), (row_id, hovered, selected)) {
                Some(RowCommand::Rename) if menu.can_rename => {
                    renaming = true;
                    events.push(RowEvent::RenameStarted { row: key.clone() });
                }
                Some(command) => events.push(RowEvent::Menu { row: key.clone(), command }),
                None => {}
            }
        }
        let band = geometry.label_band(rect);
        let (text, detail_text) = label_colors(&style, (selected, dimmed, muted), layer);
        let label_role = if strong { style.label_strong_role } else { style.label_role };
        if renaming {
            rename::show(ui, row_id, &key, &label, rename_field_name.as_deref().unwrap_or(&label), band, &mut events);
        } else if visible {
            paint_label(ui, &tokens, &style, band, &label, detail.as_deref().map(|d| (d, detail_text)), (text, italic, label_role));
        }
        for (item, at) in trailing.into_iter().zip(&geometry.items).skip(usize::from(has_menu)) {
            let Some((start, width)) = *at else { continue };
            let at = geometry.span(rect, start, width).round_to_pixels(ui.pixels_per_point());
            let mut child = ui.new_child(UiBuilder::new().max_rect(at).layout(direction::row(ui)));
            (item.add)(&mut child);
        }
        ui.set_clip_rect(outer_clip);
        if let Some(mark) = drop {
            paint_row_mark(ui, &style, rect, mark);
        }
        FocusRing::paint_if(ui, rect, corner_radius(style.radius), focused);
        RowResponse { response, events, label: band, toggles, min_width }
    }
}

/// The space between a thumbnail and the one after it: the link glyph's slot, or the gap.
fn between(style: &ListRowStyle, extra: &ExtraThumb<'_>) -> f32 {
    if extra.link.is_some() { style.icon_size } else { style.gap }
}

/// The width of a row's leading part (every thumbnail and what is between them, or the icon),
/// without the gap after it; `None` for a row without one.
fn leading_width(style: &ListRowStyle, thumb: ThumbSize, icon_slot: Option<f32>, thumb_width: Option<f32>, extras: &[ExtraThumb<'_>]) -> Option<f32> {
    let side = style.thumb_side(thumb);
    match (side > 0.0, icon_slot) {
        (true, _) => {
            let first = thumb_width.unwrap_or(side);
            let extras = if thumb != ThumbSize::None { extras } else { &[] };
            Some(first + extras.iter().map(|extra| first + between(style, extra)).sum::<f32>())
        }
        (false, slot) => slot,
    }
}

/// How many times the toggle columns' width the row spends before its label: twice when the
/// toggles come after the indent, since the leading edge keeps its empty columns.
fn toggle_rooms(indented: bool) -> f32 {
    if indented { 2.0 } else { 1.0 }
}

/// The least width of a row: everything but the indent and the optional items, and `min_label`.
fn min_width(style: &ListRowStyle, (columns, ends, indented): (usize, usize, bool), column: f32, leading: Option<f32>, items: &[(f32, bool)]) -> f32 {
    let required: f32 = items.iter().filter(|(_, optional)| !optional).map(|(w, _)| w + style.gap).sum();
    let leading = leading.map_or(0.0, |side| side + style.gap);
    columns as f32 * style.toggle_column * toggle_rooms(indented)
        + style.pad_x
        + column
        + leading
        + style.min_label
        + required
        + style.trailing_pad
        + ends as f32 * style.toggle_column
}

/// The cell of the [end column](ToggleColumn::at_end) `index` of `columns` in a row of `rect`:
/// `style.toggle_column` wide, the row's height, after the trailing items, the end columns in
/// [`ListRow::toggle`] order towards the trailing edge, the last one `style.trailing_pad` from it;
/// mirrored with the row ([`Geometry::span`]).
///
/// Filling it also changes the callers: [`min_width`] and [`Geometry::new`] count only the
/// leading columns for the reading-start room and the dividers (`paint_background`'s
/// `columns`), and reserve `n_end × style.toggle_column` at the trailing end before the trailing
/// items are placed; a leading column's offset counts only the leading columns before it.
/// Rows without end columns lay out exactly as before.
fn end_cell(geometry: &Geometry, style: &ListRowStyle, rect: Rect, columns: &[ToggleColumn], index: usize) -> Rect {
    // pc: crates/ui-egui/src/panels/brush_panel.rs:325 (the lock's 18 pt square centred 11 pt in
    // from the row's right edge, after the name).
    let place = columns[..index].iter().filter(|c| c.at_end).count();
    geometry.span(rect, geometry.ends_start + place as f32 * style.toggle_column, style.toggle_column)
}

/// Where the parts of a row lie, as offsets from the reading start of the row.
struct Geometry {
    rtl: bool,
    /// Where the first toggle cell starts.
    toggles_start: f32,
    /// Where the first end column starts; it moves out with the items of a row too narrow.
    ends_start: f32,
    column_start: f32,
    leading_start: f32,
    label_start: f32,
    label_end: f32,
    /// Offset and width of each trailing item; `None` for one that dropped out.
    items: Vec<Option<(f32, f32)>>,
}

impl Geometry {
    /// `indent` shrinks to keep the label's room; `column` (a tree row's triangle) doesn't.
    fn new(
        style: &ListRowStyle,
        rect: Rect,
        rtl: bool,
        (columns, ends, indented): (usize, usize, bool),
        (indent, column): (f32, f32),
        leading: Option<f32>,
        items: &[(f32, bool)],
    ) -> Geometry {
        let leading_columns = columns as f32 * style.toggle_column;
        let base = leading_columns * toggle_rooms(indented) + style.pad_x;
        let ends_start = rect.width() - style.trailing_pad - ends as f32 * style.toggle_column;
        let end = ends_start;
        let leading_width = leading.map_or(0.0, |side| side + style.gap);
        // Trailing items are placed first. Optional ones stay while the label keeps its least
        // width at the full indent; the indent then shrinks for what is left.
        let mut used: f32 = items.iter().filter(|(_, optional)| !optional).map(|(w, _)| w + style.gap).sum();
        let mut kept: Vec<bool> = items.iter().map(|(_, optional)| !optional).collect();
        for (i, (width, optional)) in items.iter().enumerate() {
            if *optional && end - (used + width + style.gap) - (base + indent + column + leading_width) >= style.min_label {
                kept[i] = true;
                used += width + style.gap;
            }
        }
        // A row too narrow for its required parts keeps the label's least width; the items that
        // follow it move out with it and the row's edge cuts them.
        let label_end = end - used;
        let overflow = (base + column + leading_width + style.min_label - label_end).max(0.0);
        let label_end = label_end + overflow;
        let indent = indent.min((label_end - base - column - leading_width - style.min_label).max(0.0));
        let mut from_end = 0.0;
        let items = items
            .iter()
            .zip(&kept)
            .map(|((width, _), kept)| {
                kept.then(|| {
                    let start = end - from_end - width + overflow;
                    from_end += width + style.gap;
                    (start, *width)
                })
            })
            .collect();
        // Indented toggles follow the empty columns of the leading edge and the indent.
        let toggles_start = if indented { leading_columns + indent } else { 0.0 };
        Geometry {
            rtl,
            toggles_start,
            ends_start: ends_start + overflow,
            column_start: base + indent,
            leading_start: base + indent + column,
            label_start: base + indent + column + leading_width,
            label_end,
            items,
        }
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

/// What a row's background shows: the selection, the pointer (or a highlight), and the form.
struct Fill {
    selected: bool,
    lit: bool,
    layer: bool,
    striped: bool,
}

fn paint_background(ui: &Ui, style: &ListRowStyle, rect: Rect, Fill { selected, lit, layer, striped }: Fill, columns: usize, geometry: &Geometry) {
    let fill = match (selected, lit, layer) {
        // The layer form shows no change under the pointer once selected.
        (true, _, true) => style.layer_selected,
        (true, true, false) => style.selected_hover,
        (true, false, false) => style.selected,
        (false, true, true) => style.layer_hover,
        (false, true, false) => style.hover,
        (false, false, _) => Color32::TRANSPARENT,
    };
    let painter = ui.painter();
    let radius = corner_radius(if layer { style.layer_radius } else { style.radius });
    if striped {
        painter.rect_filled(rect, radius, style.stripe);
    }
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, radius, fill);
    }
    if layer && selected && style.layer_outline != Color32::TRANSPARENT {
        painter.rect_stroke(rect, radius, Stroke::new(1.0, style.layer_outline), StrokeKind::Inside);
    }
    if layer && style.dividers {
        let ppp = ui.pixels_per_point();
        painter.hline(rect.x_range(), (rect.bottom() - style.divider.width / 2.0).round_to_pixel_center(ppp), style.divider);
        for i in 1..=columns {
            painter.vline(geometry.edge(rect, i as f32 * style.toggle_column).round_to_pixel_center(ppp), rect.y_range(), style.divider);
        }
    }
}

/// The [`ListRow::rule`] under a row of `rect`, when `rule`: `style.divider` along the bottom
/// edge, on the last physical pixel inside the row (as the layer form's divider), unless `layer`
/// already drew it there (`style.dividers` on). Nothing without `rule`.
fn paint_rule(ui: &Ui, style: &ListRowStyle, rect: Rect, (rule, layer): (bool, bool)) {
    if !rule || layer && style.dividers || !ui.is_rect_visible(rect) {
        return;
    }
    let y = (rect.bottom() - style.divider.width / 2.0).round_to_pixel_center(ui.pixels_per_point());
    ui.painter().hline(rect.x_range(), y, style.divider);
}

/// [`ListRow::thumb_drags`], when `thumb_drags`: when `response` starts a primary drag
/// (`drag_started_by(Primary)`) whose press origin (`pointer.press_origin()`) lies in an
/// [`Extra`](ThumbTarget::Extra) thumbnail of `thumbs` (the rects drawn this pass), reports
/// [`RowEvent::ThumbDrag`] for it. [`List`](super::List)'s `row_pointer` reads the event and skips
/// its `start_drag`. Nothing without `thumb_drags`.
fn report_thumb_drag<K: Clone>(
    ui: &Ui,
    (thumb_drags, response): (bool, &Response),
    key: &K,
    thumbs: &[(ThumbTarget, Rect)],
    events: &mut Vec<RowEvent<K>>,
) {
    if !thumb_drags || !response.drag_started_by(egui::PointerButton::Primary) {
        return;
    }
    let Some(origin) = ui.input(|i| i.pointer.press_origin()) else { return };
    if let Some((which @ ThumbTarget::Extra(_), _)) = thumbs.iter().find(|(_, rect)| rect.contains(origin)) {
        events.push(RowEvent::ThumbDrag { row: key.clone(), which: *which });
    }
}

/// The link glyph between two thumbnails in `slot`: a click asks for the opposite state. It is a
/// Tab stop that shows the focus ring, so the keyboard reaches it.
#[expect(clippy::too_many_arguments)]
fn show_link<K: Clone>(
    ui: &mut Ui,
    style: &ListRowStyle,
    slot: Rect,
    (row_id, index): (Id, usize),
    key: &K,
    row_label: &str,
    (linked, name): (bool, &str),
    events: &mut Vec<RowEvent<K>>,
) {
    let name = tree::with_row(name, row_label);
    let response = Interactive::with_id(row_id.with(("link", index)), Role::Button, name.clone())
        .sense(Sense::click())
        .checked(linked)
        .state_layer(false)
        .interact_at(ui, slot)
        .finish(ui);
    let color = match (linked, response.hovered()) {
        (_, true) => style.icon_hover,
        (true, false) => style.icon,
        (false, false) => style.dimmed,
    };
    let icon = if linked { Icon::Link } else { Icon::Unlink };
    icons::paint_in(ui, slot, icon, icons::active(ui.ctx()), style.icon_size, IconSize::Md.stroke(), color);
    if response.clicked() {
        events.push(RowEvent::LinkToggled { row: key.clone(), thumb: index, linked: !linked });
    }
    let _ = response.on_hover_text(name);
}

/// The label's color and the detail line's for a row's state: dimmed (`text.faint`) wins over
/// muted, and a selected muted row takes the selected dimmed color.
fn label_colors(style: &ListRowStyle, (selected, dimmed, muted): (bool, bool, bool), layer: bool) -> (Color32, Color32) {
    let (selected_text, selected_dimmed, selected_detail) = if layer {
        (style.layer_selected_text, style.layer_selected_dimmed, style.layer_selected_detail)
    } else {
        (style.selected_text, style.selected_dimmed, style.selected_detail)
    };
    match (selected, dimmed, muted) {
        (true, true, _) | (true, false, true) => (selected_dimmed, selected_detail),
        (true, false, false) => (selected_text, selected_detail),
        (false, true, _) => (style.dimmed, style.detail),
        (false, false, true) => (style.muted, style.detail),
        (false, false, false) => (style.text, style.detail),
    }
}

/// The colour of the target brackets: the row's text colour.
fn bracket_color(style: &ListRowStyle, selected: bool, layer: bool) -> Color32 {
    match (selected, layer) {
        (true, true) => style.layer_selected_text,
        (true, false) => style.selected_text,
        (false, _) => style.text,
    }
}

/// The down-right clipping arrow in `slot`: a stem down from the top, then a leg to the right
/// ending in an arrowhead (mirrored in a right-to-left row).
fn paint_clip_arrow(ui: &Ui, style: &ListRowStyle, slot: Rect, rtl: bool) {
    let side = slot.width().min(slot.height());
    let box_ = Rect::from_center_size(slot.center(), Vec2::splat(side));
    let at = |x: f32, y: f32| pos2(if rtl { box_.right() - x * side } else { box_.left() + x * side }, box_.top() + y * side);
    let stroke = Stroke::new(IconSize::Md.stroke(), style.icon);
    let painter = ui.painter();
    painter.add(egui::Shape::line(vec![at(0.2, 0.2), at(0.2, 0.7), at(0.8, 0.7)], stroke));
    painter.add(egui::Shape::line(vec![at(0.6, 0.45), at(0.85, 0.7), at(0.6, 0.95)], stroke));
}

/// Corner brackets just outside thumbnail `r`, marking it as the target of the row's editing
/// (PhotoCraft's `paint_brackets`: 3 points out, 6 long, 1.5 wide).
fn paint_brackets(painter: &Painter, r: Rect, color: Color32, style: &ListRowStyle) {
    let (outset, arm) = (style.bracket_outset, style.bracket_arm);
    let r = r.expand(outset);
    let stroke = Stroke::new(1.5, color);
    for (corner, dx, dy) in [(r.left_top(), 1.0, 1.0), (r.right_top(), -1.0, 1.0), (r.right_bottom(), -1.0, -1.0), (r.left_bottom(), 1.0, -1.0)] {
        painter.line_segment([corner, corner + vec2(arm * dx, 0.0)], stroke);
        painter.line_segment([corner, corner + vec2(0.0, arm * dy)], stroke);
    }
}

/// The thumbnail with the host's painting over all of `at`, and its frame just outside it.
fn paint_thumb(ui: &Ui, style: &ListRowStyle, at: Rect, thumbnail: Option<Thumbnail<'_>>, selected: bool) {
    if let Some(paint) = thumbnail {
        paint(&ui.painter().with_clip_rect(at), at);
    }
    let color = if selected { style.thumb_border_selected } else { style.thumb_border };
    ui.painter().rect_stroke(at, corner_radius(style.thumb_radius), Stroke::new(1.0, color), StrokeKind::Outside);
}

/// The "\u{2026}" button in `at` and its menu; the command chosen this pass. The button shows on a
/// hovered or selected row and while its menu is open, and is reached by the pointer and the
/// screen reader, not the Tab key (the list is the one focus stop).
fn show_menu(
    ui: &mut Ui,
    style: &ListRowStyle,
    at: Rect,
    (menu, label): (&RowMenu, &str),
    (row_id, hovered, selected): (Id, bool, bool),
) -> Option<RowCommand> {
    let name = tree::with_row(&RowMenuLabels::of(ui).more, label);
    let sensed = Interactive::with_id(row_id.with("row-menu"), Role::Button, name).sense(Sense::CLICK).state_layer(false).focus_ring(false);
    let button = sensed.interact_at(ui, at).finish(ui);
    let open = ContextMenu::is_open(ui.ctx(), &button);
    if ui.is_rect_visible(at) && (hovered || selected || open) {
        let color = if button.hovered() || open { style.icon_hover } else { style.icon };
        icons::paint_in(ui, at, Icon::More, icons::active(ui.ctx()), style.icon_size, IconSize::Md.stroke(), color);
    }
    let request = (button.clicked() && !open).then_some(OpenAt::Rect(button.rect));
    ContextMenu::new(&menu.entries).own_gestures(false).open_at(request).show(ui, &button).chosen
}

/// The icon, or the host's glyph, in the slot `at`.
fn paint_icon_slot(ui: &Ui, style: &ListRowStyle, at: Rect, glyph: Option<Thumbnail<'_>>, icon: Option<Icon>) {
    if let Some(paint) = glyph {
        paint(&ui.painter().with_clip_rect(at), at);
    } else if let Some(icon) = icon {
        icons::paint_in(ui, at, icon, icons::active(ui.ctx()), style.icon_size, IconSize::Md.stroke(), style.icon);
    }
}

/// The label, and the detail line under it, centred in `band` and cut with an ellipsis.
fn paint_label(
    ui: &Ui,
    tokens: &UiTokens,
    style: &ListRowStyle,
    band: Rect,
    label: &str,
    detail: Option<(&str, Color32)>,
    (color, italic, label_role): (Color32, bool, TypeRole),
) {
    if band.width() < 1.0 {
        return;
    }
    let total = style.label_line + detail.map_or(0.0, |_| style.detail_line);
    let top = band.center().y - total / 2.0;
    // Painted, not a `Label`: the row's own node carries the text, so a label node would name it twice.
    let line = |ui: &Ui, top: f32, height: f32, text: &str, role: TypeRole, color: Color32, italics: bool| {
        let at = Rect::from_x_y_ranges(band.x_range(), top..=top + height);
        let mut format = role_format(tokens, role, color);
        format.italics = italics;
        let mut job = egui::text::LayoutJob::single_section(text.to_owned(), format);
        job.wrap.max_width = at.width();
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        let galley = ui.painter().layout_job(job);
        let x = if direction::is_rtl(ui) { at.right() - galley.size().x } else { at.left() };
        ui.painter().galley(pos2(x, at.center().y - galley.size().y / 2.0), galley, color);
    };
    line(ui, top, style.label_line, label, label_role, color, italic);
    if let Some((detail, detail_color)) = detail {
        line(ui, top + style.label_line, style.detail_line, detail, style.detail_role, detail_color, false);
    }
}

/// A drop indicator line for a target that is not a list row: `from` to `to` (axis-aligned,
/// horizontal when their y matches, vertical otherwise), centred on the segment, drawn in
/// `list.drop` at `list.drop_width` on its `list.drop_halo` outline. It is the line a
/// [`ListRow`] draws for [`DropMark::Before`] and [`DropMark::After`], for headers, buttons and
/// other rows of the host's own.
pub fn drop_line(ui: &Ui, from: Pos2, to: Pos2) {
    let style = ListRow::<u8>::style(&Tokens::of(ui));
    paint_drop_line(ui, &style, from, to);
}

fn paint_drop_line(ui: &Ui, style: &ListRowStyle, from: Pos2, to: Pos2) {
    let (width, halo) = (style.drop_width, style.drop_halo);
    let painter = ui.painter();
    let reach = |across: f32| {
        let across = vec2(0.0, across / 2.0);
        let (min, max) = (from.min(to), from.max(to));
        if from.y == to.y { Rect::from_min_max(min - across, max + across) } else { Rect::from_min_max(min - across.yx(), max + across.yx()) }
    };
    painter.rect_filled(reach(width + 2.0 * halo.width), 0.0, halo.color);
    painter.rect_filled(reach(width), 0.0, style.drop);
}

/// The drop marker of a row of the host's own at `rect`, as a [`ListRow`] draws it for
/// [`ListRow::drop_mark`]. A host that builds its panel from the list parts finds the target of
/// its drag after the rows are laid out (with [`Reorder::resolve`](crate::behavior::Reorder::resolve)
/// on their spans) and calls this in the same pass, after the rows, so the marker is the
/// target of this pass and not of the one before.
pub fn paint_drop_mark(ui: &Ui, rect: Rect, mark: DropMark) {
    let style = ListRow::<u8>::style(&Tokens::of(ui));
    paint_row_mark(ui, &style, rect, mark);
}

/// The marker on its halo, both inside the row, so a row drawn after this one can't cover the
/// halo.
fn paint_row_mark(ui: &Ui, style: &ListRowStyle, rect: Rect, mark: DropMark) {
    let (width, halo) = (style.drop_width, style.drop_halo);
    let painter = ui.painter();
    let band = width + 2.0 * halo.width;
    match mark {
        DropMark::Before | DropMark::After => {
            let y = if mark == DropMark::Before { rect.top() + band / 2.0 } else { rect.bottom() - band / 2.0 };
            paint_drop_line(ui, style, pos2(rect.left(), y), pos2(rect.right(), y));
        }
        DropMark::Into => {
            painter.rect_stroke(rect, corner_radius(style.radius), Stroke::new(band, halo.color), StrokeKind::Inside);
            let inner = corner_radius((style.radius - halo.width).max(0.0));
            painter.rect_stroke(rect.shrink(halo.width), inner, Stroke::new(width, style.drop), StrokeKind::Inside);
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
pub(super) enum ThumbArg {
    #[choice(name = "None")]
    None,
    #[choice(name = "Small")]
    Small,
    #[choice(name = "Medium")]
    Medium,
    #[choice(name = "Large")]
    Large,
}

impl ThumbArg {
    fn size(self) -> ThumbSize {
        match self {
            ThumbArg::None => ThumbSize::None,
            ThumbArg::Small => ThumbSize::Small,
            ThumbArg::Medium => ThumbSize::Medium,
            ThumbArg::Large => ThumbSize::Large,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct LayersArgs {
    pub(super) thumbnails: ThumbArg,
    pub(super) detail: bool,
    #[arg(name = "highlight second row")]
    pub(super) highlight_second_row: bool,
    #[arg(name = "layer form")]
    pub(super) layer_form: bool,
    #[arg(range = 60.0..=480.0)]
    pub(super) width: f32,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/list-row/layers",
            Level::Composites,
            "ListRow",
            "Layers",
            LayersArgs { thumbnails: ThumbArg::None, detail: false, highlight_second_row: false, layer_form: true, width: 280.0 },
            show_layers
        ),
        typed_story!(
            "composites/list-row/masks",
            Level::Composites,
            "ListRow",
            "Mask thumbnail",
            MasksArgs { linked: true, width: 280.0 },
            show_masks
        ),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct MasksArgs {
    linked: bool,
    #[arg(range = 120.0..=480.0)]
    width: f32,
}

/// A layer with a layer mask and a vector mask, each behind its own link glyph, a clipped layer
/// and an italic Background. A click on a thumbnail moves the target brackets to it.
fn show_masks(ui: &mut Ui, args: &mut MasksArgs) {
    let width = args.width.min(ui.available_width());
    let state_id = ui.scope_id().with("list-row-masks");
    let mut linked = ui.data(|d| d.get_temp::<[bool; 2]>(state_id)).unwrap_or([args.linked; 2]);
    let target_id = state_id.with("target");
    let mut target = ui.data(|d| d.get_temp::<ThumbTarget>(target_id)).unwrap_or(ThumbTarget::Extra(0));
    let tokens = Tokens::of(ui);
    let (layer_fill, mask_fill) = (tokens.color.surface.raised.to_egui(), tokens.color.text.faint.to_egui());
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = 0.0;
        for (i, name) in ["Layer with masks", "Clipped layer", "Background"].into_iter().enumerate() {
            let mut row = ListRow::new(i, name).thumb(ThumbSize::Medium, move |painter, rect| {
                painter.rect_filled(rect, 0.0, layer_fill);
            });
            match i {
                0 => {
                    row = row
                        .extra_thumb(move |painter, rect| {
                            painter.rect_filled(rect, 0.0, mask_fill);
                        })
                        .thumb_link(linked[0], "Link the layer mask to {row}")
                        .extra_thumb(move |painter, rect| {
                            painter.rect_filled(rect, 0.0, mask_fill);
                        })
                        .thumb_link(linked[1], "Link the vector mask to {row}")
                        .thumb_target(target)
                        .selected(true);
                }
                1 => row = row.clipped(true),
                _ => row = row.italic(true),
            }
            for event in row.show(ui).events {
                match event {
                    RowEvent::LinkToggled { thumb, linked: on, .. } => linked[thumb.min(1)] = on,
                    RowEvent::ThumbClicked { which, .. } if i == 0 => target = which,
                    _ => {}
                }
            }
        }
    });
    ui.data_mut(|d| {
        d.insert_temp(state_id, linked);
        d.insert_temp(target_id, target);
    });
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

impl Layers {
    fn new(words: &StoryLabels) -> Self {
        let rows = vec![
            layer("Headline", false),
            layer("Photo with a name too long for the panel to show whole", false),
            layer(&words.shadow, false),
            layer("Background", true),
        ];
        Self { rows, selected: Some(0), renaming: None }
    }
}

/// A layers panel `width` wide: eye and lock columns, a thumbnail of the chosen size, a detail
/// line, an optional trailing blend label, a hidden (dimmed) layer once its eye is off, and
/// renaming by double click on the label. Drag down a column to sweep it.
pub(super) fn show_layers(ui: &mut Ui, args: &mut LayersArgs) {
    let width = args.width.min(ui.available_width());
    let thumb = args.thumbnails.size();
    let (detail, highlight, layer) = (args.detail, args.highlight_second_row, args.layer_form);
    let state_id = ui.scope_id().with("list-row-story");
    let mut layers = ui.data(|d| d.get_temp::<Layers>(state_id)).unwrap_or_else(|| Layers::new(&StoryLabels::of(ui)));
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = 0.0;
        for i in 0..layers.rows.len() {
            let row = &layers.rows[i];
            let tokens = Tokens::of(ui);
            let fill = tokens.color.surface.raised.to_egui();
            let mut item = ListRow::new(i, row.name.clone())
                .toggle(ToggleColumn::eye("Visibility {row}", row.visible))
                .toggle(ToggleColumn::lock("Lock {row}", row.locked))
                .thumb(thumb, move |painter, rect| {
                    painter.rect_filled(rect, 0.0, fill);
                })
                .optional_item(56.0, |ui| {
                    ui.label("Normal");
                })
                .selected(layers.selected == Some(i))
                .dimmed(!row.visible)
                .highlight(highlight && i == 1)
                .layer(layer)
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
