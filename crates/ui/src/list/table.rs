//! [`Table`]: rows of cells under a header of columns the user sorts, resizes, hides and moves.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use egui::accesskit::{Action, Role, SortDirection};
use egui::containers::scroll_area::ScrollSource;
use egui::os::OperatingSystem;
use egui::text::LayoutJob;
use egui::{
    Align, AsIdSalt, Color32, Context, CornerRadius, CursorIcon, DragAndDrop, Event, EventFilter, Id, IdSalt, Key, Label, Layout, Modifiers,
    PointerButton, Rect, Response, ScrollArea, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};
use serde::{Deserialize, Serialize};

use super::row::{ListRow, RowEvent};
use super::spans::Spans;
use super::tree::{self, Branch, OpenChange, TreeText};
use super::view::ContextMenuRequest;
use crate::behavior::{
    ClaimedKey, DisplayOrder, KeyClaim, ListKey, ListSelection, Selection, SelectionChange, SelectionMode, TreeMove, first_item, tree_key,
};
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::layout::{ResizeHandle, ResizeHandleStyle, SplitAxis};
use crate::localized::SharedLabels as _;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use crate::patterns::panel::ScrollBarLabels;
use crate::popup_list;
use crate::primitives::{Content, FocusRing, Text};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format, tabular};
use craft_theme::TypeRole;

crate::words! {
    /// The table's words, English by default.
    #[non_exhaustive]
    pub struct TableLabels {
        sorted_ascending: "table.sortedAscending" = "sorted ascending";
        sorted_descending: "table.sortedDescending" = "sorted descending";
        /// A header's accessible name while its column sorts: `{column}` its label, `{order}`
        /// `sorted_ascending` or `sorted_descending` (PdfCraft's "File name, sorted ascending").
        header_sorted: "table.headerSorted" = "{column}, {order}";
        /// The header menu's group label over the column checks (EffectCraft's).
        columns: "table.columns" = "Columns";
        reset_columns: "table.resetColumns" = "Reset Columns";
        /// A header's tooltip (PdfCraft's, Title Case dropped: it is a sentence).
        header_tip: "table.headerTip" = "Click to sort, drag to move the column. Drag its edge to resize it.";
        /// A column edge's accessible name (a `Splitter`): `{column}` the label of the column it
        /// resizes.
        resize_column: "table.resizeColumn" = "Resize {column}";
        /// What the body shows with no rows.
        empty: "table.empty" = "No items";
    }
}

/// Which way a column sorts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

impl SortOrder {
    /// The other order.
    pub fn flipped(self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

/// The column the rows sort by, and which way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableSort<C> {
    pub column: C,
    pub order: SortOrder,
}

impl<C: PartialEq> TableSort<C> {
    /// The sort a click on `column`'s header asks for: the same column flips its order, another
    /// column sorts in `first` (the column's [`first_order`](TableColumn::first_order)); EffectCraft's
    /// Project panel and PdfCraft's Combine Files table sort ascending.
    // effectcraft: crates/ui-egui/src/panels/project.rs:559 (header click)
    pub fn clicked(current: Option<&Self>, column: C, first: SortOrder) -> Self {
        match current {
            Some(sort) if sort.column == column => Self { column, order: sort.order.flipped() },
            _ => Self { column, order: first },
        }
    }

    /// Sorts `rows` by this sort: `compare(a, b, column)` orders two rows ascending, and the
    /// order reverses it. The sort is stable, so rows the column ties keep their order; a host
    /// that wants EffectCraft's fallback (ties by name) compares names in `compare`.
    // effectcraft: crates/ui-egui/src/panels/project.rs:86 sort_items
    pub fn apply<K>(&self, rows: &mut [K], compare: impl Fn(&K, &K, &C) -> Ordering) {
        rows.sort_by(|a, b| {
            let ordering = compare(a, b, &self.column);
            match self.order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
        });
    }
}

/// One column of a [`Table`]: its key, its header and its rules.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn<C> {
    key: C,
    label: String,
    min_width: Option<f32>,
    initial_width: Option<f32>,
    hideable: bool,
    sortable: bool,
    align: Align,
    first_order: SortOrder,
    tip: Option<String>,
}

impl<C> TableColumn<C> {
    /// A column with header `label` (as shown, translated); sortable (ascending first), not
    /// hideable, left aligned, at least `table.min_column` wide, sharing the free width until the
    /// user resizes it.
    pub fn new(key: C, label: impl Into<String>) -> Self {
        Self {
            key,
            label: label.into(),
            min_width: None,
            initial_width: None,
            hideable: false,
            sortable: true,
            align: Align::Min,
            first_order: SortOrder::Ascending,
            tip: None,
        }
    }

    /// The narrowest the user may drag the column (PdfCraft: File name 120, Pages 110, Size 60,
    /// Modified 80, Warnings 100).
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// The order a click on the header sorts the column in when it isn't the sorting column
    /// (default ascending; counts biggest first are descending).
    pub fn first_order(mut self, order: SortOrder) -> Self {
        self.first_order = order;
        self
    }

    /// The header's tooltip, in place of [`TableLabels::header_tip`]: the column's long name when
    /// the label is an abbreviation.
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// The width before the user resizes it; without one the column shares what the fixed
    /// columns leave.
    pub fn initial_width(mut self, width: f32) -> Self {
        self.initial_width = Some(width);
        self
    }

    /// The header menu has a check for it, so the user can hide it (EffectCraft's optional
    /// columns: Type, Size, Duration…). A column that can't be hidden is always shown.
    pub fn hideable(mut self, hideable: bool) -> Self {
        self.hideable = hideable;
        self
    }

    /// A click on its header sorts by it (default on).
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Numbers: right aligned (mirrored in a right-to-left interface), and
    /// [`CellContext::label`] writes them with tabular figures.
    pub fn numeric(mut self) -> Self {
        self.align = Align::Max;
        self
    }

    pub fn key(&self) -> &C {
        &self.key
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    /// The least width: the column's own, else the style's `table.min_column`.
    fn least(&self, style: &TableStyle) -> f32 {
        self.min_width.unwrap_or(style.min_column)
    }
}

/// What a cell is drawn with, given to the cell closure of [`Table::show`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellContext {
    /// The row is selected.
    pub selected: bool,
    /// The text colour of the row: `list.text.selected` in a selected row, `list.text` otherwise.
    pub text: Color32,
    /// The column is [numeric](TableColumn::numeric).
    pub numeric: bool,
}

impl CellContext {
    /// `text` in the body role and the row's text colour, with tabular figures in a numeric
    /// column. The library's `Text` takes its colour from the content, not from the row, so a
    /// cell drawn with it keeps the unselected colour in a selected row.
    pub fn label(&self, ui: &mut Ui, text: impl Into<String>) -> Response {
        let mut format = role_format(&Tokens::of(ui), TypeRole::Body, self.text);
        format.valign = ui.text_valign();
        if self.numeric {
            format = tabular(format);
        }
        ui.add(Label::new(LayoutJob::single_section(text.into(), format)))
    }
}

/// The user's arrangement of a table's columns: order, hidden columns, widths and sort. The app
/// keeps it in its settings (it is serde), one per table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableLayout<C> {
    /// Every column in the order shown, hidden ones included.
    pub order: Vec<C>,
    /// The hidden columns.
    pub hidden: Vec<C>,
    /// The widths the user dragged; empty until a column is resized, and the columns fit the
    /// table's width until then (PdfCraft's `Columns::widths`).
    pub widths: Vec<(C, f32)>,
    pub sort: Option<TableSort<C>>,
}

impl<C: Clone + PartialEq> TableLayout<C> {
    /// The layout of `columns` as the app lists them: in that order, nothing hidden, fitted,
    /// unsorted.
    pub fn new(columns: &[TableColumn<C>]) -> Self {
        Self { order: columns.iter().map(|c| c.key.clone()).collect(), hidden: Vec::new(), widths: Vec::new(), sort: None }
    }

    /// The layout read back from settings, repaired against `columns`: unknown keys dropped,
    /// missing columns appended in the app's order, widths that aren't finite dropped and the
    /// rest clamped to each column's own least width (a column without one is clamped when the
    /// table is shown) and 4000 (PdfCraft's `Columns::from_json`). A hidden column doesn't sort.
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/files.rs:63 from_json
    pub fn repaired(self, columns: &[TableColumn<C>]) -> Self {
        let column = |key: &C| columns.iter().find(|c| c.key == *key);
        let mut order: Vec<C> = Vec::new();
        for key in self.order {
            if column(&key).is_some() && !order.contains(&key) {
                order.push(key);
            }
        }
        for c in columns {
            if !order.contains(&c.key) {
                order.push(c.key.clone());
            }
        }
        let mut hidden: Vec<C> = Vec::new();
        for key in self.hidden {
            if column(&key).is_some_and(|c| c.hideable) && !hidden.contains(&key) {
                hidden.push(key);
            }
        }
        let mut widths: Vec<(C, f32)> = Vec::new();
        for (key, width) in self.widths {
            if let Some(c) = column(&key)
                && width.is_finite()
                && !widths.iter().any(|(k, _)| *k == key)
            {
                widths.push((key, clamp_width(width, c.min_width.unwrap_or(0.0))));
            }
        }
        let sort = self.sort.filter(|sort| column(&sort.column).is_some() && !hidden.contains(&sort.column));
        Self { order, hidden, widths, sort }
    }

