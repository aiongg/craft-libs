//! `SheetGrid` and `SheetRow`: a sheet's cell grid on the collection contract, with typing into a cell on the
//! editor contract and the arrow split of 18.4.
//!
//! `SheetGrid` is a collection with `INPUT = Input::Editor`. Its `KEYS` keep no arrows, so with no cell open the
//! arrows are navigation: the collection's step 9 moves the lead cell. Typing opens the lead cell in Enter mode, where
//! the arrows stay navigation and moving to another cell commits the draft. F2 opens it in Edit mode, which claims
//! Left and Right to move the caret. A draft that starts with `=` and ends in an operator is in formula point mode,
//! which claims all four arrows and moves a reference cursor from `cx.take_claimed`.
//!
//! The grid does not hold the data: it reports the committed draft as a [`CellEdit`] and the app writes the cell.
//! The editor part sits over the lead cell at the position the grid computes from its own geometry, so the example
//! does not scroll horizontally and its vertical scroll would misplace the part: a collection's `parts` has no
//! access to the layout's offset.

use super::code_editor::{CodeDocument, caret_x};
use crate::contract::{
    Access, Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Focus as _, MeasureCx, Name,
    NoValue, Outcome, PaintUi, PartsCx, Remember as _, Role, SelectMode, Styled as _,
};
use crate::primitives::{EditorLine, EditorPurpose, EditorState, PaintShape, StrokeWidth, TextRun};
use crate::runtime::focus::{KeepKeys, KeyUse};
use crate::style::{ComponentStyle, Flags, Stateful, StyleColor, StyleCx, SurfaceLook};
use craft_behavior::{
    CellSelection, ColumnKey, DisplayOrder, EditorInput, Flow, Insets, ItemSource, Metrics, Mixed, MoveMods, NavMove, PartKey, SelectionModel as _,
    SheetLayout,
};
use craft_keymap::{Chord, Key};
use egui::{CornerRadius, Pos2, Rect, Response, Vec2};

const EDITOR: PartKey = PartKey::new("editor");
/// The column headers, left to right.
const NAMES: [&str; 4] = ["A", "B", "C", "D"];
/// A column is this many control heights wide.
const COLUMN_IN_HEIGHTS: f32 = 5.0;
/// The characters after which a formula expects a reference.
const OPERATORS: &str = "=+-*/(,";
/// The arrows with the row and column step each takes in a left-to-right sheet.
const ARROWS: [(Key, isize, isize); 4] = [(Key::Up, -1, 0), (Key::Down, 1, 0), (Key::Left, 0, -1), (Key::Right, 0, 1)];

/// A draft the grid committed: the cell it was for and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CellEdit {
    /// The row.
    pub row: usize,
    /// The column.
    pub column: usize,
    /// The text to write into the cell.
    pub text: String,
}

impl CellEdit {
    /// An edit writing `text` into cell (`row`, `column`).
    pub fn new(row: usize, column: usize, text: impl Into<String>) -> Self {
        CellEdit { row, column, text: text.into() }
    }
}

/// What the grid returns: the collection's response and the edit committed in this pass.
#[non_exhaustive]
pub struct SheetOutput {
    /// The collection's response and events.
    pub collection: CollectionResponse<usize, ()>,
    /// The draft committed in this pass, for the app to write.
    pub committed: Option<CellEdit>,
}

impl Outcome for SheetOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.collection.response
    }
}

/// The rows of a sheet as a collection source: `count` rows keyed by position.
#[non_exhaustive]
pub struct Rows(pub usize);

impl Rows {
    /// A source of `count` rows.
    pub const fn new(count: usize) -> Self {
        Rows(count)
    }
}

impl DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }
}

impl ItemSource<usize> for Rows {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Ready,
    Enter,
    Edit,
}

/// The reference cursor of formula point mode: the cell it names and where in the draft the reference starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point {
    at: (usize, usize),
    from: usize,
}

/// The cell being edited and its draft.
#[derive(Clone, Default)]
struct EditState {
    mode: Mode,
    cell: (usize, usize),
    draft: CodeDocument,
    point: Option<Point>,
}

