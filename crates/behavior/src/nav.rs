//! The navigation core (spec 9.5): shapes, the key-free outline, stepping, roving groups, type-ahead and
//! single-value keys. Focus movement and collection selection share it.

use crate::intent::{MoveMods, NavMove};
use crate::wheel::Wheel;
use emath::Rect;

/// The geometry a collection's keyboard navigation follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NavShape {
    /// One row after another.
    Flat,
    /// Rows that expand and collapse.
    Tree,
    /// Equal-width cells that wrap at `columns`.
    #[non_exhaustive]
    WrappedGrid {
        /// Cells per row.
        columns: usize,
    },
    /// Equal-width cells in `columns` columns whose Left and Right stop at the row ends; Up and Down stop at the first
    /// and last rows.
    #[non_exhaustive]
    BoundedGrid {
        /// Cells per row.
        columns: usize,
    },
    /// Rows of different lengths.
    RaggedGrid,
    /// Cells of a sheet.
    Cell,
    /// Items with free positions; arrows go to the nearest item in that direction.
    Spatial,
}

impl NavShape {
    /// Equal-width cells that wrap at `columns`.
    pub const fn wrapped_grid(columns: usize) -> Self {
        Self::WrappedGrid { columns }
    }

    /// Equal-width cells in `columns` columns whose Left and Right stop at the row ends.
    pub const fn bounded_grid(columns: usize) -> Self {
        Self::BoundedGrid { columns }
    }
}

/// Positions, not keys: the key-free part of `DisplayOrder`.
pub trait NavOutline {
    /// The number of positions.
    fn len(&self) -> usize;
    /// The nesting depth of the position, 0 for a root.
    fn depth(&self, at: usize) -> usize;
    /// Whether the position has children.
    fn has_children(&self, at: usize) -> bool;
    /// Whether the position's children show.
    fn open(&self, at: usize) -> bool;
    /// Whether the position is a section header, which a move skips.
    fn is_section(&self, at: usize) -> bool;
    /// Whether the position can take the lead.
    fn enabled(&self, at: usize) -> bool;

    /// `RaggedGrid`: the number of positions in `row`; rows hold consecutive positions. `None` past the last row.
    fn row_len(&self, _row: usize) -> Option<usize> {
        None
    }

    /// `Cell`: the cells of every position (a row of a sheet).
    fn columns(&self) -> usize {
        1
    }

    /// `WrappedGrid`: the column of position 0 when the grid has blank cells before it (the leading days of a
    /// calendar month), `None` for a grid that fills from its first column. A grid with a first column keeps its rows
    /// whole: Home and End go to the ends of the lead's row.
    fn first_column(&self) -> Option<usize> {
        None
    }

    /// `Spatial`: where the position sits, in any one coordinate space.
    fn rect(&self, _at: usize) -> Option<Rect> {
        None
    }

    /// Whether there are no positions.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Where a navigation stands between moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct NavState {
    /// The lead position, `None` before the first move.
    pub lead: Option<usize>,
    /// The fixed end of an extended selection.
    pub anchor: Option<usize>,
    /// Positions a page move covers.
    pub page: usize,
    /// Whether a move past an end continues at the other end.
    pub wrap: bool,
    /// `Cell`: the column of the lead cell.
    pub column: usize,
}

impl NavState {
    /// A state with no lead, a page of ten and no wrap.
    pub const fn new() -> Self {
        NavState { lead: None, anchor: None, page: 10, wrap: false, column: 0 }
    }

    /// A state at `lead`.
    pub const fn at(lead: usize) -> Self {
        NavState { lead: Some(lead), anchor: None, page: 10, wrap: false, column: 0 }
    }

    /// Sets the page size.
    pub const fn page(mut self, rows: usize) -> Self {
        self.page = if rows == 0 { 1 } else { rows };
        self
    }

    /// Sets whether moves wrap.
    pub const fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Sets the lead cell's column.
    pub const fn column(mut self, column: usize) -> Self {
        self.column = column;
        self
    }
}

impl Default for NavState {
    fn default() -> Self {
        NavState::new()
    }
}

/// What one move did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NavStep {
    /// Nothing changed.
    Stay,
    /// The lead moved to this position; `state` holds the new lead and anchor.
    Moved(usize),
    /// A tree row opens.
    Expand(usize),
    /// A tree row closes.
    Collapse(usize),
    /// The lead cell moved to this column of the same row; `state.column` holds it.
    Column(usize),
    /// The shape has no stepping rule for this move; the caller handles it.
    Unhandled,
}