    /// Whether `column` is shown.
    pub fn is_visible(&self, column: &C) -> bool {
        !self.hidden.contains(column)
    }

    /// Shows or hides `column`; its place in [`order`](Self::order) stays, so it comes back
    /// where it was (EffectCraft's `set_column` keeps the canonical order). Hiding the sorting
    /// column clears [`sort`](Self::sort), so no row order depends on a column that isn't shown.
    // effectcraft: crates/ui-egui/src/panels/project.rs:1357 set_column
    pub fn set_visible(&mut self, column: &C, visible: bool) {
        self.hidden.retain(|c| c != column);
        if !visible {
            self.hidden.push(column.clone());
            if self.sort.as_ref().is_some_and(|sort| sort.column == *column) {
                self.sort = None;
            }
        }
    }

    /// Moves `column` before `to` (after it with `after`), as a header dropped on another does.
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/table.rs:65 (HeadingEvent::Move)
    pub fn move_column(&mut self, column: &C, to: &C, after: bool) {
        if column == to || !self.order.contains(column) || !self.order.contains(to) {
            return;
        }
        self.order.retain(|c| c != column);
        let at = self.order.iter().position(|c| c == to).map_or(self.order.len(), |p| p + usize::from(after));
        self.order.insert(at, column.clone());
    }
}

/// The look of a table's header and rows, from [`Table::style`].
#[derive(Clone, Debug, PartialEq)]
pub struct TableStyle {
    /// Header height: `table.header_height`, the dock tab height, so a table under a panel's
    /// tabs lines up with them; PdfCraft's is 26.
    pub header_height: f32,
    /// Row height: `list.height` (PdfCraft 22).
    pub row_height: f32,
    /// Between a cell's edge and its text: `list.pad_x` (PdfCraft 4).
    pub cell_pad_x: f32,
    /// The sort glyph's side and its gap after the label: `table.sort_glyph` (PdfCraft's
    /// chevron, 12) and `table.sort_gap` (PdfCraft 14 to its centre, so 8 to its edge).
    pub sort_glyph: f32,
    pub sort_gap: f32,
    /// The resize grip's hit width, centred on a column edge: `table.grip_width`.
    pub grip_width: f32,
    /// Header text: the `Label` role in `text.muted`, the sorting column's in `text.normal`
    /// (PdfCraft's).
    pub header_text: Color32,
    pub header_text_sorted: Color32,
    /// The header's hover and pressed fills: `list.hover` and `state.pressed`.
    pub header_hover: Color32,
    pub header_pressed: Color32,
    /// The drop line a dragged header shows, `table.drop_line_width` (2.5, PdfCraft's) wide in
    /// `accent.solid`, reaching `table.drop_line_slack` past the cell it marks.
    pub drop_line: Color32,
    pub drop_line_width: f32,
    pub drop_line_slack: f32,
    /// The least width of a column that sets none: `table.min_column`.
    pub min_column: f32,
    /// A column edge's fill while hovered or dragged, and while it has the keyboard focus: the
    /// dock dividers' (`dock.resize` and `float.focus`, the dock look's `resize` and `focus`).
    /// Header cells stay square, as Photoshop's and the system file lists' do.
    pub edge: Color32,
    pub edge_focus: Color32,
    /// Every other row's fill (`list.stripe`).
    pub stripe: Color32,
}

/// What the table reported this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TableEvent<K, C> {
    /// A header was clicked, or Space or Enter pressed on one: the layout's sort has changed to
    /// this; the host sorts its rows ([`TableSort::apply`]).
    Sort(TableSort<C>),
    /// The sorting column was hidden, so the layout has no sort: the host puts its rows in their
    /// natural order. [`Layout`](Self::Layout) is reported with it.
    Unsorted,
    /// The user resized (pointer or keys), moved, hid, showed or reset columns: the layout has
    /// changed; the host saves it.
    Layout,
    /// The selection changed, by the rules of [`ListSelection`](crate::behavior::ListSelection).
    Selection(SelectionChange<K>),
    /// Enter or a double click on a row.
    Activate(K),
    /// The row's context menu was asked for (secondary click, Menu key, Shift+F10).
    ContextMenu(ContextMenuRequest<K>),
    /// In a [tree table](Table::tree), a group was asked to open or close (its triangle, ← and
    /// →, Alt+← and Alt+→, a screen reader's Expand and Collapse); the host applies it to its
    /// open state ([`TreeState::apply`](super::TreeState::apply) or its own model) and gives the
    /// next pass the new display order.
    Open(OpenChange<K>),
}

/// What [`Table::show`] returns: a composite of header, edges and body, as
/// [`ListOutput`](super::ListOutput).
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the table's events"]
pub struct TableOutput<K, C> {
    /// The body: its focus and its rect.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<TableEvent<K, C>>,
}

/// A sortable table with resizable, hideable and movable columns, after PdfCraft's Combine Files
/// table, with EffectCraft's Project panel column menu.
///
/// ```ignore
/// let out = Table::new("combine", "Files", &columns, &mut prefs.combine_columns, rows.as_slice())
///     .selection(&mut selection)
///     .show(ui, |ui, file, column, cx| match column {
///         Col::Name => { cx.label(ui, &files[*file].name); }
///         Col::Pages => { cx.label(ui, files[*file].pages.to_string()); }
///         _ => {}
///     });
/// for event in out.events {
///     match event {
///         TableEvent::Sort(sort) => sort.apply(&mut order, |a, b, col| compare(a, b, col)),
///         TableEvent::Unsorted => order = natural_order(),
///         TableEvent::Layout => settings.save(),
///         _ => {}
///     }
/// }
/// ```
///
/// - **Header.** One header cell per shown column, in the layout's order, at the dock tab height.
///   The whole cell sorts by its column on a click ([`TableSort::clicked`]); the sorting column's
///   label is in the primary text colour with a chevron after it (up ascending, down descending;
///   PdfCraft's `chevron-up`/`chevron-down` 12 points, 14 past the label's end). Its tooltip is
///   [`TableLabels::header_tip`] or the column's own [`tip`](TableColumn::tip). The first click
///   sorts in the column's [`first_order`](TableColumn::first_order). Hiding the sorting column
///   clears the sort and reports [`TableEvent::Unsorted`]. A drag on a header moves the column: a 2.5-point accent line
///   marks the edge it would go to (the side of the target's centre the pointer is on), and the
///   release moves it. A drag on the edge between two headers resizes the column on its left
///   (on its right in a right-to-left interface) down to its least width; the cursor shows
///   `ResizeColumn` over the edge. Esc or focus loss during an edge drag puts back the width
///   from before the drag (the dock's resize rule). A double click on an edge fits the column
///   to its widest cell among the rows in view or its header's label, padding included
///   (Photoshop's and the system file lists'; each cell is drawn once in a hidden `Ui` without
///   a width limit). The edge shows the dock dividers' fill while hovered, dragged or focused.
/// - **Edges.** Each shown column's trailing edge is a
///   [`ResizeHandle`](crate::layout::ResizeHandle), the widget the split panes and the dock's
///   column and stack dividers use, so a column resizes by the suite's keyboard rule: the edge
///   is a Tab stop right after its column's header; ← and → (physical, as on every divider) move
///   it by the handle's keyboard step (8 points), down to the column's least width and up to
///   4000; the handle's `value(width, min..=4000)` exposes the width to AccessKit as a
///   `Splitter` named [`TableLabels::resize_column`], with Increment, Decrement and SetValue;
///   `reversed` is set in a right-to-left interface, where the edge is the column's left one and
///   moving it left widens the column. A pointer drag focuses the edge only for the gesture,
///   as on the dividers. Each step changes the layout's widths at once
///   (the first one writes every shown column's width, as the first drag does) and reports
///   [`TableEvent::Layout`] in the same pass.
/// - **Header menu.** A secondary click on the header (on a cell or the empty part after the last
///   one) opens a [`ContextMenu`](crate::menu::ContextMenu): "Columns" over a keep-open check row
///   per hideable column ([`MenuItem::keep_open`](crate::menu::MenuItem::keep_open)), then
///   Reset Columns, which puts back the app's order, shows every column, drops the widths and
///   keeps the sort.
/// - **Widths.** Until the user resizes a column, the columns fit the table: each column with an
///   initial width keeps it and the others share what is left in equal parts, none narrower than
///   its least width; a table narrower than that scrolls sideways. The first resize writes every
///   shown column's width into the layout, which keeps them from then on (PdfCraft).
/// - **Rows.** The host gives the rows as a [`DisplayOrder`] (already sorted) and draws each
///   cell in `cell(ui, key, column, cx)`, in a `Ui` clipped to the cell, left-to-right centred
///   (right aligned for a [numeric](TableColumn::numeric) column). The [`CellContext`] says
///   whether the row is selected and gives its text colour; [`CellContext::label`] draws text
///   in it. Only the rows in view are built. The header stays at the top of the view and scrolls
///   sideways with the columns; the vertical scroll bar is the view's, so it is in sight
///   whatever the columns' width. Rows draw as [`ListRow`](super::ListRow)s do: the selection, hover and stripe fills
///   of the `list.*` tokens.
/// - **Keys and focus.** One Tab stop for the body, as a [`List`](super::List): the list keys and
///   the selection rules of [`ListSelection`](crate::behavior::ListSelection), Enter activates.
///   The headers are buttons the Tab key visits after the body is left (Shift+Tab from the body
///   reaches them), each followed by its column's edge (above); Space or Enter on a header
///   sorts, the Menu key and Shift+F10 open the header menu, and Ctrl+← / Ctrl+→ (Cmd on macOS)
///   move its column. A focused header claims Space, Enter and those chords
///   ([`KeyClaim`](crate::behavior::KeyClaim)), so the app's shortcuts don't take them
///   ([`focus::key_owner`](crate::focus::key_owner) reports a focused widget); every event of
///   the pass is handled in order in that pass.
/// - **AccessKit.** The table is a `Table` named by its label with `row_count` and
///   `column_count`; each header a `ColumnHeader` named by its label, or by
///   [`TableLabels::header_sorted`] while it sorts, with `sort_direction`; each row a `Row`
///   (`selected`) and each cell a `Cell`.
/// - **Tree.** [`tree`](Self::tree) indents the rows by their depth in the display order and adds
///   the tree keys and roles; see there.
pub struct Table<'a, K, C, O: ?Sized> {
    id_salt: IdSalt,
    label: String,
    columns: &'a [TableColumn<C>],
    layout: &'a mut TableLayout<C>,
    rows: &'a O,
    selection: Option<&'a mut Selection<K>>,
    mode: SelectionMode,
    striped: bool,
    movable: bool,
    max_height: Option<f32>,
    labels: Option<TableLabels>,
    /// Set by [`tree`](Self::tree): each row's name for its disclosure triangle.
    tree: Option<RowName<'a, K>>,
    tree_text: Option<TreeText>,
}

