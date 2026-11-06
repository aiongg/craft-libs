//! The navigation core (spec 9.5): shapes, the key-free outline, stepping, roving groups, type-ahead and
//! single-value keys. Focus movement and collection selection share it.

use crate::intent::{Granularity, MoveMods, NavMove};

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
}

impl NavState {
    /// A state with no lead, a page of ten and no wrap.
    pub const fn new() -> Self {
        NavState { lead: None, anchor: None, page: 10, wrap: false }
    }

    /// A state at `lead`.
    pub const fn at(lead: usize) -> Self {
        NavState { lead: Some(lead), anchor: None, page: 10, wrap: false }
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
    /// The shape has no stepping rule here; the caller handles the move.
    Unhandled,
}

/// Steps the lead of a navigation of `shape` over `outline` by `mv`. `mods.extend` keeps the anchor at the
/// old lead; any other move clears it. `Cell`, `RaggedGrid` and `Spatial` need row lengths or geometry the
/// outline does not carry, so their steps are `Unhandled`.
pub fn step(shape: NavShape, outline: &dyn NavOutline, state: &mut NavState, mv: NavMove, mods: MoveMods) -> NavStep {
    let len = outline.len();
    if len == 0 {
        return NavStep::Stay;
    }
    let Some(lead) = state.lead else {
        return first_move(outline, state, mv, mods);
    };
    let lead = lead.min(len - 1);
    let result = match shape {
        NavShape::Flat => linear(outline, state, lead, mv),
        NavShape::Tree => tree(outline, state, lead, mv),
        NavShape::WrappedGrid { columns } => grid(outline, state, lead, mv, columns.max(1)),
        _ => return NavStep::Unhandled,
    };
    if let NavStep::Moved(to) = result {
        state.anchor = if mods.extend { state.anchor.or(Some(lead)) } else { None };
        state.lead = Some(to);
        if to == lead {
            return NavStep::Stay;
        }
    }
    result
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

/// `count` selectable positions from `from`, stopping at the last one reached.
fn advance(outline: &dyn NavOutline, from: usize, forward: bool, count: usize) -> usize {
    let no_wrap = NavState::new();
    let mut at = from;
    for _ in 0..count {
        match neighbour(outline, &no_wrap, at, forward) {
            Some(next) => at = next,
            None => break,
        }
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
        NavMove::PageUp => NavStep::Moved(advance(outline, lead, false, state.page)),
        NavMove::PageDown => NavStep::Moved(advance(outline, lead, true, state.page)),
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

fn grid(outline: &dyn NavOutline, state: &NavState, lead: usize, mv: NavMove, columns: usize) -> NavStep {
    let len = outline.len();
    let vertical = |down: bool, rows: usize| {
        let delta = columns * rows;
        let to = if down { lead.checked_add(delta).filter(|&at| at < len) } else { lead.checked_sub(delta) };
        // A step past the last row's end lands on the last position of a shorter row only for pages.
        to.filter(|&at| selectable(outline, at))
    };
    match mv {
        NavMove::Back => moved(neighbour(outline, state, lead, false)),
        NavMove::Forward => moved(neighbour(outline, state, lead, true)),
        NavMove::Up => moved(vertical(false, 1)),
        NavMove::Down => moved(vertical(true, 1)),
        NavMove::PageUp => moved(vertical(false, state.page).or_else(|| Some(lead % columns).filter(|&at| selectable(outline, at)))),
        NavMove::PageDown => {
            let last_row_start = (len - 1) / columns * columns;
            let column = lead % columns;
            moved(vertical(true, state.page).or_else(|| Some((last_row_start + column).min(len - 1)).filter(|&at| selectable(outline, at))))
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
}

impl Roving {
    /// A group of `shape` with Home and End, no wrap, no paging and no letters.
    pub const fn new(shape: NavShape) -> Roving {
        Roving { shape, wrap: false, home_end: true, page: None, letters: Letters::None }
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

    /// The state a step over this group starts from, at `lead`.
    pub const fn state(&self, lead: Option<usize>) -> NavState {
        let page = match self.page {
            Some(rows) if rows > 0 => rows,
            _ => 10,
        };
        NavState { lead, anchor: None, page, wrap: self.wrap }
    }

    /// Whether the group handles `mv`: Page moves only with a page size, Start and End only with Home and End.
    pub const fn takes(&self, mv: NavMove) -> bool {
        match mv {
            NavMove::PageUp | NavMove::PageDown => self.page.is_some(),
            NavMove::Start | NavMove::End => self.home_end,
            _ => true,
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
    pub fn pick<'a>(prefix: &str, from: Option<usize>, len: usize, label: impl Fn(usize) -> &'a str) -> Option<usize> {
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
}

impl ValueSteps {
    /// Steps over `min..=max` from `value`.
    pub const fn new(value: f64, min: f64, max: f64, small: f64, large: f64) -> Self {
        ValueSteps { value, min, max, small, large }
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
/// `PageUp` increase; `Start` and `End` go to the ends; Shift (`extend`) steps by `large`.
pub fn step_value(steps: &ValueSteps, axes: ValueAxes, mv: NavMove, mods: MoveMods) -> StepResult {
    let horizontal = !matches!(axes, ValueAxes::Vertical);
    let vertical = !matches!(axes, ValueAxes::Horizontal);
    let small = if mods.extend { steps.large } else { steps.small };
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(go(NavShape::Cell, &rows, &mut state, NavMove::Back), NavStep::Unhandled);
    }

    #[test]
    fn type_ahead_builds_a_prefix_for_one_second_and_repeated_letters_cycle() {
        let labels = ["apple", "apricot", "banana", "avocado"];
        let label = |at: usize| labels[at];
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
    fn single_value_moves_clamp_and_follow_the_axes() {
        let steps = ValueSteps::new(5.0, 0.0, 10.0, 1.0, 4.0);
        let plain = MoveMods::PLAIN;
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Forward, plain), StepResult::Set(6.0));
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Down, plain), StepResult::Set(4.0));
        assert_eq!(step_value(&steps, ValueAxes::Horizontal, NavMove::Up, plain), StepResult::Ignored);
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Forward, MoveMods::new(true, false, Granularity::Unit)), StepResult::Set(9.0));
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::PageUp, plain), StepResult::Set(9.0));
        assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::End, plain), StepResult::Set(10.0));
        assert_eq!(step_value(&ValueSteps::new(9.0, 0.0, 10.0, 2.0, 4.0), ValueAxes::Both, NavMove::Forward, plain), StepResult::Set(10.0));
    }
}