/// Steps the lead of a navigation of `shape` over `outline` by `mv`. `mods.extend` keeps the anchor at the
/// old lead; any other move clears it. `RaggedGrid` reads `row_len`, `Cell` reads `columns` and `Spatial` reads
/// `rect` from the outline.
pub fn step(shape: NavShape, outline: &dyn NavOutline, state: &mut NavState, mv: NavMove, mods: MoveMods) -> NavStep {
    let len = outline.len();
    if len == 0 {
        return NavStep::Stay;
    }
    let Some(lead) = state.lead else {
        return first_move(outline, state, mv, mods);
    };
    let lead = lead.min(len - 1);
    if let Some(step) = header_rule(shape, outline, lead, mv) {
        return step;
    }
    let result = match shape {
        NavShape::Flat => linear(outline, state, lead, mv),
        NavShape::Tree => tree(outline, state, lead, mv),
        NavShape::WrappedGrid { columns } => grid(outline, state, lead, mv, mods, columns.max(1), true),
        NavShape::BoundedGrid { columns } => grid(outline, state, lead, mv, mods, columns.max(1), false),
        NavShape::RaggedGrid => ragged(outline, state, lead, mv),
        NavShape::Cell => cell(outline, state, lead, mv, mods),
        NavShape::Spatial => spatial(outline, lead, mv),
    };
    if let NavStep::Column(_) = result {
        state.anchor = if mods.extend { state.anchor.or(Some(lead)) } else { None };
    }
    if let NavStep::Moved(to) = result {
        state.anchor = if mods.extend { state.anchor.or(Some(lead)) } else { None };
        state.lead = Some(to);
        if to == lead {
            return NavStep::Stay;
        }
    }
    result
}

/// A lead on a group header (a section with children, which moves skip but a press can reach): Forward opens it and
/// Back closes it, the tree rule in every shape that reads Left and Right as steps.
fn header_rule(shape: NavShape, outline: &dyn NavOutline, lead: usize, mv: NavMove) -> Option<NavStep> {
    if matches!(shape, NavShape::Cell | NavShape::Spatial) || !outline.is_section(lead) || !outline.has_children(lead) {
        return None;
    }
    match mv {
        NavMove::Forward if !outline.open(lead) => Some(NavStep::Expand(lead)),
        NavMove::Back if outline.open(lead) => Some(NavStep::Collapse(lead)),
        _ => None,
    }
}

/// The first move from no lead: forward moves land on the first position, backward moves on the last.
fn first_move(outline: &dyn NavOutline, state: &mut NavState, mv: NavMove, mods: MoveMods) -> NavStep {
    let backward = matches!(mv, NavMove::Back | NavMove::Up | NavMove::PageUp | NavMove::End);
    let to = if backward { last_enabled(outline) } else { first_enabled(outline) };
    match to {
        Some(at) => {
            state.lead = Some(at);
            state.anchor = mods.extend.then_some(at);
            NavStep::Moved(at)
        }
        None => NavStep::Stay,
    }
}

fn selectable(outline: &dyn NavOutline, at: usize) -> bool {
    outline.enabled(at) && !outline.is_section(at)
}

fn first_enabled(outline: &dyn NavOutline) -> Option<usize> {
    (0..outline.len()).find(|&at| selectable(outline, at))
}

fn last_enabled(outline: &dyn NavOutline) -> Option<usize> {
    (0..outline.len()).rev().find(|&at| selectable(outline, at))
}

/// The next selectable position from `from` in `direction` (+1 or -1), wrapping when the state says so.
fn neighbour(outline: &dyn NavOutline, state: &NavState, from: usize, forward: bool) -> Option<usize> {
    let len = outline.len();
    let mut at = from;
    for _ in 0..len {
        at = match (forward, at) {
            (true, at) if at + 1 < len => at + 1,
            (true, _) if state.wrap => 0,
            (false, at) if at > 0 => at - 1,
            (false, _) if state.wrap => len - 1,
            _ => return None,
        };
        if selectable(outline, at) {
            return Some(at);
        }
    }
    None
}

/// `count` selectable positions from `from`, stopping at the last one reached. A page move that starts on the last
/// position of its direction wraps like a single step does, when the state wraps.
fn advance(outline: &dyn NavOutline, state: &NavState, from: usize, forward: bool, count: usize) -> usize {
    let stop = NavState { wrap: false, ..*state };
    let mut at = from;
    for _ in 0..count {
        match neighbour(outline, &stop, at, forward) {
            Some(next) => at = next,
            None => break,
        }
    }
    if at == from && state.wrap {
        return neighbour(outline, state, from, forward).unwrap_or(from);
    }
    at
}

fn moved(to: Option<usize>) -> NavStep {
    to.map_or(NavStep::Stay, NavStep::Moved)
}

fn linear(outline: &dyn NavOutline, state: &NavState, lead: usize, mv: NavMove) -> NavStep {
    match mv {
        NavMove::Back | NavMove::Up => moved(neighbour(outline, state, lead, false)),
        NavMove::Forward | NavMove::Down => moved(neighbour(outline, state, lead, true)),
        NavMove::PageUp => NavStep::Moved(advance(outline, state, lead, false, state.page)),
        NavMove::PageDown => NavStep::Moved(advance(outline, state, lead, true, state.page)),
        NavMove::Start => moved(first_enabled(outline)),
        NavMove::End => moved(last_enabled(outline)),
    }
}