impl<'a, K, C, O> Table<'a, K, C, O>
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    C: Clone + PartialEq + Debug + Send + Sync + 'static,
    O: DisplayOrder<K> + ?Sized,
{
    /// A table named `label` for AccessKit, with `columns` arranged by `layout` and showing
    /// `rows`; `id_salt` keeps its focus, scroll position and gestures.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, columns: &'a [TableColumn<C>], layout: &'a mut TableLayout<C>, rows: &'a O) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            columns,
            layout,
            rows,
            selection: None,
            mode: SelectionMode::default(),
            striped: true,
            movable: true,
            max_height: None,
            labels: None,
            tree: None,
            tree_text: None,
        }
    }

    /// The host's selection, which the table edits and reports.
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Single or multiple selection (default multiple).
    pub fn mode(mut self, mode: SelectionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Every other row filled (default on, PdfCraft's `striped(true)`).
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Headers move by drag and by keys (default on).
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// The most the body takes before it scrolls; the available height by default.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: TableLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Makes the table a tree, as [`List::tree`](super::List::tree) makes a list one. The rows'
    /// depths, groups and open states come from the display order ([`DisplayOrder::depth`],
    /// [`has_children`](DisplayOrder::has_children), [`open`](DisplayOrder::open)), and
    /// `row_name` names a row for its disclosure triangle ("Expand {row}", [`TreeText`]): a
    /// table's rows have no label of their own, since the host draws every cell.
    ///
    /// - **Rows.** The first shown column (in the layout's order, whichever column that is)
    ///   carries the tree: an indent of `list.indent` per level (a leaf inside a group one level
    ///   more, past its parent's triangle, as a list's tree rows), then the
    ///   `list.disclosure` column with a group's triangle, then the cell. The indent shrinks
    ///   before the cell gets narrower than `list.min_label`; the triangle column doesn't
    ///   ([`ListRow`]'s rule). A double click on that column's edge fits the widest cell plus its
    ///   indent and triangle column.
    /// - **Triangle.** The list's: down when open, toward the reading end when closed, with the
    ///   [`TreeText`] name and tooltip. A click opens or closes the group, with Alt every group.
    /// - **Keys.** In the focused body, → opens a closed group or moves to its first child; ←
    ///   closes an open group or moves to its parent (swapped in a right-to-left layout); Alt+→
    ///   and Alt+← open or close every group. A move to a child or parent selects that row alone,
    ///   as ↑ and ↓ do. A group that closes over the lead takes the lead, without a selection
    ///   change. These are [`List`](super::List)'s tree keys and rules
    ///   ([`tree_key`](crate::behavior::tree_key)).
    /// - **Events.** Opening and closing are [`TableEvent::Open`]; the host owns the open state.
    ///   Sorting stays the host's: it sorts each group's children among themselves and keeps them
    ///   right after their group in the display order.
    /// - **AccessKit.** The table is a `TreeGrid`; each row a `TreeItem` with `level` (its depth),
    ///   `expanded` and the Expand or Collapse action for a group, and its position among its
    ///   siblings (`position_in_set`, `size_of_set`); a screen reader's Expand and Collapse report
    ///   [`TableEvent::Open`]. Cells stay `Cell`s.
    pub fn tree(mut self, row_name: impl Fn(&K) -> String + 'a) -> Self {
        self.tree = Some(Box::new(row_name));
        self
    }

    /// The words of a tree table's disclosure triangles, translated by the host.
    pub fn tree_text(mut self, text: TreeText) -> Self {
        self.tree_text = Some(text);
        self
    }

    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/table.rs:32 table, :96 heading, :167 table_body
    pub fn show(self, ui: &mut Ui, mut cell: impl FnMut(&mut Ui, &K, &C, &CellContext)) -> TableOutput<K, C> {
        let Table { id_salt, label, columns, layout, rows, selection, mode, striped, movable, max_height, labels, tree, tree_text } = self;
        let labels = labels.map_or_else(|| TableLabels::shared(ui), Arc::new);
        let tree = tree.as_deref().map(|name| TreeRows { name, text: tree_text.map_or_else(|| TreeText::shared(ui), Arc::new) });
        let is_tree = tree.is_some();
        let tokens = Tokens::of(ui);
        let style = table_style(&tokens);
        let rules = ListSelection::new(mode);
        let id = ui.make_persistent_id(id_salt);
        let rtl = direction::is_rtl(ui);
        let mac = matches!(ui.ctx().os(), OperatingSystem::Mac | OperatingSystem::IOS);
        let mut state = State::<K, C>::load(ui.ctx(), id);
        let mut own = std::mem::take(&mut state.own);
        let selection: &mut Selection<K> = match selection {
            Some(selection) => selection,
            None => &mut own,
        };
        let focused = ui.memory(|m| m.has_focus(id));
        let spans = Spans::uniform(style.row_height, rows.len());
        let mut events: Vec<TableEvent<K, C>> = Vec::new();
        let mut pass = Pass { rows, rules, selection, events: &mut events, focus: false, lead_row: None, reveal: state.reveal.take(), tree };

        if focused && !state.focused && FocusRing::visible(ui.ctx()) {
            pass.reveal = pass.lead_at().map(|at| rows.key(at).clone());
        }
        let mut menu_key = false;
        // An open popup (the header menu) takes the keys.
        if focused && ui.is_enabled() && !popup_list::any_open(ui.ctx()) {
            let page = state.viewport.map_or(1, |h| spans.page(state.offset..state.offset + h));
            menu_key = pass.keys(ui, page, mac);
        }
        let scroll_to = pass.scroll_target(&spans, state.offset, state.viewport);

        let mut shown = shown_columns(columns, layout);
        // A solid scroll bar takes room from the columns once the rows overflow.
        let bar = if state.viewport.is_some_and(|height| spans.total() > height) { ui.spacing().scroll.allocated_width() } else { 0.0 };
        let room = ui.available_width() - bar;
        let mut widths = resolve_widths(columns, &shown, layout, room, &style);
        let header_height = style.header_height;
        let (offset, viewport_height) = (state.offset, state.viewport);
        let builder = UiBuilder::new()
            .scope_id(id)
            .sense(Sense::CLICK | Sense::FOCUSABLE)
            .accessibility_role(if is_tree { Role::TreeGrid } else { Role::Table })
            .accessibility_label(label)
            .layout(Layout::top_down(Align::Min));
        let response = ui
            .scope_builder(builder, |ui| {
                let mut scroll = ScrollArea::both()
                    .id_salt("rows")
                    .vertical_scroll_bar_label(ScrollBarLabels::name(ui))
                    .auto_shrink([false, false])
                    .content_margin(0.0)
                    .scroll_source(ScrollSource::SCROLL_BAR | ScrollSource::MOUSE_WHEEL);
                if let Some(height) = max_height {
                    scroll = scroll.max_height(height);
                }
                if let Some(offset) = scroll_to {
                    scroll = scroll.vertical_scroll_offset(offset);
                }
                let out = scroll.show_viewport(ui, |ui, viewport| {
                    let content_width = widths.iter().sum::<f32>().max(viewport.width());
                    ui.set_width(content_width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    // The header stays at the top of the view while the rows scroll under it; it
                    // scrolls sideways with them, and the vertical bar is the view's.
                    let top = ui.max_rect().min + vec2(0.0, viewport.min.y);
                    let header_rect = Rect::from_min_size(top, vec2(content_width, header_height));
                    let header = Header { id, style: &style, labels: &labels, columns, tokens: &tokens, movable, rtl, rect: header_rect };
                    let first = shown.first().map(|&at| columns[at].key.clone());
                    let mut fit = |ui: &mut Ui, column: &TableColumn<C>| {
                        let Some(height) = viewport_height else { return 0.0 };
                        let visible = spans.in_view(offset..offset + height);
                        let tree = is_tree && first.as_ref() == Some(&column.key);
                        widest_cell(ui, rows, visible, column, rtl, tree, &style, &mut cell)
                    };
                    header.show(ui, layout, &mut shown, &mut widths, &mut state, &mut pass, &mut fit);

                    ui.add_space(header_height);
                    ui.set_clip_rect(ui.clip_rect().intersect(Rect::everything_below(top.y + header_height)));
                    let body = Body {
                        style: &style,
                        labels: &labels,
                        columns,
                        shown: &shown,
                        widths: &widths,
                        striped,
                        rtl,
                        spans: &spans,
                        focused,
                        header_height,
                    };
                    body.draw(ui, viewport, &mut pass, &mut cell);
                });
                state.offset = out.state.clamped_offset().y;
                state.viewport = Some((out.inner_rect.height() - header_height).max(0.0));
            })
            .response;

        pass.background(&response);
        if menu_key {
            pass.menu_for_lead(response.rect);
        }
        if pass.focus {
            ui.memory_mut(|m| m.request_focus(id));
        }
        let focused = ui.memory(|m| m.has_focus(id));
        let active = pass.lead_row.map(|(row, _)| row).filter(|_| focused);
        let bounds = response.rect;
        let (row_count, column_count) = (rows.len(), shown.len());
        ui.ctx().accesskit_node_builder(id, |node| {
            // The `Ui`'s node took its bounds before its rect was known.
            node.set_bounds(egui::accesskit::Rect {
                x0: bounds.min.x.into(),
                y0: bounds.min.y.into(),
                x1: bounds.max.x.into(),
                y1: bounds.max.y.into(),
            });
            node.remove_action(Action::Click);
            node.set_row_count(row_count);
            node.set_column_count(column_count);
            if mode == SelectionMode::Multiple {
                node.set_multiselectable();
            }
            match active {
                Some(row) => node.set_active_descendant(row.accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        FocusRing::paint_if(ui, response.rect, CornerRadius::ZERO, focused && rows.is_empty());
        if focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false });
            });
        }
        state.focused = focused;
        state.reveal = pass.reveal.take();
        state.own = own;
        state.store(ui.ctx(), id);
        TableOutput { response, events }
    }
}

impl Table<'static, (), (), [()]> {
    /// The table's look from the tokens.
    // new: the `table.*` tokens
    pub fn style(tokens: &UiTokens) -> TableStyle {
        table_style(tokens)
    }
}

