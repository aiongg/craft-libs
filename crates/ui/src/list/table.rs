#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`Table`]: rows of cells under a header of columns the user sorts, resizes, hides and moves.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;

use egui::{Align, AsIdSalt, Color32, IdSalt, Response, Ui};
use serde::{Deserialize, Serialize};

use super::view::ContextMenuRequest;
use crate::behavior::{DisplayOrder, Selection, SelectionChange, SelectionMode};
use crate::theme::UiTokens;

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
    /// column sorts ascending (EffectCraft's Project panel and PdfCraft's Combine Files table).
    // effectcraft: crates/ui-egui/src/panels/project.rs:559 (header click)
    pub fn clicked(current: Option<&Self>, column: C) -> Self {
        todo!()
    }

    /// Sorts `rows` by this sort: `compare(a, b, column)` orders two rows ascending, and the
    /// order reverses it. The sort is stable, so rows the column ties keep their order; a host
    /// that wants EffectCraft's fallback (ties by name) compares names in `compare`.
    // effectcraft: crates/ui-egui/src/panels/project.rs:86 sort_items
    pub fn apply<K>(&self, rows: &mut [K], compare: impl Fn(&K, &K, &C) -> Ordering) {
        todo!()
    }
}

/// One column of a [`Table`]: its key, its header and its rules.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn<C> {
    key: C,
    label: String,
    min_width: f32,
    initial_width: Option<f32>,
    hideable: bool,
    sortable: bool,
    align: Align,
}

impl<C> TableColumn<C> {
    /// A column with header `label` (as shown, translated); sortable, not hideable, left
    /// aligned, at least 40 points wide, sharing the free width until the user resizes it.
    pub fn new(key: C, label: impl Into<String>) -> Self {
        Self { key, label: label.into(), min_width: 40.0, initial_width: None, hideable: false, sortable: true, align: Align::Min }
    }

    /// The narrowest the user may drag the column (PdfCraft: File name 120, Pages 110, Size 60,
    /// Modified 80, Warnings 100).
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
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

    /// Numbers: right aligned (mirrored in a right-to-left interface), tabular figures.
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
    /// rest clamped to each column's least width and 4000 (PdfCraft's `Columns::from_json`).
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/files.rs:63 from_json
    pub fn repaired(self, columns: &[TableColumn<C>]) -> Self {
        todo!()
    }

    /// Whether `column` is shown.
    pub fn is_visible(&self, column: &C) -> bool {
        !self.hidden.contains(column)
    }

    /// Shows or hides `column`; its place in [`order`](Self::order) stays, so it comes back
    /// where it was (EffectCraft's `set_column` keeps the canonical order).
    // effectcraft: crates/ui-egui/src/panels/project.rs:1357 set_column
    pub fn set_visible(&mut self, column: &C, visible: bool) {
        todo!()
    }

    /// Moves `column` before `to` (after it with `after`), as a header dropped on another does.
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/table.rs:65 (HeadingEvent::Move)
    pub fn move_column(&mut self, column: &C, to: &C, after: bool) {
        todo!()
    }
}

/// The look of a table's header and rows, from [`Table::style`].
#[derive(Clone, Debug, PartialEq)]
pub struct TableStyle {
    /// Header height: the dock tab height (`tab.height`), so a table under a panel's tabs
    /// lines up with them; PdfCraft's is 26.
    pub header_height: f32,
    /// Row height: `list.height` (PdfCraft 22).
    pub row_height: f32,
    /// Between a cell's edge and its text: `list.pad_x` (PdfCraft 4).
    pub cell_pad_x: f32,
    /// The sort glyph's side (PdfCraft's chevron, 12) and its gap after the label (PdfCraft 14
    /// to its centre, so 8 to its edge).
    pub sort_glyph: f32,
    pub sort_gap: f32,
    /// The resize grip's hit width either side of a column edge.
    pub grip_width: f32,
    /// Header text: the `Label` role in `text.muted`, the sorting column's in `text.normal`
    /// (PdfCraft's).
    pub header_text: Color32,
    pub header_text_sorted: Color32,
    /// The header's hover and pressed fills: `list.hover` and `state.pressed`.
    pub header_hover: Color32,
    pub header_pressed: Color32,
    /// The drop line a dragged header shows, 2.5 wide in `accent.solid` (PdfCraft's).
    pub drop_line: Color32,
    pub drop_line_width: f32,
    /// Every other row's fill (the proposed `list.stripe`); none when the theme has none.
    pub stripe: Color32,
}

/// What the table reported this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TableEvent<K, C> {
    /// A header was clicked, or Space or Enter pressed on one: the layout's sort has changed to
    /// this; the host sorts its rows ([`TableSort::apply`]).
    Sort(TableSort<C>),
    /// The user resized (pointer or keys), moved, hid, showed or reset columns: the layout has
    /// changed; the host saves it.
    Layout,
    /// The selection changed, by the rules of [`ListSelection`](crate::behavior::ListSelection).
    Selection(SelectionChange<K>),
    /// Enter or a double click on a row.
    Activate(K),
    /// The row's context menu was asked for (secondary click, Menu key, Shift+F10).
    ContextMenu(ContextMenuRequest<K>),
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
///     .show(ui, |ui, file, column| match column {
///         Col::Name => { ui.label(&files[*file].name); }
///         Col::Pages => { ui.label(files[*file].pages.to_string()); }
///         _ => {}
///     });
/// for event in out.events {
///     match event {
///         TableEvent::Sort(sort) => sort.apply(&mut order, |a, b, col| compare(a, b, col)),
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
///   [`TableLabels::header_tip`]. A drag on a header moves the column: a 2.5-point accent line
///   marks the edge it would go to (the side of the target's centre the pointer is on), and the
///   release moves it. A drag on the edge between two headers resizes the column on its left
///   (on its right in a right-to-left interface) down to its least width; the cursor shows
///   `ResizeColumn` over the edge. Esc or focus loss during an edge drag puts back the width
///   from before the drag (the dock's resize rule). A double click on an edge fits the column
///   to its widest shown cell (no app does; Photoshop's Shortcuts table does).
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
///   cell in `cell(ui, key, column)`, in a `Ui` clipped to the cell, left-to-right centred
///   (right aligned for a [numeric](TableColumn::numeric) column). Only the rows in view are
///   built. Rows draw as [`ListRow`](super::ListRow)s do: the selection, hover and stripe fills
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

    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/table.rs:32 table, :96 heading, :167 table_body
    pub fn show(self, ui: &mut Ui, cell: impl FnMut(&mut Ui, &K, &C)) -> TableOutput<K, C> {
        todo!()
    }
}

impl Table<'static, (), (), [()]> {
    /// The table's look from the tokens.
    // new: the `table.*` tokens the spec proposes (header height from `tab.height`)
    pub fn style(tokens: &UiTokens) -> TableStyle {
        todo!()
    }
}