fn tree(outline: &dyn NavOutline, state: &NavState, lead: usize, mv: NavMove) -> NavStep {
    let expandable = outline.has_children(lead);
    match mv {
        NavMove::Forward if expandable && !outline.open(lead) => NavStep::Expand(lead),
        NavMove::Forward if expandable => moved(neighbour(outline, state, lead, true)),
        NavMove::Forward => NavStep::Stay,
        NavMove::Back if expandable && outline.open(lead) => NavStep::Collapse(lead),
        NavMove::Back => {
            let depth = outline.depth(lead);
            let parent = (0..lead).rev().find(|&at| outline.depth(at) + 1 == depth && selectable(outline, at));
            if depth == 0 { NavStep::Stay } else { moved(parent) }
        }
        _ => linear(outline, state, lead, mv),
    }
}

/// The row and column of position `at` in a ragged grid, with the first position of its row.
fn locate(outline: &dyn NavOutline, at: usize) -> Option<(usize, usize, usize)> {
    let mut start = 0;
    for row in 0.. {
        let len = outline.row_len(row)?;
        if at < start + len {
            return Some((row, at - start, start));
        }
        start += len;
    }
    None
}

/// The position at `column` of `row`, clamped to the row's last position.
fn ragged_at(outline: &dyn NavOutline, row: usize, column: usize) -> Option<usize> {
    let start: usize = (0..row).map(|earlier| outline.row_len(earlier).unwrap_or(0)).sum();
    let len = outline.row_len(row).filter(|&len| len > 0)?;
    let at = start + column.min(len - 1);
    selectable(outline, at).then_some(at)
}

fn ragged(outline: &dyn NavOutline, state: &NavState, lead: usize, mv: NavMove) -> NavStep {
    let Some((row, column, _)) = locate(outline, lead) else { return NavStep::Unhandled };
    let rows = (0..).take_while(|&r| outline.row_len(r).is_some()).count();
    // A row without a position that takes the lead (a closed group's) is passed over in the move's direction.
    let toward = |rows: Vec<usize>| moved(rows.into_iter().find_map(|row| ragged_at(outline, row, column)));
    let vertical = |to: Option<usize>| moved(to.and_then(|row| ragged_at(outline, row, column)));
    match mv {
        NavMove::Back => moved(neighbour(outline, state, lead, false)),
        NavMove::Forward => moved(neighbour(outline, state, lead, true)),
        NavMove::Up => toward((0..row).rev().collect()),
        NavMove::Down => toward((row + 1..rows).collect()),
        NavMove::PageUp => vertical(Some(row.saturating_sub(state.page))),
        NavMove::PageDown => vertical(Some((row + state.page).min(rows - 1))),
        NavMove::Start => moved(first_enabled(outline)),
        NavMove::End => moved(last_enabled(outline)),
    }
}

/// A sheet: positions are rows and `state.column` is the lead cell's column. Home and End go to the row's ends,
/// with Ctrl (`mods.lead`) to the sheet's first and last row.
fn cell(outline: &dyn NavOutline, state: &mut NavState, lead: usize, mv: NavMove, mods: MoveMods) -> NavStep {
    let last_column = outline.columns().saturating_sub(1);
    let column = state.column.min(last_column);
    let across = |state: &mut NavState, to: usize| {
        state.column = to;
        if to == column { NavStep::Stay } else { NavStep::Column(to) }
    };
    match mv {
        NavMove::Back => across(state, column.saturating_sub(1)),
        NavMove::Forward => across(state, (column + 1).min(last_column)),
        NavMove::Start if mods.lead => moved(first_enabled(outline)),
        NavMove::End if mods.lead => moved(last_enabled(outline)),
        NavMove::Start => across(state, 0),
        NavMove::End => across(state, last_column),
        NavMove::Up | NavMove::Down | NavMove::PageUp | NavMove::PageDown => linear(outline, state, lead, mv),
    }
}

type Axis = fn(emath::Vec2) -> f32;

/// Items at free positions: the nearest item whose centre lies beyond the lead's in the move's direction, with
/// the off-axis distance counting double. Back and Forward are toward lower and higher x.
fn spatial(outline: &dyn NavOutline, lead: usize, mv: NavMove) -> NavStep {
    let Some(from) = outline.rect(lead).map(|rect| rect.center()) else { return NavStep::Unhandled };
    let (along, across): (Axis, Axis) = match mv {
        NavMove::Back => (|d| -d.x, |d| d.y),
        NavMove::Forward => (|d| d.x, |d| d.y),
        NavMove::Up | NavMove::PageUp => (|d| -d.y, |d| d.x),
        NavMove::Down | NavMove::PageDown => (|d| d.y, |d| d.x),
        NavMove::Start => return moved(first_enabled(outline)),
        NavMove::End => return moved(last_enabled(outline)),
    };
    let nearest = (0..outline.len())
        .filter(|&at| at != lead && selectable(outline, at))
        .filter_map(|at| outline.rect(at).map(|rect| (at, rect.center() - from)))
        .filter(|(_, delta)| along(*delta) > 0.0)
        .min_by(|(_, a), (_, b)| (along(*a) + 2.0 * across(*a).abs()).total_cmp(&(along(*b) + 2.0 * across(*b).abs())));
    moved(nearest.map(|(at, _)| at))
}