/// [`TableStyle`] from the `table.*` and `list.*` tokens.
fn table_style(tokens: &UiTokens) -> TableStyle {
    let (list, table) = (&tokens.component.list, &tokens.component.table);
    TableStyle {
        header_height: table.header_height,
        row_height: list.row_height,
        cell_pad_x: list.pad_x,
        sort_glyph: table.sort_glyph,
        sort_gap: table.sort_gap,
        grip_width: table.grip_width,
        header_text: tokens.color.text.muted.to_egui(),
        header_text_sorted: tokens.color.text.normal.to_egui(),
        header_hover: list.hover.to_egui(),
        header_pressed: tokens.color.state.pressed.to_egui(),
        drop_line: tokens.color.accent.solid.to_egui(),
        drop_line_width: table.drop_line_width,
        drop_line_slack: table.drop_line_slack,
        min_column: table.min_column,
        edge: tokens.component.dock.resize.to_egui(),
        edge_focus: tokens.component.float.focus.to_egui(),
        stripe: list.stripe.to_egui(),
    }
}

/// The widest a column gets.
const MAX_WIDTH: f32 = 4000.0;

fn clamp_width(width: f32, min: f32) -> f32 {
    width.min(MAX_WIDTH).max(min)
}

/// The columns shown, as indices into the app's columns, in the layout's order: the layout's
/// order first, then any column it doesn't list.
fn shown_columns<C: Clone + PartialEq>(columns: &[TableColumn<C>], layout: &TableLayout<C>) -> Vec<usize> {
    let mut all: Vec<usize> = layout.order.iter().filter_map(|key| columns.iter().position(|c| c.key == *key)).collect();
    all.dedup();
    for at in 0..columns.len() {
        if !all.contains(&at) {
            all.push(at);
        }
    }
    all.retain(|&at| layout.is_visible(&columns[at].key));
    all
}

/// The width of each shown column: the layout's where it has one, else the fit. The fit gives each
/// column with an initial width that width and shares what is left in equal parts, none narrower
/// than its least width.
fn resolve_widths<C: Clone + PartialEq>(
    columns: &[TableColumn<C>],
    shown: &[usize],
    layout: &TableLayout<C>,
    available: f32,
    style: &TableStyle,
) -> Vec<f32> {
    let fixed: f32 = shown.iter().filter_map(|&at| columns[at].initial_width.map(|w| w.max(columns[at].least(style)))).sum();
    let free = shown.iter().filter(|&&at| columns[at].initial_width.is_none()).count();
    let share = if free == 0 { 0.0 } else { (available - fixed).max(0.0) / free as f32 };
    shown
        .iter()
        .map(|&at| {
            let column = &columns[at];
            let stored = layout.widths.iter().find(|(key, _)| *key == column.key).map(|(_, w)| *w).filter(|w| w.is_finite());
            let fit = column.initial_width.unwrap_or(share);
            clamp_width(stored.unwrap_or(fit), column.least(style))
        })
        .collect()
}

/// A header being dragged to another place.
#[derive(Clone)]
struct HeaderDrag<C> {
    table: Id,
    column: C,
}

/// What the table keeps between passes, in egui memory under its id.
#[derive(Clone)]
struct State<K, C> {
    /// The selection of a table whose host gives none.
    own: Selection<K>,
    /// A row to scroll into view.
    reveal: Option<K>,
    offset: f32,
    viewport: Option<f32>,
    focused: bool,
    /// The layout's widths before the edge drag under way, put back by Esc or focus loss.
    widths_before_drag: Option<Vec<(C, f32)>>,
}

impl<K, C> Default for State<K, C> {
    fn default() -> Self {
        Self { own: Selection::default(), reveal: None, offset: 0.0, viewport: None, focused: false, widths_before_drag: None }
    }
}

impl<K: Clone + Send + Sync + 'static, C: Clone + Send + Sync + 'static> State<K, C> {
    fn load(ctx: &Context, id: Id) -> Self {
        ctx.data(|d| d.get_temp::<Self>(id)).unwrap_or_default()
    }

    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

/// What a pressed key does in the focused body.
enum Command {
    List(ListKey),
    Menu,
    /// ← or →, with Alt for every group, in a tree table.
    Tree {
        right: bool,
        all: bool,
    },
}

fn take_command(key: Key, modifiers: Modifiers, mac: bool, tree: bool) -> Option<Command> {
    if (key == Key::Menu && modifiers.matches_exact(Modifiers::NONE)) || (key == Key::F10 && modifiers.matches_exact(Modifiers::SHIFT)) {
        return Some(Command::Menu);
    }
    if tree && matches!(key, Key::ArrowLeft | Key::ArrowRight) {
        let all = modifiers.matches_exact(Modifiers::ALT);
        return (all || modifiers.matches_exact(Modifiers::NONE)).then_some(Command::Tree { right: key == Key::ArrowRight, all });
    }
    ListKey::from_key(key, modifiers, mac).filter(|key| *key != ListKey::Rename).map(Command::List)
}

/// A row's name in a tree table ([`Table::tree`]).
type RowName<'a, K> = Box<dyn Fn(&K) -> String + 'a>;

/// A tree table's parts, set by [`Table::tree`].
struct TreeRows<'p, K> {
    /// A row's name for its disclosure triangle.
    name: &'p (dyn Fn(&K) -> String + 'p),
    text: Arc<TreeText>,
}

/// The selection and the events of one pass, shared by the header and the body.
struct Pass<'p, K, C, O: ?Sized> {
    rows: &'p O,
    rules: ListSelection,
    selection: &'p mut Selection<K>,
    events: &'p mut Vec<TableEvent<K, C>>,
    /// The table takes the focus at the end of the pass.
    focus: bool,
    /// The lead row's id and rect, once drawn.
    lead_row: Option<(Id, Rect)>,
    reveal: Option<K>,
    /// Set in a tree table.
    tree: Option<TreeRows<'p, K>>,
}