impl EditState {
    /// Whether the draft is a formula waiting for a reference.
    fn pointing(&self) -> bool {
        let text = self.draft.text();
        self.mode != Mode::Ready && (self.point.is_some() || text.starts_with('=') && text.ends_with(|c: char| OPERATORS.contains(c)))
    }
}

/// The style of the grid: the field's look, the row and column sizes and the colours of the open cell.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SheetStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    row_height: f32,
    column_width: f32,
    pad: f32,
    edit_fill: StyleColor,
    edit_edge: StyleColor,
}

/// A sheet of cells with a lead cell, typed edits and formula pointing.
#[derive(crate::contract::Component)]
#[component(collection)]
pub struct SheetGrid<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    selection: Bind<'a, CellSelection<usize>>,
    rows: &'a [Vec<String>],
    committed: Option<CellEdit>,
}

impl<'a> SheetGrid<'a> {
    /// A grid over `rows` whose cell selection is `selection`.
    pub fn new(selection: Bind<'a, CellSelection<usize>>, rows: &'a [Vec<String>]) -> Self {
        SheetGrid {
            common: Common::new(),
            props: ContainerProps::new(),
            items: CollectionProps::new().select(SelectMode::Cells),
            selection,
            rows,
            committed: None,
        }
    }

    /// The row and column of the lead cell.
    fn lead_cell(&mut self) -> Option<(usize, usize)> {
        let Some(Mixed::Same(held)) = self.selection.reborrow().get() else { return None };
        let row = *held.rows.lead()?;
        let column = held.lead_column.as_ref().and_then(|key| NAMES.iter().position(|name| ColumnKey::new(name) == *key)).unwrap_or(0);
        Some((row, column))
    }

    fn cell_text(&self, (row, column): (usize, usize)) -> &str {
        self.rows.get(row).and_then(|cells| cells.get(column)).map_or("", String::as_str)
    }

    fn commit(&mut self, state: &mut EditState) {
        let (row, column) = state.cell;
        self.committed = Some(CellEdit { row, column, text: state.draft.text() });
        state.mode = Mode::Ready;
        state.point = None;
    }
}

/// The rect of cell `(row, column)` in a grid whose viewport is `area`.
fn cell_rect(area: Rect, style: &SheetStyle, rtl: bool, (row, column): (usize, usize)) -> Rect {
    let x = Flow::new(area, rtl).span(column as f32 * style.column_width, style.column_width);
    Rect::from_min_size(Pos2::new(x.left(), area.top() + row as f32 * style.row_height), Vec2::new(style.column_width, style.row_height))
}

/// The name of a cell as a formula writes it: `B3`.
fn reference((row, column): (usize, usize)) -> String {
    format!("{}{}", NAMES[column], row + 1)
}