fn grid(outline: &dyn NavOutline, state: &NavState, lead: usize, mv: NavMove, mods: MoveMods, columns: usize, wraps: bool) -> NavStep {
    let len = outline.len();
    let blanks = outline.first_column().map_or(0, |column| column % columns);
    // Members sit after `blanks` blank cells of the first row.
    let column = lead.saturating_add(blanks) % columns;
    let vertical = |down: bool, rows: usize| {
        let delta = columns * rows;
        let to = if down { lead.checked_add(delta).filter(|&at| at < len) } else { lead.checked_sub(delta) };
        // A step past the last row's end lands on the last position of a shorter row only for pages.
        to.filter(|&at| selectable(outline, at))
    };
    let row_start = (lead + blanks) / columns * columns;
    let beside = |forward: bool| {
        let at_row_end = if forward { column + 1 == columns } else { column == 0 };
        if !wraps && at_row_end { None } else { neighbour(outline, state, lead, forward) }
    };
    match mv {
        NavMove::Back => moved(beside(false)),
        NavMove::Forward => moved(beside(true)),
        NavMove::Up => moved(vertical(false, 1)),
        NavMove::Down => moved(vertical(true, 1)),
        NavMove::PageUp => {
            let top = if column >= blanks { column - blanks } else { column + columns - blanks };
            moved(vertical(false, state.page).or_else(|| Some(top).filter(|&at| at < len && selectable(outline, at))))
        }
        NavMove::PageDown => {
            let last = len - 1 + blanks;
            let at = ((last / columns * columns + column).min(last)).saturating_sub(blanks);
            moved(vertical(true, state.page).or_else(|| Some(at).filter(|&at| selectable(outline, at))))
        }
        NavMove::Start if outline.first_column().is_some() && !mods.lead => {
            moved(Some(row_start.saturating_sub(blanks)).filter(|&at| selectable(outline, at)))
        }
        NavMove::End if outline.first_column().is_some() && !mods.lead => {
            let end = (row_start + columns - 1).min(len - 1 + blanks) - blanks;
            moved(Some(end).filter(|&at| selectable(outline, at)))
        }
        NavMove::Start => moved(first_enabled(outline)),
        NavMove::End => moved(last_enabled(outline)),
    }
}

/// Whether a roving group matches typed letters to entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Letters {
    /// Letters do nothing.
    #[default]
    None,
    /// Letters build a prefix that picks the next matching entry.
    TypeAhead,
}

/// A roving group's rules: one Tab stop whose members take moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Roving {
    /// The navigation shape.
    pub shape: NavShape,
    /// Whether a move past an end continues at the other end.
    pub wrap: bool,
    /// Whether Home and End move to the ends.
    pub home_end: bool,
    /// Rows a Page key covers, `None` when Page keys are not the group's.
    pub page: Option<usize>,
    /// Whether letters pick entries.
    pub letters: Letters,
    /// The arrows the group takes; the others reach the focused member.
    pub arrows: Arrows,
    /// `WrappedGrid`: the column of the first member when blank cells come before it, `None` when members fill the
    /// rows from the first column.
    pub first_column: Option<usize>,
    /// Whether a move the group cannot make (a step past its first or last row, say) reaches the owning component
    /// instead of being dropped.
    pub hands_back: bool,
    /// Whether the focused member is no lead, so the next move is a first move: forward moves land on the first
    /// enabled member, backward moves on the last.
    pub no_lead: bool,
}

/// Which arrow keys a roving group moves its lead with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Arrows {
    /// Every arrow.
    #[default]
    All,
    /// Left and Right only: Up and Down reach the focused member, as a menu bar title opens its menu with Down.
    Horizontal,
    /// Up and Down only: Left and Right reach the focused member, as a menu moves to its neighbour with them.
    Vertical,
}

impl Roving {
    /// A group of `shape` with Home and End, no wrap, no paging and no letters.
    pub const fn new(shape: NavShape) -> Roving {
        Roving {
            shape,
            wrap: false,
            home_end: true,
            page: None,
            letters: Letters::None,
            arrows: Arrows::All,
            first_column: None,
            hands_back: false,
            no_lead: false,
        }
    }

    /// Whether moves wrap.
    pub const fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Whether Home and End move to the ends.
    pub const fn home_end(mut self, on: bool) -> Self {
        self.home_end = on;
        self
    }

    /// Rows a Page key covers.
    pub const fn page(mut self, rows: Option<usize>) -> Self {
        self.page = rows;
        self
    }

    /// Whether letters pick entries.
    pub const fn letters(mut self, letters: Letters) -> Self {
        self.letters = letters;
        self
    }

    /// The arrows the group takes.
    pub const fn arrows(mut self, arrows: Arrows) -> Self {
        self.arrows = arrows;
        self
    }