impl<K, C, O> Pass<'_, K, C, O>
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    O: DisplayOrder<K> + ?Sized,
{
    /// The lead's position, or the first row's when no lead is shown: where the focus sits.
    fn lead_at(&self) -> Option<usize> {
        self.selection
            .lead
            .as_ref()
            .and_then(|lead| self.rows.position(lead))
            .filter(|&at| !self.rows.is_section(at))
            .or_else(|| first_item(self.rows))
    }

    fn keyed(&self) -> Selection<K> {
        let mut keyed = self.selection.clone();
        keyed.lead = self.lead_at().map(|at| self.rows.key(at).clone());
        keyed
    }

    fn apply(&mut self, change: SelectionChange<K>) {
        self.selection.clone_from(&change.selection);
        self.events.push(TableEvent::Selection(change));
    }

    /// Takes the list keys of the focused body; true when the Menu key or Shift+F10 asked for the
    /// lead's context menu.
    fn keys(&mut self, ui: &Ui, page: usize, mac: bool) -> bool {
        let mut commands = Vec::new();
        let tree = self.tree.is_some();
        let rtl = direction::is_rtl(ui);
        ui.input_mut(|i| {
            i.events.retain(|event| {
                let Event::Key { key, pressed: true, modifiers, .. } = event else { return true };
                match take_command(*key, *modifiers, mac, tree) {
                    Some(command) => {
                        commands.push(command);
                        false
                    }
                    None => true,
                }
            });
        });
        let mut menu = false;
        for command in commands {
            match command {
                Command::List(key) => self.list_key(key, page),
                Command::Menu => {
                    self.reveal = self.lead_at().map(|at| self.rows.key(at).clone());
                    menu = true;
                }
                Command::Tree { right, all } => self.tree_key(right != rtl, all),
            }
        }
        menu
    }

    /// The tree keys on the lead ([`Table::tree`]): toward the end opens a closed group or moves
    /// to its first child, toward the start closes an open group or moves to its parent; with
    /// `all`, every group opens or closes ([`TableEvent::Open`] with `all`). A move selects the
    /// row alone through `self.rules.click(.., Modifiers::NONE)` and reveals it. A group closing
    /// over the lead (every group: the lead's topmost ancestor, `tree::topmost`; one group:
    /// `tree::is_below`) takes the lead in `self.selection.lead` without a selection event, and
    /// is revealed.
    fn tree_key(&mut self, toward_end: bool, all: bool) {
        let Some(at) = self.lead_at() else { return };
        if all {
            let row = self.rows.key(at).clone();
            self.open_change(at, OpenChange { row, open: toward_end, all: true });
            return;
        }
        match tree_key(self.rows, at, toward_end) {
            Some(TreeMove::Open(at)) => {
                let row = self.rows.key(at).clone();
                self.open_change(at, OpenChange { row, open: true, all: false });
            }
            Some(TreeMove::Close(at)) => {
                let row = self.rows.key(at).clone();
                self.open_change(at, OpenChange { row, open: false, all: false });
            }
            Some(TreeMove::To(to)) => {
                if let Some(change) = self.rules.click(self.selection, self.rows, to, Modifiers::NONE) {
                    self.apply(change);
                }
                self.reveal = Some(self.rows.key(to).clone());
            }
            None => {}
        }
    }

    /// Reports a group asked to open or close. The lead moves to the group that closes over it
    /// (to the topmost one when all close), since the rows below it leave the display order; the
    /// selection is unchanged.
    fn open_change(&mut self, at: usize, change: OpenChange<K>) {
        if !change.open
            && let Some(lead) = self.selection.lead.as_ref().and_then(|lead| self.rows.position(lead))
        {
            let target = if change.all { Some(tree::topmost(self.rows, lead)) } else { tree::is_below(self.rows, at, lead).then_some(at) };
            if let Some(target) = target.filter(|&t| t != lead) {
                self.selection.lead = Some(self.rows.key(target).clone());
                self.reveal = self.selection.lead.clone();
            }
        }
        self.events.push(TableEvent::Open(change));
    }

    fn list_key(&mut self, key: ListKey, page: usize) {
        let keyed = self.keyed();
        let lead = keyed.lead.clone();
        match key {
            ListKey::Select { step, extend } => {
                if let Some(change) = self.rules.select(&keyed, self.rows, step, page, extend) {
                    self.apply(change);
                }
            }
            ListKey::MoveLead(step) => {
                if let Some(to) = self.rules.move_lead(&keyed, self.rows, step, page) {
                    self.selection.lead = Some(to);
                }
            }
            ListKey::ToggleLead => {
                if let Some(change) = self.rules.toggle_lead(&keyed, self.rows) {
                    self.apply(change);
                }
            }
            ListKey::Activate => {
                if let Some(lead) = lead.clone() {
                    self.events.push(TableEvent::Activate(lead));
                }
            }
            ListKey::Rename => {}
        }
        self.reveal = self.selection.lead.clone().or(lead);
    }

    fn scroll_target(&mut self, spans: &Spans, offset: f32, viewport: Option<f32>) -> Option<f32> {
        let height = viewport?;
        let key = self.reveal.take()?;
        spans.reveal(self.rows.position(&key)?, offset, height)
    }

    /// A press on the empty space gives the table the focus; a click there clears the selection,
    /// and a secondary click asks for a context menu without a row.
    fn background(&mut self, response: &Response) {
        if response.is_pointer_button_down_on() || response.clicked_by(PointerButton::Primary) {
            self.focus = true;
        }
        if response.clicked_by(PointerButton::Primary)
            && let Some(change) = self.rules.click_empty(self.selection)
        {
            self.apply(change);
        }
        if response.secondary_clicked() {
            self.focus = true;
            let request =
                ContextMenuRequest { row: None, selected: false, anchor: response.rect, pointer: response.interact_pointer_pos(), toggle: None };
            self.events.push(TableEvent::ContextMenu(request));
        }
    }

    /// The Menu key or Shift+F10: the context menu of the lead, anchored at its row.
    fn menu_for_lead(&mut self, table: Rect) {
        let lead = self.lead_at().map(|at| self.rows.key(at).clone());
        let anchor = self.lead_row.map_or(table, |(_, rect)| rect);
        let selected = lead.as_ref().is_some_and(|k| self.selection.contains(k));
        self.events.push(TableEvent::ContextMenu(ContextMenuRequest { row: lead, selected, anchor, pointer: None, toggle: None }));
    }
}

/// What a header menu row does.
#[derive(Clone, Debug, PartialEq)]
enum HeaderCommand {
    /// The "Columns" label row.
    Label,
    /// Show or hide the column at this index of the app's columns.
    Toggle(usize),
    Reset,
}

/// What a header cell and its edge reported, kept to draw them once the widths are settled.
struct CellDraw {
    /// Index of the column in the app's columns.
    at: usize,
    response: Response,
    dragged: bool,
    /// While another header is dragged over this one: whether the drop line is at its leading side.
    drop_leading: Option<bool>,
    edge: Response,
}

/// What the header's widgets ask for, applied in order once every header has been read.
enum HeaderAction<C> {
    Sort(C),
    Move {
        column: C,
        to: C,
        after: bool,
    },
    /// The column at this position of the shown columns changes by `delta` points.
    Resize {
        at: usize,
        delta: f32,
    },
    /// Esc or focus loss during an edge drag.
    CancelResize,
}

/// The header row: cells, edges and the menu.
struct Header<'a, C> {
    id: Id,
    style: &'a TableStyle,
    labels: &'a TableLabels,
    columns: &'a [TableColumn<C>],
    tokens: &'a UiTokens,
    movable: bool,
    rtl: bool,
    rect: Rect,
}