impl Component for SheetGrid<'_> {
    const ROLE: Role = Role::TABLE;
    const INPUT: crate::contract::Input = crate::contract::Input::Editor;
    const KEYS: KeyUse = KeyUse::of(crate::contract::Input::Editor).with_keep(KeepKeys::None);
    type Style = SheetStyle;
    type Output = SheetOutput;

    fn style(cx: &StyleCx<'_>, _: SheetGridStyleKey) -> SheetStyle {
        let control = cx.control();
        SheetStyle {
            surface: cx.field(),
            metrics: Metrics::new(),
            row_height: control.height,
            column_width: control.height * COLUMN_IN_HEIGHTS,
            pad: control.pad_x,
            edit_fill: cx.color(cx.tokens.color.surface.raised),
            edit_edge: cx.color(cx.tokens.color.line.focus),
        }
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let cell = self.lead_cell().unwrap_or((0, 0));
        cx.editor(EDITOR, cell_rect(rect, cx.style, cx.flow.is_rtl(), cell), EditorPurpose::Text);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let lead = self.lead_cell();
        let mut state = cx.memory::<EditState>().clone();
        let escape = cx.take_claimed(Chord::plain(Key::Escape));
        let enter = cx.take_claimed(Chord::plain(Key::Enter));
        let f2 = cx.take_claimed(Chord::plain(Key::F2));
        let arrows = ARROWS.map(|(key, ..)| cx.take_claimed(Chord::plain(key)));
        if state.mode != Mode::Ready && lead.is_some_and(|lead| lead != state.cell) {
            self.commit(&mut state);
        }
        if state.mode != Mode::Ready && escape {
            state.mode = Mode::Ready;
            state.point = None;
        } else if state.mode != Mode::Ready && enter {
            self.commit(&mut state);
        } else if state.mode == Mode::Ready
            && f2
            && let Some(cell) = lead
        {
            state.cell = cell;
            state.draft.set_text(self.cell_text(cell));
            state.mode = Mode::Edit;
        }
        if state.pointing() {
            let rtl = cx.flow.is_rtl();
            for ((_, rows, columns), taken) in ARROWS.iter().zip(arrows) {
                if !taken {
                    continue;
                }
                let columns = if rtl { -columns } else { *columns };
                let (at, from) = state.point.map_or((state.cell, state.draft.caret().byte), |point| (point.at, point.from));
                let to = (
                    at.0.saturating_add_signed(*rows).min(self.rows.len().saturating_sub(1)),
                    at.1.saturating_add_signed(columns).min(NAMES.len() - 1),
                );
                let text = format!("{}{}", &state.draft.text()[..from], reference(to));
                state.draft.set_text(&text);
                state.point = Some(Point { at: to, from });
            }
        } else if state.mode == Mode::Edit {
            // The claimed Left and Right move the caret; the document reads left to right.
            for (at, mv) in [(2, NavMove::Back), (3, NavMove::Forward)] {
                if arrows[at] {
                    state.draft.apply(EditorInput::Move(mv, MoveMods::PLAIN));
                }
            }
        }
        let pointing = state.pointing();
        for input in cx.editor_input().collect::<Vec<_>>() {
            match (state.mode, input) {
                (Mode::Ready, EditorInput::Text(text)) => {
                    if let Some(cell) = lead {
                        state.cell = cell;
                        state.draft.set_text("");
                        state.mode = Mode::Enter;
                        state.draft.apply(EditorInput::Text(text));
                    }
                }
                (Mode::Ready, _) | (_, EditorInput::Newline | EditorInput::Tab) => {}
                (_, EditorInput::Move(..)) if pointing => {}
                (_, input) => {
                    state.point = None;
                    state.draft.apply(input);
                }
            }
        }
        cx.claim(Chord::plain(Key::F2));
        if state.mode != Mode::Ready {
            cx.claim(Chord::plain(Key::Enter));
            cx.claim(Chord::plain(Key::Escape));
        }
        let claimed: &[usize] = if state.pointing() {
            &[0, 1, 2, 3]
        } else if state.mode == Mode::Edit {
            &[2, 3]
        } else {
            &[]
        };
        for at in claimed {
            cx.claim(Chord::plain(ARROWS[*at].0));
        }
        *cx.memory::<EditState>() = state;
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let state = cx.recall::<EditState>();
        let rtl = cx.env.direction().is_rtl();
        let draw = ui.draw();
        let lead = self.lead_cell().unwrap_or((0, 0));
        if state.mode == Mode::Ready {
            let cell = cell_rect(cx.content_rect, cx.style, rtl, lead);
            cx.editor_state(EditorState::new(
                Rect::from_min_size(cell.min, Vec2::new(0.0, cell.height())),
                &[],
                craft_behavior::TextPos::new(0, 0),
                0,
            ));
            return;
        }
        let cell = cell_rect(cx.content_rect, cx.style, rtl, state.cell);
        draw.fill(&PaintShape::Rect(cell, CornerRadius::ZERO), cx.style.edit_fill);
        draw.stroke(&PaintShape::Rect(cell, CornerRadius::ZERO), StrokeWidth::Medium, cx.style.edit_edge);
        let inner = Flow::new(cell, rtl).inset(Insets::horizontal(cx.style.pad));
        let text = state.draft.shown(0).into_owned();
        let layout = draw.layout(&TextRun::new(text.clone()), inner.width());
        let line = Rect::from_min_size(inner.min, Vec2::new(inner.width(), layout.size().y));
        draw.galley(line.min, &layout);
        let runs = layout.editor_runs();
        let at = state.draft.caret_shown();
        let x = caret_x(&runs, &text, at.byte);
        let top = line.min + Vec2::new(x, 0.0);
        if cx.is_focused() {
            draw.stroke(&PaintShape::Path(vec![top, top + Vec2::new(0.0, line.height())]), StrokeWidth::Medium, cx.style.edit_edge);
        }
        let lines = [EditorLine::new(&text, line, line.top() + layout.first_baseline, &runs)];
        let mut editor = EditorState::new(Rect::from_min_size(top, Vec2::new(0.0, line.height())), &lines, at, state.draft.revision());
        editor.selection = state.draft.selection();
        cx.editor_state(editor);
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> SheetOutput {
        SheetOutput { collection: cx.collection_response(), committed: self.committed }
    }
}