    /// The column of the first member of a wrapped grid with `column` blank cells before it: Up and Down keep the
    /// column, Home and End go to the ends of the row.
    pub const fn first_column(mut self, column: usize) -> Self {
        self.first_column = Some(column);
        self
    }

    /// A move the group cannot make reaches the owning component, which may page or change what the group shows.
    pub const fn hands_back(mut self) -> Self {
        self.hands_back = true;
        self
    }

    /// Whether the focused member is no lead: the next move starts at an end, as from a group with no lead.
    pub const fn no_lead(mut self, on: bool) -> Self {
        self.no_lead = on;
        self
    }

    /// The state a step over this group starts from, at `lead`, or from no lead when the group has none.
    pub const fn state(&self, lead: Option<usize>) -> NavState {
        let lead = if self.no_lead { None } else { lead };
        let page = match self.page {
            Some(rows) if rows > 0 => rows,
            _ => 10,
        };
        NavState { lead, anchor: None, page, wrap: self.wrap, column: 0 }
    }

    /// Whether the group handles `mv`: Page moves only with a page size, Start and End only with Home and End.
    pub const fn takes(&self, mv: NavMove) -> bool {
        match mv {
            NavMove::PageUp | NavMove::PageDown => self.page.is_some(),
            NavMove::Start | NavMove::End => self.home_end,
            NavMove::Back | NavMove::Forward => !matches!(self.arrows, Arrows::Vertical),
            NavMove::Up | NavMove::Down => !matches!(self.arrows, Arrows::Horizontal),
        }
    }
}

/// Typed letters of the last second as one prefix.
#[derive(Clone, Debug, Default)]
pub struct TypeAhead {
    buffer: String,
    last: f64,
}

impl TypeAhead {
    /// The seconds after which the next letter starts a new prefix.
    pub const RESET_AFTER: f64 = 1.0;

    /// An empty buffer.
    pub const fn new() -> Self {
        TypeAhead { buffer: String::new(), last: f64::NEG_INFINITY }
    }

    /// Adds `letter` typed at `now` seconds and returns the prefix.
    pub fn push(&mut self, letter: char, now: f64) -> &str {
        if now - self.last > Self::RESET_AFTER {
            self.buffer.clear();
        }
        self.last = now;
        self.buffer.extend(letter.to_lowercase());
        &self.buffer
    }

    /// The entry a prefix picks: the first match from the position after `from` (from `from` itself when the
    /// prefix is one repeated letter, so repeating a letter cycles), wrapping at the end.
    pub fn pick<'a>(prefix: &str, from: Option<usize>, len: usize, label: impl Fn(usize) -> std::borrow::Cow<'a, str>) -> Option<usize> {
        if len == 0 || prefix.is_empty() {
            return None;
        }
        let mut chars = prefix.chars();
        let first = chars.next();
        let repeated = first.is_some_and(|c| prefix.chars().all(|p| p == c)) && prefix.chars().count() > 1;
        let needle: String = if repeated { first.into_iter().collect() } else { prefix.to_owned() };
        let start = match from {
            Some(at) if repeated || prefix.chars().count() == 1 => at + 1,
            Some(at) => at,
            None => 0,
        };
        (0..len).map(|offset| (start + offset) % len).find(|&at| label(at).to_lowercase().starts_with(&needle))
    }
}

/// The axes a single-value control reads moves on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueAxes {
    /// Left and right arrows.
    Horizontal,
    /// Up and down arrows.
    Vertical,
    /// Both pairs.
    Both,
}

/// How many small steps one large step is, when a control sets no large step of its own.
pub const LARGE_STEPS: f64 = 10.0;

/// The share of a value's span that one small step is, when a control sets no step.
pub const STEP_SHARE: f64 = 0.01;

/// The value and steps of a single-value control.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ValueSteps {
    /// The current value.
    pub value: f64,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
    /// One arrow press.
    pub small: f64,
    /// One Page key, or an arrow with the extend modifier.
    pub large: f64,
    /// An arrow with Alt.
    pub fine: f64,
    /// One wheel notch.
    pub notch: f64,
}

impl ValueSteps {
    /// Steps over `min..=max` from `value`. The fine step is `small / LARGE_STEPS` and a wheel notch is `small`.
    pub const fn new(value: f64, min: f64, max: f64, small: f64, large: f64) -> Self {
        ValueSteps { value, min, max, small, large, fine: small / LARGE_STEPS, notch: small }
    }

    /// The same steps with an Alt arrow of `fine`.
    pub const fn fine(mut self, fine: f64) -> Self {
        self.fine = fine;
        self
    }

    /// The same steps with a wheel notch of `notch`.
    pub const fn notch(mut self, notch: f64) -> Self {
        self.notch = notch;
        self
    }
}

/// What a single-value move does.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum StepResult {
    /// The control takes this value.
    Set(f64),
    /// The move is not a value move on these axes.
    Ignored,
}