impl<C> Header<'_, C>
where
    C: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    /// The x range of a column that starts `from` points into the row and is `width` wide.
    fn span(&self, from: f32, width: f32) -> (f32, f32) {
        if self.rtl {
            (self.rect.right() - from - width, self.rect.right() - from)
        } else {
            (self.rect.left() + from, self.rect.left() + from + width)
        }
    }

    #[expect(clippy::too_many_arguments, reason = "the pieces of one pass")]
    fn show<K, O>(
        &self,
        ui: &mut Ui,
        layout: &mut TableLayout<C>,
        shown: &mut Vec<usize>,
        widths: &mut Vec<f32>,
        state: &mut State<K, C>,
        pass: &mut Pass<'_, K, C, O>,
        fit: &mut dyn FnMut(&mut Ui, &TableColumn<C>) -> f32,
    ) where
        K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
        O: DisplayOrder<K> + ?Sized,
    {
        let ctx = ui.ctx().clone();
        let style = self.style;
        let strip = ui.interact(self.rect, self.id.with("strip"), Sense::CLICK);
        let mut actions: Vec<HeaderAction<C>> = Vec::new();
        let mut menu_at: Option<OpenAt> = strip.secondary_clicked().then(|| ctx.input(|i| i.pointer.interact_pos())).flatten().map(OpenAt::Pointer);
        let dragging = DragAndDrop::payload::<HeaderDrag<C>>(&ctx).filter(|p| p.table == self.id);
        let escape = ctx.input(|i| i.key_pressed(Key::Escape)) || crate::focus::lost(&ctx);
        let mut from = 0.0;
        let order = shown.clone();
        let mut cells: Vec<CellDraw> = Vec::new();
        for (position, (&at, &width)) in order.iter().zip(widths.iter()).enumerate() {
            let column = &self.columns[at];
            let (min_x, max_x) = self.span(from, width);
            let rect = Rect::from_min_max(pos2(min_x, self.rect.top()), pos2(max_x, self.rect.bottom()));
            from += width;
            let sorting = layout.sort.as_ref().filter(|sort| sort.column == column.key).map(|sort| sort.order);
            let header_id = self.id.with(("header", at));
            // The edges own the strip either side of a column boundary.
            let hit = rect.shrink2(vec2(style.grip_width / 2.0, 0.0));
            let response = ui.interact(hit, header_id, if self.movable { Sense::click_and_drag() } else { Sense::click() });
            let name = match sorting {
                Some(order) => {
                    let word = match order {
                        SortOrder::Ascending => &self.labels.sorted_ascending,
                        SortOrder::Descending => &self.labels.sorted_descending,
                    };
                    craft_i18n::fmt(&self.labels.header_sorted, &[("column", column.label.as_str()), ("order", word.as_str())])
                }
                None => column.label.clone(),
            };
            response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &name));
            ctx.accesskit_node_builder(header_id, |node| {
                node.set_role(Role::ColumnHeader);
                node.set_column_index(position);
                match sorting {
                    Some(SortOrder::Ascending) => node.set_sort_direction(SortDirection::Ascending),
                    Some(SortOrder::Descending) => node.set_sort_direction(SortDirection::Descending),
                    None => node.clear_sort_direction(),
                }
            });

            let being_dragged = dragging.as_ref().is_some_and(|p| p.column == column.key);

            if self.movable && response.drag_started() {
                DragAndDrop::set_payload(&ctx, HeaderDrag { table: self.id, column: column.key.clone() });
            }
            let after = |r: &Response| r.ctx.pointer_latest_pos().is_some_and(|p| (p.x > rect.center().x) != self.rtl);
            let drop_leading = response
                .dnd_hover_payload::<HeaderDrag<C>>()
                .is_some_and(|d| d.table == self.id && d.column != column.key)
                .then(|| after(&response) == self.rtl);
            if let Some(dragged) = response.dnd_release_payload::<HeaderDrag<C>>().filter(|d| d.table == self.id && d.column != column.key) {
                actions.push(HeaderAction::Move { column: dragged.column.clone(), to: column.key.clone(), after: after(&response) });
            }
            let hover = response.clone().on_hover_cursor(if being_dragged { CursorIcon::Grabbing } else { CursorIcon::PointingHand });
            hover.on_hover_text(column.tip.as_deref().unwrap_or(self.labels.header_tip.as_str()));
            if response.secondary_clicked()
                && let Some(pos) = ctx.input(|i| i.pointer.interact_pos())
            {
                menu_at = Some(OpenAt::Pointer(pos));
            }
            if response.clicked() && column.sortable {
                actions.push(HeaderAction::Sort(column.key.clone()));
            }
            if response.has_focus() {
                KeyClaim::claim(
                    &ctx,
                    header_id,
                    &[
                        ClaimedKey::exact(Modifiers::NONE, Key::Space),
                        ClaimedKey::exact(Modifiers::NONE, Key::Enter),
                        ClaimedKey::exact(Modifiers::COMMAND, Key::ArrowLeft),
                        ClaimedKey::exact(Modifiers::COMMAND, Key::ArrowRight),
                    ],
                );
                let (left, right, menu) = ctx.input_mut(|i| {
                    (
                        self.movable && i.consume_key(Modifiers::COMMAND, Key::ArrowLeft),
                        self.movable && i.consume_key(Modifiers::COMMAND, Key::ArrowRight),
                        i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10),
                    )
                });
                if left || right {
                    let forward = right != self.rtl;
                    let neighbour = if forward { order.get(position + 1) } else { position.checked_sub(1).and_then(|p| order.get(p)) };
                    if let Some(&to) = neighbour {
                        actions.push(HeaderAction::Move { column: column.key.clone(), to: self.columns[to].key.clone(), after: forward });
                    }
                }
                if menu {
                    menu_at = Some(OpenAt::Rect(rect));
                }
            }

            let edge_x = if self.rtl { rect.left() } else { rect.right() };
            let edge = Rect::from_min_max(pos2(edge_x - style.grip_width / 2.0, rect.top()), pos2(edge_x + style.grip_width / 2.0, rect.bottom()));
            let resize_name = craft_i18n::fmt(&self.labels.resize_column, &[("column", column.label.as_str())]);
            let handle = ResizeHandle::new(
                self.id.with(("edge", at)),
                SplitAxis::Horizontal,
                edge,
                &resize_name,
                ResizeHandleStyle { hovered: Color32::TRANSPARENT, dragged: Color32::TRANSPARENT, focused: Color32::TRANSPARENT },
            )
            .hit_margin(0.0)
            .value(width, column.least(style)..=MAX_WIDTH)
            .reversed(self.rtl)
            .show(ui);
            cells.push(CellDraw { at, response: response.clone(), dragged: being_dragged, drop_leading, edge: handle.response.clone() });
            if handle.response.hovered() || handle.response.dragged() {
                ctx.set_cursor_icon(CursorIcon::ResizeColumn);
            }
            if handle.response.drag_started() {
                state.widths_before_drag = Some(layout.widths.clone());
            }
            // egui ends a drag on Esc itself, so the pass that sees Esc may already report the drag as stopped.
            let cancelled = escape && (handle.response.dragged() || handle.response.drag_stopped()) && state.widths_before_drag.is_some();
            if handle.response.drag_stopped() && !cancelled {
                state.widths_before_drag = None;
            }
            if cancelled {
                actions.push(HeaderAction::CancelResize);
                ctx.stop_dragging();
            } else if handle.response.hovered() && ctx.input(|i| i.pointer.button_double_clicked(PointerButton::Primary)) {
                // The widest cell in view or the header's own label and chevron, with the padding
                // either side.
                let content = fit(ui, column).max(self.label_width(ui, column, sorting.is_some()));
                actions.push(HeaderAction::Resize { at: position, delta: content + 2.0 * style.cell_pad_x - width });
            } else if handle.delta != 0.0 {
                let physical = if self.rtl { -handle.delta } else { handle.delta };
                actions.push(HeaderAction::Resize { at: position, delta: physical });
            }
        }

        let entries = self.menu_entries(layout);
        let chosen = ContextMenu::new(&entries).own_gestures(false).open_at(menu_at).show(ui, &strip).chosen;

        self.apply(actions, chosen, layout, shown, widths, &mut state.widths_before_drag, pass.events);
        self.paint(ui, layout, shown, widths, &cells);
    }

    /// Draws the cells and edges at the widths the pass ended with, so the header and the body
    /// agree.
    fn paint(&self, ui: &Ui, layout: &TableLayout<C>, shown: &[usize], widths: &[f32], cells: &[CellDraw]) {
        let style = self.style;
        let mut from = 0.0;
        for (&at, &width) in shown.iter().zip(widths) {
            let column = &self.columns[at];
            let (min_x, max_x) = self.span(from, width);
            let rect = Rect::from_min_max(pos2(min_x, self.rect.top()), pos2(max_x, self.rect.bottom()));
            from += width;
            let Some(cell) = cells.iter().find(|c| c.at == at) else { continue };
            let sorting = layout.sort.as_ref().filter(|sort| sort.column == column.key).map(|sort| sort.order);
            self.paint_cell(ui, rect, column, sorting, &cell.response, cell.dragged);
            if let Some(leading) = cell.drop_leading {
                let x = if leading { rect.left() } else { rect.right() };
                ui.painter().with_clip_rect(rect.expand(style.drop_line_slack)).vline(
                    x,
                    rect.y_range(),
                    Stroke::new(style.drop_line_width, style.drop_line),
                );
            }
            let edge_x = if self.rtl { rect.left() } else { rect.right() };
            let edge = Rect::from_min_max(pos2(edge_x - style.grip_width / 2.0, rect.top()), pos2(edge_x + style.grip_width / 2.0, rect.bottom()));
            let fill = if cell.edge.dragged() {
                Some(style.edge)
            } else if cell.edge.has_focus() {
                Some(style.edge_focus)
            } else if cell.edge.hovered() {
                Some(style.edge)
            } else {
                None
            };
            if let Some(fill) = fill {
                ui.painter().rect_filled(edge, 0.0, fill);
            }
        }
    }

    fn label_galley(&self, ui: &Ui, column: &TableColumn<C>, color: Color32) -> Arc<egui::Galley> {
        let mut job = LayoutJob::single_section(column.label.clone(), role_format(self.tokens, TypeRole::Label, color));
        job.wrap.max_width = f32::INFINITY;
        ui.fonts_mut(|f| f.layout_job(job))
    }

    /// The width the label takes without padding, and the sort chevron's room while `sorting`.
    fn label_width(&self, ui: &Ui, column: &TableColumn<C>, sorting: bool) -> f32 {
        let text = self.label_galley(ui, column, self.style.header_text).size().x;
        if sorting { text + self.style.sort_gap + self.style.sort_glyph } else { text }
    }

    /// Draws one header cell: its fill, label, sort chevron and focus ring.
    fn paint_cell(&self, ui: &Ui, rect: Rect, column: &TableColumn<C>, sorting: Option<SortOrder>, response: &Response, dragged: bool) {
        let style = self.style;
        let ctx = ui.ctx();
        if dragged || response.is_pointer_button_down_on() {
            ui.painter().rect_filled(rect, CornerRadius::ZERO, style.header_pressed);
        } else if response.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::ZERO, style.header_hover);
        }
        let color = if sorting.is_some() { style.header_text_sorted } else { style.header_text };
        let galley = self.label_galley(ui, column, color);
        let text_width = galley.size().x;
        let pad = style.cell_pad_x;
        let text_x = if self.rtl { rect.right() - pad - text_width } else { rect.left() + pad };
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        painter.galley(pos2(text_x, rect.center().y - galley.size().y / 2.0), galley, color);
        if let Some(order) = sorting {
            let offset = text_width + style.sort_gap + style.sort_glyph / 2.0;
            let x = if self.rtl { rect.right() - pad - offset } else { rect.left() + pad + offset };
            let glyph = Rect::from_center_size(pos2(x, rect.center().y), Vec2::splat(style.sort_glyph));
            let icon = match order {
                SortOrder::Ascending => Icon::ChevronUp,
                SortOrder::Descending => Icon::ChevronDown,
            };
            icons::paint_in(ui, glyph, icon, icons::active(ctx), style.sort_glyph, IconSize::Md.stroke(), color);
        }
        FocusRing::paint_if(ui, rect, CornerRadius::ZERO, response.has_focus());
    }

    /// Applies what the header asked for, in order, and reports `Layout` once if the layout changed.
    #[expect(clippy::too_many_arguments, reason = "the pieces of one pass")]
    fn apply<K>(
        &self,
        actions: Vec<HeaderAction<C>>,
        chosen: Option<HeaderCommand>,
        layout: &mut TableLayout<C>,
        shown: &mut Vec<usize>,
        widths: &mut Vec<f32>,
        widths_before_drag: &mut Option<Vec<(C, f32)>>,
        events: &mut Vec<TableEvent<K, C>>,
    ) {
        let mut layout_changed = false;
        for action in actions {
            match action {
                HeaderAction::Sort(column) => {
                    let first = self.columns.iter().find(|c| c.key == column).map(|c| c.first_order).unwrap_or_default();
                    let sort = TableSort::clicked(layout.sort.as_ref(), column, first);
                    layout.sort = Some(sort.clone());
                    events.push(TableEvent::Sort(sort));
                }
                HeaderAction::Move { column, to, after } => {
                    let before = layout.order.clone();
                    layout.move_column(&column, &to, after);
                    layout_changed |= layout.order != before;
                }
                HeaderAction::Resize { at, delta } => {
                    let Some(&column) = shown.get(at) else { continue };
                    let min = self.columns[column].least(self.style);
                    let width = clamp_width(widths[at] + delta, min);
                    if (width - widths[at]).abs() < 1e-3 {
                        continue;
                    }
                    widths[at] = width;
                    layout.widths.retain(|(key, _)| !shown.iter().any(|&s| self.columns[s].key == *key));
                    layout.widths.extend(shown.iter().zip(widths.iter()).map(|(&s, &w)| (self.columns[s].key.clone(), w)));
                    layout_changed = true;
                }
                HeaderAction::CancelResize => {
                    if let Some(before) = widths_before_drag.take() {
                        layout_changed |= layout.widths != before;
                        layout.widths = before;
                        *widths = resolve_widths(self.columns, shown, layout, self.rect.width(), self.style);
                    }
                }
            }
        }
        match chosen {
            Some(HeaderCommand::Toggle(at)) => {
                let key = &self.columns[at].key;
                let sorted = layout.sort.is_some();
                layout.set_visible(key, !layout.is_visible(key));
                if sorted && layout.sort.is_none() {
                    events.push(TableEvent::Unsorted);
                }
                layout_changed = true;
            }
            Some(HeaderCommand::Reset) => {
                let fresh = TableLayout::new(self.columns);
                layout.order = fresh.order;
                layout.hidden = Vec::new();
                layout.widths = Vec::new();
                layout_changed = true;
            }
            Some(HeaderCommand::Label) | None => {}
        }
        if layout_changed {
            events.push(TableEvent::Layout);
            *shown = shown_columns(self.columns, layout);
            *widths = resolve_widths(self.columns, shown, layout, self.rect.width(), self.style);
        }
    }

    /// "Columns" over a keep-open check row per hideable column, then Reset Columns.
    fn menu_entries(&self, layout: &TableLayout<C>) -> Vec<MenuEntry<HeaderCommand>> {
        let mut entries = Vec::new();
        let hideable: Vec<usize> = (0..self.columns.len()).filter(|&at| self.columns[at].hideable).collect();
        if !hideable.is_empty() {
            entries.push(MenuItem::new(self.labels.columns.as_str(), HeaderCommand::Label).enabled(false).into());
            let mut ordered: Vec<usize> =
                layout.order.iter().filter_map(|key| hideable.iter().copied().find(|&at| self.columns[at].key == *key)).collect();
            let missing: Vec<usize> = hideable.iter().copied().filter(|at| !ordered.contains(at)).collect();
            ordered.extend(missing);
            for at in ordered {
                let column = &self.columns[at];
                entries.push(
                    MenuItem::new(column.label.as_str(), HeaderCommand::Toggle(at)).check(layout.is_visible(&column.key)).keep_open(true).into(),
                );
            }
            entries.push(MenuEntry::Separator);
        }
        entries.push(MenuItem::new(self.labels.reset_columns.as_str(), HeaderCommand::Reset).into());
        entries
    }
}