impl Collection for SheetGrid<'_> {
    type Key = usize;
    type ItemLayout = SheetLayout;
    type Selection = CellSelection<usize>;
    type Item<'r> = SheetRow<'r>;
    type ItemEvent = ();

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, CellSelection<usize>>> {
        Some(self.selection.reborrow())
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, _: f32, cx: &StyleCx<'_>) -> SheetLayout {
        let control = cx.control();
        let width = control.height * COLUMN_IN_HEIGHTS;
        SheetLayout::new(source, control.height, NAMES.iter().map(|name| (ColumnKey::new(name), width)).collect())
    }
}

/// The style of a sheet row: the fill and edge of a cell by the flags the item step writes into its cell part.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SheetRowStyle {
    #[style(metrics)]
    metrics: Metrics,
    fill: Stateful<StyleColor>,
    edge: Stateful<StyleColor>,
    pad: f32,
}

/// One row of a sheet: the text of each cell, with a cell part per column.
#[derive(crate::contract::Component)]
pub struct SheetRow<'r> {
    #[common]
    common: Common,
    cells: Vec<(ColumnKey, Rect)>,
    values: &'r [String],
}

impl<'r> SheetRow<'r> {
    /// A row showing `values` in `cells`, the rects the item context lists.
    pub fn new(values: &'r [String], cells: Vec<(ColumnKey, Rect)>) -> Self {
        SheetRow { common: Common::new(), cells, values }
    }
}

impl Component for SheetRow<'_> {
    const ROLE: Role = Role::ROW;
    type Style = SheetRowStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: SheetRowStyleKey) -> SheetRowStyle {
        let clear = cx.color(craft_theme::Rgba::TRANSPARENT);
        SheetRowStyle {
            metrics: Metrics::new(),
            fill: Stateful::new(clear).when(Flags::SELECTED, cx.color(cx.tokens.color.state.text_selection)),
            edge: Stateful::new(clear).when(Flags::LEAD, cx.color(cx.tokens.color.line.focus)),
            pad: cx.control().pad_x,
        }
    }

    /// The row as the item step laid its cells out.
    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        self.cells.iter().fold(Rect::NOTHING, |all, (_, rect)| all.union(*rect)).size().max(Vec2::ZERO)
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        for (at, (_, rect)) in self.cells.iter().enumerate() {
            cx.part(PartKey::CELL.at(at as u64), *rect);
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        for at in 0..self.cells.len() {
            access.node(PartKey::CELL.at(at as u64), Role::CELL).name = Name::Text(self.values.get(at).cloned().unwrap_or_default());
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rtl = cx.env.direction().is_rtl();
        let draw = ui.draw();
        for (at, (_, rect)) in self.cells.iter().enumerate() {
            let part = PartKey::CELL.at(at as u64);
            draw.fill(&PaintShape::Rect(*rect, CornerRadius::ZERO), cx.resolve_part(part, &cx.style.fill));
            draw.stroke(&PaintShape::Rect(*rect, CornerRadius::ZERO), StrokeWidth::Medium, cx.resolve_part(part, &cx.style.edge));
            let inner = Flow::new(*rect, rtl).inset(Insets::horizontal(cx.style.pad));
            draw.text(inner, &TextRun::new(self.values.get(at).map_or("", String::as_str)).elide());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