/// Maps a move to the new value of a slider, dial, rating, resize handle or colour pad. `Forward`, `Up` and
/// `PageUp` increase; `Start` and `End` go to the ends. An arrow steps by `small`; with Shift (`extend`) by `large`,
/// with Alt (`fine`) by `fine`, and with both by `small`.
pub fn step_value(steps: &ValueSteps, axes: ValueAxes, mv: NavMove, mods: MoveMods) -> StepResult {
    let horizontal = !matches!(axes, ValueAxes::Vertical);
    let vertical = !matches!(axes, ValueAxes::Horizontal);
    let small = match (mods.extend, mods.fine) {
        (true, false) => steps.large,
        (false, true) => steps.fine,
        _ => steps.small,
    };
    let delta = match mv {
        NavMove::Forward if horizontal => small,
        NavMove::Back if horizontal => -small,
        NavMove::Up if vertical => small,
        NavMove::Down if vertical => -small,
        NavMove::PageUp => steps.large,
        NavMove::PageDown => -steps.large,
        NavMove::Start => return StepResult::Set(steps.min),
        NavMove::End => return StepResult::Set(steps.max),
        _ => return StepResult::Ignored,
    };
    StepResult::Set((steps.value + delta).clamp(steps.min, steps.max))
}

/// Maps a wheel event to the new value of a single-value control: one `notch` per event, up for positive scroll.
/// Vertical scroll counts first; horizontal scroll counts when there is none (a vertical-only control ignores it),
/// flipped under right-to-left text. Shift scales the notch by `large / small` and Alt by `fine / small`; both
/// together keep the notch, as the arrow keys do. Zero scroll is `Ignored`.
pub fn wheel_step(steps: &ValueSteps, axes: ValueAxes, wheel: &Wheel, rtl: bool) -> StepResult {
    let along = if wheel.scroll.y != 0.0 {
        wheel.scroll.y
    } else if matches!(axes, ValueAxes::Vertical) || wheel.scroll.x == 0.0 {
        return StepResult::Ignored;
    } else if rtl {
        -wheel.scroll.x
    } else {
        wheel.scroll.x
    };
    let scale = match (wheel.mods.shift, wheel.mods.alt) {
        (true, false) if steps.small != 0.0 => steps.large / steps.small,
        (false, true) if steps.small != 0.0 => steps.fine / steps.small,
        _ => 1.0,
    };
    let delta = steps.notch * scale * f64::from(along.signum());
    StepResult::Set((steps.value + delta).clamp(steps.min, steps.max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::Granularity;

    struct Rows {
        depth: Vec<usize>,
        open: Vec<bool>,
        children: Vec<bool>,
        disabled: Vec<usize>,
    }

    impl Rows {
        fn flat(n: usize) -> Rows {
            Rows { depth: vec![0; n], open: vec![false; n], children: vec![false; n], disabled: vec![] }
        }
    }

    impl NavOutline for Rows {
        fn len(&self) -> usize {
            self.depth.len()
        }
        fn depth(&self, at: usize) -> usize {
            self.depth[at]
        }
        fn has_children(&self, at: usize) -> bool {
            self.children[at]
        }
        fn open(&self, at: usize) -> bool {
            self.open[at]
        }
        fn is_section(&self, _: usize) -> bool {
            false
        }
        fn enabled(&self, at: usize) -> bool {
            !self.disabled.contains(&at)
        }
    }

    fn go(shape: NavShape, rows: &Rows, state: &mut NavState, mv: NavMove) -> NavStep {
        step(shape, rows, state, mv, MoveMods::PLAIN)
    }

    #[test]
    fn flat_moves_skip_disabled_rows_and_stop_or_wrap_at_the_ends() {
        let mut rows = Rows::flat(5);
        rows.disabled = vec![2];
        let mut state = NavState::at(1);
        assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::Down), NavStep::Moved(3));
        assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::End), NavStep::Moved(4));
        assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::Forward), NavStep::Stay);
        state.wrap = true;
        assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::Forward), NavStep::Moved(0));
        assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::PageDown), NavStep::Moved(4));
        let mut none = NavState::new();
        assert_eq!(go(NavShape::Flat, &rows, &mut none, NavMove::Up), NavStep::Moved(4), "the first backward move lands on the last row");
    }

    #[test]
    fn a_page_move_from_the_last_row_wraps_only_when_the_state_wraps() {
        let rows = Rows::flat(5);
        let mut stops = NavState::at(4);
        assert_eq!(go(NavShape::Flat, &rows, &mut stops, NavMove::PageDown), NavStep::Stay);
        let mut wraps = NavState::at(4).wrap(true);
        assert_eq!(go(NavShape::Flat, &rows, &mut wraps, NavMove::PageDown), NavStep::Moved(0));
        assert_eq!(go(NavShape::Flat, &rows, &mut wraps, NavMove::PageUp), NavStep::Moved(4));
    }

    #[test]
    fn extending_keeps_the_anchor_at_the_first_lead() {
        let rows = Rows::flat(5);
        let mut state = NavState::at(1);
        let extend = MoveMods::new(true, false, Granularity::Unit);
        step(NavShape::Flat, &rows, &mut state, NavMove::Down, extend);
        step(NavShape::Flat, &rows, &mut state, NavMove::Down, extend);
        assert_eq!((state.anchor, state.lead), (Some(1), Some(3)));
        step(NavShape::Flat, &rows, &mut state, NavMove::Up, MoveMods::PLAIN);
        assert_eq!(state.anchor, None);
    }

    #[test]
    fn tree_forward_expands_then_enters_and_back_collapses_then_leaves() {
        let mut rows = Rows::flat(3);
        rows.depth = vec![0, 1, 1];
        rows.children = vec![true, false, false];
        let mut state = NavState::at(0);
        assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Forward), NavStep::Expand(0));
        rows.open[0] = true;
        assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Forward), NavStep::Moved(1));
        assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Back), NavStep::Moved(0), "Back on a child goes to its parent");
        assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Back), NavStep::Collapse(0));
        rows.open[0] = false;
        assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Back), NavStep::Stay);
    }

    #[test]
    fn wrapped_grids_move_by_columns_and_pages_land_in_the_same_column() {
        let rows = Rows::flat(10);
        let shape = NavShape::wrapped_grid(3);
        let mut state = NavState::at(4).page(2);
        assert_eq!(go(shape, &rows, &mut state, NavMove::Down), NavStep::Moved(7));
        assert_eq!(go(shape, &rows, &mut state, NavMove::Down), NavStep::Stay, "no row below");
        assert_eq!(go(shape, &rows, &mut state, NavMove::Up), NavStep::Moved(4));
        assert_eq!(go(shape, &rows, &mut state, NavMove::PageDown), NavStep::Moved(9), "a short last row takes the last position");
        assert_eq!(go(shape, &rows, &mut state, NavMove::Forward), NavStep::Stay);
        assert_eq!(go(shape, &rows, &mut state, NavMove::Back), NavStep::Moved(8));
    }

    #[test]
    fn bounded_grids_stop_left_and_right_at_the_row_ends() {
        let rows = Rows::flat(9);
        let shape = NavShape::bounded_grid(3);
        let mut state = NavState::at(2);
        assert_eq!(go(shape, &rows, &mut state, NavMove::Forward), NavStep::Stay, "the end of a row does not continue on the next");
        assert_eq!(go(shape, &rows, &mut state, NavMove::Down), NavStep::Moved(5));
        let mut state = NavState::at(3);
        assert_eq!(go(shape, &rows, &mut state, NavMove::Back), NavStep::Stay, "the start of a row does not continue on the previous");
        assert_eq!(go(shape, &rows, &mut state, NavMove::Forward), NavStep::Moved(4));
        assert_eq!(go(shape, &rows, &mut NavState::at(0), NavMove::Up), NavStep::Stay, "no row above");
    }

    /// Rows of 3, 1 and 2 positions; a sheet of 3 rows by 4 columns; four items at free positions.
    struct Shapes;

    impl NavOutline for Shapes {
        fn len(&self) -> usize {
            6
        }
        fn depth(&self, _: usize) -> usize {
            0
        }
        fn has_children(&self, _: usize) -> bool {
            false
        }
        fn open(&self, _: usize) -> bool {
            false
        }
        fn is_section(&self, _: usize) -> bool {
            false
        }
        fn enabled(&self, _: usize) -> bool {
            true
        }
        fn row_len(&self, row: usize) -> Option<usize> {
            [3, 1, 2].get(row).copied()
        }
        fn columns(&self) -> usize {
            4
        }
        fn rect(&self, at: usize) -> Option<Rect> {
            let spots = [(0.0, 0.0), (100.0, 10.0), (10.0, 100.0), (110.0, 110.0), (200.0, 0.0), (0.0, 200.0)];
            spots.get(at).map(|&(x, y)| Rect::from_min_size(emath::Pos2::new(x, y), emath::Vec2::splat(20.0)))
        }
    }

    #[test]
    fn ragged_rows_keep_the_column_where_the_row_is_long_enough() {
        let mut state = NavState::at(2).page(5);
        let go = |state: &mut NavState, mv| step(NavShape::RaggedGrid, &Shapes, state, mv, MoveMods::PLAIN);
        assert_eq!(go(&mut state, NavMove::Down), NavStep::Moved(3), "the single-item row takes any column");
        assert_eq!(go(&mut state, NavMove::Down), NavStep::Moved(4), "the lead of the single-item row sits in column 0");
        assert_eq!(go(&mut state, NavMove::Down), NavStep::Stay);
        assert_eq!(go(&mut state, NavMove::PageUp), NavStep::Moved(0));
        assert_eq!(go(&mut state, NavMove::Forward), NavStep::Moved(1));
    }

    #[test]
    fn sheet_cells_move_by_column_and_rows_stop_at_the_ends() {
        let mut state = NavState::at(1);
        let go = |state: &mut NavState, mv, lead| step(NavShape::Cell, &Shapes, state, mv, MoveMods::new(false, lead, Granularity::Unit));
        assert_eq!(go(&mut state, NavMove::Forward, false), NavStep::Column(1));
        assert_eq!(go(&mut state, NavMove::End, false), NavStep::Column(3));
        assert_eq!(go(&mut state, NavMove::Forward, false), NavStep::Stay);
        assert_eq!((state.lead, state.column), (Some(1), 3));
        assert_eq!(go(&mut state, NavMove::Up, false), NavStep::Moved(0));
        assert_eq!(go(&mut state, NavMove::End, true), NavStep::Moved(5), "Ctrl+End is the last row");
        assert_eq!(state.column, 3, "a row move keeps the column");
    }

    #[test]
    fn spatial_moves_go_to_the_nearest_item_in_that_direction() {
        let mut state = NavState::at(0);
        let go = |state: &mut NavState, mv| step(NavShape::Spatial, &Shapes, state, mv, MoveMods::PLAIN);
        assert_eq!(go(&mut state, NavMove::Forward), NavStep::Moved(1), "the near item at the right, not the far one in line");
        assert_eq!(go(&mut state, NavMove::Down), NavStep::Moved(3));
        assert_eq!(go(&mut state, NavMove::Back), NavStep::Moved(2));
        assert_eq!(go(&mut state, NavMove::Up), NavStep::Moved(0));
        assert_eq!(go(&mut state, NavMove::Up), NavStep::Stay);
    }

    #[test]
    fn type_ahead_builds_a_prefix_for_one_second_and_repeated_letters_cycle() {
        let labels = ["apple", "apricot", "banana", "avocado"];
        let label = |at: usize| std::borrow::Cow::Borrowed(labels[at]);
        let mut buffer = TypeAhead::new();
        assert_eq!(buffer.push('a', 0.0), "a");
        assert_eq!(buffer.push('p', 0.5), "ap");
        assert_eq!(buffer.push('r', 1.4), "apr");
        assert_eq!(buffer.push('x', 3.0), "x", "a pause starts a new prefix");
        assert_eq!(TypeAhead::pick("ap", Some(0), 4, label), Some(0));
        assert_eq!(TypeAhead::pick("apr", Some(0), 4, label), Some(1));
        assert_eq!(TypeAhead::pick("a", Some(0), 4, label), Some(1), "one letter moves past the lead");
        assert_eq!(TypeAhead::pick("aa", Some(3), 4, label), Some(0), "a repeated letter cycles and wraps");
        assert_eq!(TypeAhead::pick("z", None, 4, label), None);
    }

    #[test]
    fn alt_steps_finely_and_shift_with_alt_steps_by_small() {
        let steps = ValueSteps::new(5.0, 0.0, 10.0, 1.0, 4.0).fine(0.25);
        let fine = MoveMods::PLAIN.fine(true);
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Forward, fine), StepResult::Set(5.25));
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Down, fine), StepResult::Set(4.75));
        let both = MoveMods::new(true, false, Granularity::Unit).fine(true);
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Forward, both), StepResult::Set(6.0));
        let StepResult::Set(default_fine) = step_value(&ValueSteps::new(5.0, 0.0, 10.0, 1.0, 4.0), ValueAxes::Both, NavMove::Forward, fine) else {
            panic!("a forward move sets the value")
        };
        assert!((default_fine - 5.1).abs() < 1e-9);
    }

    #[test]
    fn wheel_steps_ignore_zero_flip_under_rtl_and_scale_the_notch() {
        use crate::wheel::{WheelPhase, WheelUnit};
        use craft_keymap::Mods;
        use emath::vec2;
        let steps = ValueSteps::new(5.0, 0.0, 10.0, 0.01, 0.1).fine(0.001).notch(0.01);
        let wheel = |scroll, mods| Wheel::new(scroll, mods, WheelUnit::Point, WheelPhase::Move);
        let set = |wheel: Wheel, rtl| match wheel_step(&steps, ValueAxes::Both, &wheel, rtl) {
            StepResult::Set(value) => value - 5.0,
            StepResult::Ignored => f64::NAN,
        };
        assert_eq!(wheel_step(&steps, ValueAxes::Both, &wheel(vec2(0.0, 0.0), Mods::NONE), false), StepResult::Ignored);
        assert!((set(wheel(vec2(0.0, 3.0), Mods::NONE), false) - 0.01).abs() < 1e-9);
        assert!((set(wheel(vec2(0.0, -3.0), Mods::NONE), false) + 0.01).abs() < 1e-9);
        assert!((set(wheel(vec2(3.0, 0.0), Mods::NONE), false) - 0.01).abs() < 1e-9);
        assert!((set(wheel(vec2(3.0, 0.0), Mods::NONE), true) + 0.01).abs() < 1e-9);
        assert!((set(wheel(vec2(0.0, 1.0), Mods::SHIFT), false) - 0.1).abs() < 1e-9);
        assert!((set(wheel(vec2(0.0, 1.0), Mods::ALT), false) - 0.001).abs() < 1e-9);
        assert_eq!(wheel_step(&steps, ValueAxes::Vertical, &wheel(vec2(3.0, 0.0), Mods::NONE), false), StepResult::Ignored);
    }
}