/// The layout of a column's cells: a numeric column ends where the reading direction ends.
fn cell_layout<C>(column: &TableColumn<C>, rtl: bool) -> Layout {
    if (column.align == Align::Max) != rtl { Layout::right_to_left(Align::Center) } else { Layout::left_to_right(Align::Center) }
}

/// The widest `column`'s cell is among the rows `visible`, drawn without a width limit in a hidden
/// `Ui` (no padding): what a double click on the column's edge fits the column to. In a tree table
/// (`tree`) the column carrying the tree adds each row's indent and disclosure column. A cell that
/// fills whatever width it is given has no natural width and measures the widest a column gets.
#[expect(clippy::too_many_arguments, reason = "the pieces of one pass")]
fn widest_cell<K, C, O>(
    ui: &mut Ui,
    rows: &O,
    visible: impl Iterator<Item = usize>,
    column: &TableColumn<C>,
    rtl: bool,
    tree: bool,
    style: &TableStyle,
    cell: &mut impl FnMut(&mut Ui, &K, &C, &CellContext),
) -> f32
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    O: DisplayOrder<K> + ?Sized,
{
    let list = ListRow::<'_, K>::style(&Tokens::of(ui));
    let text = list.text;
    let context = CellContext { selected: false, text, numeric: column.align == Align::Max };
    let area = Rect::from_min_size(ui.max_rect().min, vec2(MAX_WIDTH, style.row_height));
    let mut widest: f32 = 0.0;
    for at in visible.filter(|&at| !rows.is_section(at)) {
        let builder = UiBuilder::new().id_salt(("fit", at)).max_rect(area).layout(cell_layout(column, rtl)).invisible().sizing_pass();
        let mut child = ui.new_child(builder);
        cell(&mut child, rows.key(at), &column.key, &context);
        let lead = if tree {
            let branch = branch_at(rows, at);
            branch.steps() as f32 * list.indent + if branch.group { list.disclosure } else { 0.0 }
        } else {
            0.0
        };
        widest = widest.max(child.min_rect().width() + lead);
    }
    widest
}

/// The rows below the header.
struct Body<'a, C> {
    style: &'a TableStyle,
    labels: &'a TableLabels,
    columns: &'a [TableColumn<C>],
    shown: &'a [usize],
    widths: &'a [f32],
    striped: bool,
    rtl: bool,
    spans: &'a Spans,
    focused: bool,
    /// The header above the first row, which the rows scroll under.
    header_height: f32,
}

impl<C> Body<'_, C>
where
    C: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    /// Lays out the rows in view, and the lead while the table has the focus (the active
    /// descendant).
    fn draw<K, O>(&self, ui: &mut Ui, viewport: Rect, pass: &mut Pass<'_, K, C, O>, cell: &mut impl FnMut(&mut Ui, &K, &C, &CellContext))
    where
        K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
        O: DisplayOrder<K> + ?Sized,
    {
        let content = ui.max_rect();
        let width: f32 = self.widths.iter().sum::<f32>().max(content.width());
        let header = self.header_height;
        ui.set_min_height(header + self.spans.total());
        if pass.rows.is_empty() {
            ui.add(Text::new(self.labels.empty.as_str()).content(Content::Muted));
            return;
        }
        let lead = pass.lead_at();
        let mut drawn: Vec<usize> = self.spans.in_view(viewport.min.y..viewport.max.y - header).collect();
        drawn.extend(lead.filter(|_| self.focused));
        drawn.sort_unstable();
        drawn.dedup();
        for at in drawn {
            let rect =
                Rect::from_min_size(content.min + vec2(0.0, header + self.spans.top(at)), vec2(width, self.spans.bottom(at) - self.spans.top(at)));
            self.row(ui, at, rect, lead == Some(at), pass, cell);
        }
    }

    fn row<K, O>(
        &self,
        ui: &mut Ui,
        at: usize,
        rect: Rect,
        lead: bool,
        pass: &mut Pass<'_, K, C, O>,
        cell: &mut impl FnMut(&mut Ui, &K, &C, &CellContext),
    ) where
        K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
        O: DisplayOrder<K> + ?Sized,
    {
        let key = pass.rows.key(at).clone();
        let style = self.style;
        let list = ListRow::<'_, K>::style(&Tokens::of(ui));
        let selected = pass.selection.contains(&key);
        let hovered = ui.rect_contains_pointer(rect);
        // The stripe is the row's ground; the state fill goes over it, as `ListRow` paints.
        if self.striped && at % 2 == 1 {
            ui.painter().rect_filled(rect, CornerRadius::ZERO, style.stripe);
        }
        let fill = match (selected, hovered) {
            (true, true) => Some(list.selected_hover),
            (true, false) => Some(list.selected),
            (false, true) => Some(list.hover),
            (false, false) => None,
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
        }
        let text = if selected { list.selected_text } else { list.text };
        let branch = pass.tree.as_ref().map(|_| branch_at(pass.rows, at));
        let role = if branch.is_some() { Role::TreeItem } else { Role::Row };
        let tree = pass.tree.as_ref();
        let mut opened: Vec<OpenChange<K>> = Vec::new();
        let scope = ui.scope_builder(UiBuilder::new().id_salt(("row", &key)).max_rect(rect).sense(Sense::CLICK).accessibility_role(role), |ui| {
            let mut from = 0.0;
            for (position, (&column, &width)) in self.shown.iter().zip(self.widths).enumerate() {
                let (min_x, max_x) =
                    if self.rtl { (rect.right() - from - width, rect.right() - from) } else { (rect.left() + from, rect.left() + from + width) };
                from += width;
                let cell_rect = Rect::from_min_max(pos2(min_x, rect.top()), pos2(max_x, rect.bottom()));
                let mut inner = cell_rect.shrink2(vec2(style.cell_pad_x, 0.0));
                if position == 0
                    && let (Some(tree), Some(branch)) = (tree, &branch)
                {
                    inner = self.tree_lead(ui, tree, inner, branch, &key, &mut opened);
                }
                let column = &self.columns[column];
                let context = CellContext { selected, text, numeric: column.align == Align::Max };
                ui.scope_builder(
                    UiBuilder::new().id_salt(("cell", position)).max_rect(inner).layout(cell_layout(column, self.rtl)).accessibility_role(Role::Cell),
                    |ui| {
                        ui.set_clip_rect(cell_rect.intersect(ui.clip_rect()));
                        ui.visuals_mut().override_text_color = Some(text);
                        ui.ctx().accesskit_node_builder(ui.scope_id(), |node| node.set_column_index(position));
                        cell(ui, &key, &column.key, &context);
                    },
                );
            }
        });
        let response = scope.response;
        if let Some(branch) = &branch {
            tree_item(ui, &response, branch, &key, &mut opened);
        }
        pass.events.extend(opened.into_iter().map(TableEvent::Open));
        if lead {
            pass.lead_row = Some((response.id, rect));
            FocusRing::paint_if(ui, rect, CornerRadius::ZERO, self.focused);
        }
        ui.ctx().accesskit_node_builder(response.id, |node| {
            // The row's `Ui` took its bounds before its cells were laid out.
            node.set_bounds(egui::accesskit::Rect { x0: rect.min.x.into(), y0: rect.min.y.into(), x1: rect.max.x.into(), y1: rect.max.y.into() });
            node.set_selected(selected);
            if let Some(tree) = tree {
                // A tree item needs a name; a plain row's is its cells'.
                node.set_label((tree.name)(&key));
            }
            node.set_row_index(at);
            node.add_action(Action::Focus);
            node.add_action(Action::ScrollIntoView);
        });
        let modifiers = ui.input(|i| i.modifiers);
        if response.is_pointer_button_down_on() || response.clicked_by(PointerButton::Primary) {
            pass.focus = true;
        }
        if response.double_clicked() {
            pass.events.push(TableEvent::Activate(key.clone()));
        } else if response.clicked()
            && let Some(change) = pass.rules.click(pass.selection, pass.rows, at, modifiers)
        {
            pass.apply(change);
        }
        if response.secondary_clicked() {
            pass.focus = true;
            let request =
                ContextMenuRequest { row: Some(key.clone()), selected, anchor: rect, pointer: response.interact_pointer_pos(), toggle: None };
            pass.events.push(TableEvent::ContextMenu(request));
        }
        if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Focus)) && pass.selection.lead.as_ref() != Some(&key) {
            pass.selection.lead = Some(key);
            pass.focus = true;
        }
    }
}

/// Where the row at `at` of a tree table sits, as [`List`](super::List)'s tree rows take it.
fn branch_at<K>(rows: &(impl DisplayOrder<K> + ?Sized), at: usize) -> Branch {
    Branch { depth: rows.depth(at), group: rows.has_children(at), open: rows.open(at), siblings: Some(tree::siblings(rows, at)) }
}

impl<C> Body<'_, C>
where
    C: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    /// The tree part of a tree table's first shown cell ([`Table::tree`]): the indent of
    /// `branch.steps()` levels of `list.indent` (shrunk so that `inner` keeps `list.min_label`),
    /// then the `list.disclosure` column, whose triangle `tree::show_disclosure` senses and paints
    /// for a group, named with `tree.name(key)` and `tree.text`; its `RowEvent::Open` reports are
    /// pushed to `opened` as `OpenChange`s. Mirrored in a right-to-left layout. Returns what is
    /// left of `inner` for the host's cell.
    fn tree_lead<K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
        &self,
        ui: &mut Ui,
        tree: &TreeRows<'_, K>,
        inner: Rect,
        branch: &Branch,
        key: &K,
        opened: &mut Vec<OpenChange<K>>,
    ) -> Rect {
        let list = ListRow::<'_, K>::style(&Tokens::of(ui));
        let column = if branch.group { list.disclosure } else { 0.0 };
        // The indent shrinks to keep the cell's least width; the triangle column doesn't.
        let indent = (branch.steps() as f32 * list.indent).min((inner.width() - column - list.min_label).max(0.0));
        let mut events = Vec::new();
        let rect = if self.rtl {
            let right = inner.right() - indent;
            if branch.group {
                let rect = Rect::from_min_max(pos2(right - column, inner.top()), pos2(right, inner.bottom()));
                tree::show_disclosure(ui, &list, rect, branch, &tree.text, ui.scope_id(), key, &(tree.name)(key), &mut events);
            }
            Rect::from_min_max(inner.min, pos2((right - column).max(inner.left()), inner.bottom()))
        } else {
            let left = inner.left() + indent;
            if branch.group {
                let rect = Rect::from_min_max(pos2(left, inner.top()), pos2(left + column, inner.bottom()));
                tree::show_disclosure(ui, &list, rect, branch, &tree.text, ui.scope_id(), key, &(tree.name)(key), &mut events);
            }
            Rect::from_min_max(pos2((left + column).min(inner.right()), inner.top()), inner.max)
        };
        opened.extend(events.into_iter().filter_map(|event| match event {
            RowEvent::Open(change) => Some(change),
            _ => None,
        }));
        rect
    }
}

/// A tree table row's AccessKit part: `level`, position among its siblings and the Expand or
/// Collapse action (`tree::report`), plus `expanded` for a group; a screen reader's Expand or
/// Collapse is pushed to `opened`.
fn tree_item<K: Clone>(ui: &Ui, response: &Response, branch: &Branch, key: &K, opened: &mut Vec<OpenChange<K>>) {
    let mut events = Vec::new();
    tree::report(ui, response, branch, key, &mut events);
    if branch.group {
        ui.ctx().accesskit_node_builder(response.id, |node| node.set_expanded(branch.open));
    }
    opened.extend(events.into_iter().filter_map(|event| match event {
        RowEvent::Open(change) => Some(change),
        _ => None,
    }));
}
