//! [`ListSelection`]: the selection rules of a list over its display order, kept apart from how
//! the list draws and reads input.

use egui::{Key, Modifiers};

/// One row of a display list, for a host that keeps its rows in a slice (see [`DisplayOrder`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowInfo<K> {
    /// The row's stable key.
    pub key: K,
    /// How deep the row sits in the tree, from 0 at the top level.
    pub depth: usize,
    /// Whether the row is a group, open or closed.
    pub has_children: bool,
    /// Whether a group shows its children; they follow it in the display order when it does.
    pub open: bool,
    /// Whether the row is a section label: static text between rows that is never selected,
    /// focused or counted. See [`DisplayOrder::is_section`].
    pub section: bool,
}

impl<K> RowInfo<K> {
    /// A top-level row without children.
    pub fn leaf(key: K) -> Self {
        Self { key, depth: 0, has_children: false, open: false, section: false }
    }

    /// A top-level group, open or closed.
    pub fn group(key: K, open: bool) -> Self {
        Self { key, depth: 0, has_children: true, open, section: false }
    }

    /// A section label of a flat list: it names the rows after it, and takes no part in selection
    /// or keyboard movement. It is not a group and has no children.
    pub fn section(key: K) -> Self {
        Self { key, depth: 0, has_children: false, open: false, section: true }
    }

    /// The same row `depth` levels down.
    pub fn at_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }
}

/// The rows of a list as shown, topmost (or first) to last: a group directly above its children,
/// the children of closed groups left out. A position counts the rows shown, from 0.
pub trait DisplayOrder<K> {
    /// How many rows are shown.
    fn len(&self) -> usize;

    /// Whether no row is shown.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The key of the row at position `at`, below [`len`](DisplayOrder::len).
    fn key(&self, at: usize) -> &K;

    /// The depth of the row at `at`; 0 in a flat list.
    fn depth(&self, _at: usize) -> usize {
        0
    }

    /// Whether the row at `at` is a group.
    fn has_children(&self, _at: usize) -> bool {
        false
    }

    /// Whether the group at `at` shows its children.
    fn open(&self, _at: usize) -> bool {
        false
    }

    /// Whether the row at `at` is a section label rather than an item: it is drawn but is never
    /// selected, never the lead and never reached by a key, a click or type-ahead. Sections are for
    /// flat lists; a list that reorders or shows a tree has none.
    fn is_section(&self, _at: usize) -> bool {
        false
    }

    /// The position of `key`, if it is shown. The default looks through all rows; a host that
    /// knows the answer sooner says so.
    fn position(&self, key: &K) -> Option<usize>
    where
        K: PartialEq,
    {
        (0..self.len()).find(|&at| self.key(at) == key)
    }
}

impl<K> DisplayOrder<K> for [RowInfo<K>] {
    fn len(&self) -> usize {
        <[RowInfo<K>]>::len(self)
    }

    fn key(&self, at: usize) -> &K {
        &self[at].key
    }

    fn depth(&self, at: usize) -> usize {
        self[at].depth
    }

    fn has_children(&self, at: usize) -> bool {
        self[at].has_children
    }

    fn open(&self, at: usize) -> bool {
        self[at].open
    }

    fn is_section(&self, at: usize) -> bool {
        self[at].section
    }
}

/// The position of the nearest item (a row that is not a section) at or after `at`, else the
/// nearest before it.
pub fn nearest_item<K>(order: &(impl DisplayOrder<K> + ?Sized), at: usize) -> Option<usize> {
    (at..order.len()).find(|&p| !order.is_section(p)).or_else(|| (0..at.min(order.len())).rev().find(|&p| !order.is_section(p)))
}

/// The position of the first item.
pub fn first_item<K>(order: &(impl DisplayOrder<K> + ?Sized)) -> Option<usize> {
    (0..order.len()).find(|&p| !order.is_section(p))
}

/// The position of the last item.
pub fn last_item<K>(order: &(impl DisplayOrder<K> + ?Sized)) -> Option<usize> {
    (0..order.len()).rev().find(|&p| !order.is_section(p))
}

/// The position of the parent of the row at `at`: the nearest row above it that is shallower.
pub fn parent<K>(order: &(impl DisplayOrder<K> + ?Sized), at: usize) -> Option<usize> {
    let depth = order.depth(at);
    (0..at).rev().find(|&p| order.depth(p) < depth)
}

/// The position of the first child of the row at `at`: the row below it, when it is a group that
/// shows children and that row is deeper.
pub fn first_child<K>(order: &(impl DisplayOrder<K> + ?Sized), at: usize) -> Option<usize> {
    let next = at + 1;
    (order.has_children(at) && order.open(at) && next < order.len() && order.depth(next) > order.depth(at)).then_some(next)
}

/// What the key toward the end of a row's reading direction (WAI-ARIA tree: Right, or Left in a
/// right-to-left layout) or toward its start does on the row at a position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeMove {
    /// Open the closed group at the position.
    Open(usize),
    /// Close the open group at the position.
    Close(usize),
    /// Move the lead to the row at the position: a group's first child, or a row's parent.
    To(usize),
}

/// The tree key's effect on the row at `at`: toward the end opens a closed group or moves to its
/// first child; toward the start closes an open group or moves to the parent. `None` where
/// nothing happens (a leaf toward the end, a top-level row that is not open toward the start).
pub fn tree_key<K>(order: &(impl DisplayOrder<K> + ?Sized), at: usize, toward_end: bool) -> Option<TreeMove> {
    if at >= order.len() {
        return None;
    }
    let group = order.has_children(at);
    if toward_end {
        match (group, order.open(at)) {
            (true, false) => Some(TreeMove::Open(at)),
            (true, true) => first_child(order, at).map(TreeMove::To),
            (false, _) => None,
        }
    } else if group && order.open(at) {
        Some(TreeMove::Close(at))
    } else {
        parent(order, at).map(TreeMove::To)
    }
}

/// The selected rows of a list, owned by the host: the selected keys, the *lead* (the active row
/// and the keyboard's position) and the *anchor* (where a range starts).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection<K> {
    /// The selected keys, in the order they were selected.
    pub items: Vec<K>,
    /// The active row.
    pub lead: Option<K>,
    /// The row a range extends from.
    pub anchor: Option<K>,
}

impl<K> Default for Selection<K> {
    fn default() -> Self {
        Self { items: Vec::new(), lead: None, anchor: None }
    }
}

impl<K: Clone + PartialEq> Selection<K> {
    /// No row selected.
    pub fn new() -> Self {
        Self::default()
    }

    /// Only `key` selected, as lead and anchor.
    pub fn only(key: K) -> Self {
        Self { items: vec![key.clone()], lead: Some(key.clone()), anchor: Some(key) }
    }

    /// Whether `key` is selected.
    pub fn contains(&self, key: &K) -> bool {
        self.items.contains(key)
    }

    /// Whether no row is selected.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// How many rows a list selects at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// One row; every click replaces, modifiers are ignored.
    Single,
    /// Any number of rows.
    #[default]
    Multiple,
}

/// How a change was picked, for a host whose engine has its own rules for each (PhotoCraft's
/// `layer.select` modes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionPick {
    /// The row replaces the selection.
    Replace,
    /// The row was added to or removed from the selection.
    Toggle,
    /// The rows from the anchor to the row replace the selection.
    Range,
    /// The selection was emptied.
    Clear,
}

/// A change of the selection: how it was picked, on which row (none for a clear), and the
/// selection that results, for a host that takes it as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionChange<K> {
    /// How the change was picked.
    pub pick: SelectionPick,
    /// The row that was picked; `None` for [`SelectionPick::Clear`].
    pub row: Option<K>,
    /// The selection after the change.
    pub selection: Selection<K>,
}

/// Where a key moves the lead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The previous row.
    Previous,
    /// The next row.
    Next,
    /// A page up.
    PageUp,
    /// A page down.
    PageDown,
    /// The first row.
    First,
    /// The last row.
    Last,
}

/// What a key does in a list that has the keyboard focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKey {
    /// Move the lead and select it alone, or with `extend` extend the range from the anchor to it.
    Select { step: Step, extend: bool },
    /// Move the lead without changing the selection.
    MoveLead(Step),
    /// Toggle the lead's row (select it in single mode).
    ToggleLead,
    /// The row's default action (Enter).
    Activate,
    /// Rename the lead's row (F2).
    Rename,
}

impl ListKey {
    /// What `key` pressed with `modifiers` does, if anything. `mac` says whether Cmd and Control
    /// are different keys (macOS). Keys match their modifiers exactly; Cmd/Ctrl+A, Delete and
    /// other commands stay the app's.
    ///
    /// - Up, Down, Page Up, Page Down, Home, End select (Shift extends). Ctrl+Home and Ctrl+End,
    ///   and on macOS Cmd+Up and Cmd+Down, go to the first and last row.
    /// - Control+Up and Control+Down (Control on every platform) move the lead only; Space toggles.
    /// - Enter activates; F2 renames.
    pub fn from_key(key: Key, modifiers: Modifiers, mac: bool) -> Option<Self> {
        let Modifiers { alt, ctrl, shift, mac_cmd, .. } = modifiers;
        if alt {
            return None;
        }
        let none = !ctrl && !mac_cmd;
        let select = |step, extend| Some(Self::Select { step, extend });
        let plain_step = match key {
            Key::ArrowUp => Some(Step::Previous),
            Key::ArrowDown => Some(Step::Next),
            Key::PageUp => Some(Step::PageUp),
            Key::PageDown => Some(Step::PageDown),
            Key::Home => Some(Step::First),
            Key::End => Some(Step::Last),
            _ => None,
        };
        let to_end = match key {
            Key::Home => Some(Step::First),
            Key::End => Some(Step::Last),
            Key::ArrowUp if mac => Some(Step::First),
            Key::ArrowDown if mac => Some(Step::Last),
            _ => None,
        };
        match key {
            Key::Enter if none && !shift => Some(Self::Activate),
            Key::F2 if none && !shift => Some(Self::Rename),
            Key::Space if none && !shift => Some(Self::ToggleLead),
            _ if none => plain_step.and_then(|step| select(step, shift)),
            Key::ArrowUp if ctrl && !mac_cmd && !shift => Some(Self::MoveLead(Step::Previous)),
            Key::ArrowDown if ctrl && !mac_cmd && !shift => Some(Self::MoveLead(Step::Next)),
            _ if ctrl && !mac_cmd && matches!(key, Key::Home | Key::End) => to_end.and_then(|step| select(step, shift)),
            _ if mac_cmd && !ctrl => to_end.filter(|_| matches!(key, Key::ArrowUp | Key::ArrowDown)).and_then(|step| select(step, shift)),
            _ => None,
        }
    }
}

/// The rules that turn clicks and keys into [`SelectionChange`]s, over a display order. Pure: the
/// list reads the input and draws; the host keeps the [`Selection`].
///
/// Pointer, as in PhotoCraft's Layers panel:
/// - A click replaces the selection; lead and anchor are the row.
/// - Cmd-click (Ctrl-click elsewhere) toggles the row and makes it the anchor; it never removes
///   the last selected row. Removing the lead makes the last remaining row the lead; adding a row
///   makes it the lead. Cmd/Ctrl wins when Shift is held too.
/// - Shift-click selects the rows from the anchor to the row, replacing the selection; the row is
///   the lead and the anchor stays. With no anchor (or one that is no longer shown) the range
///   starts at the lead, else at the row; that start becomes the anchor.
/// - A click on empty space clears the selection, where empty is allowed.
/// - In [`SelectionMode::Single`] every click replaces and modifiers are ignored.
///
/// Keys, as WAI-ARIA listboxes and trees have them ([`ListKey`]): Up and Down move the lead and
/// select it alone, without wrapping; Shift extends from the anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListSelection {
    mode: SelectionMode,
    allow_empty: bool,
}

impl Default for ListSelection {
    fn default() -> Self {
        Self::new(SelectionMode::Multiple)
    }
}

impl ListSelection {
    /// Rules for `mode`, where the selection may be empty.
    pub fn new(mode: SelectionMode) -> Self {
        Self { mode, allow_empty: true }
    }

    /// Whether a click on empty space may clear the selection (default true); a list that
    /// always has a current row, such as a history, says false.
    pub fn allow_empty(mut self, allow_empty: bool) -> Self {
        self.allow_empty = allow_empty;
        self
    }

    /// The selection mode.
    pub fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// A click on the row at position `at` with `modifiers` held. `None` when it changes nothing.
    pub fn click<K: Clone + PartialEq>(
        &self,
        selection: &Selection<K>,
        order: &(impl DisplayOrder<K> + ?Sized),
        at: usize,
        modifiers: Modifiers,
    ) -> Option<SelectionChange<K>> {
        if at >= order.len() || order.is_section(at) {
            return None;
        }
        let row = order.key(at).clone();
        let multiple = self.mode == SelectionMode::Multiple;
        let (pick, new) = if multiple && modifiers.command {
            (SelectionPick::Toggle, Self::toggled(selection, row.clone()))
        } else if multiple && modifiers.shift {
            (SelectionPick::Range, Self::ranged(selection, order, at))
        } else {
            (SelectionPick::Replace, Selection::only(row.clone()))
        };
        Self::change(selection, pick, Some(row), new)
    }

    /// A click on the empty space below the rows. `None` when empty is not allowed or the
    /// selection is already empty.
    pub fn click_empty<K: Clone + PartialEq>(&self, selection: &Selection<K>) -> Option<SelectionChange<K>> {
        if !self.allow_empty || selection.is_empty() {
            return None;
        }
        Self::change(selection, SelectionPick::Clear, None, Selection::new())
    }

    /// The position the lead moves to for `step`, over `page` rows at a time, without wrapping.
    /// With no lead, Next goes to the first item and Previous to the last. Sections are skipped.
    /// `None` for a list without items.
    pub fn target<K: PartialEq>(&self, selection: &Selection<K>, order: &(impl DisplayOrder<K> + ?Sized), step: Step, page: usize) -> Option<usize> {
        let (first, last) = (first_item(order)?, last_item(order)?);
        let page = page.max(1);
        let at = selection.lead.as_ref().and_then(|lead| order.position(lead));
        // A step that lands on a section moves on to the item after it, or before it at the end.
        let item = |p: usize| nearest_item(order, p);
        Some(match step {
            Step::Next => at.map_or(first, |p| item(p + 1).filter(|&n| n > p).unwrap_or(p)),
            Step::Previous => at.map_or(last, |p| (0..p).rev().find(|&n| !order.is_section(n)).unwrap_or(p)),
            Step::PageDown => at.map_or(first, |p| item((p + page).min(last)).unwrap_or(p)),
            Step::PageUp => at.map_or(last, |p| item(p.saturating_sub(page).max(first)).unwrap_or(p)),
            Step::First => first,
            Step::Last => last,
        })
    }

    /// Up, Down, Page Up, Page Down, Home or End: the lead moves to the target of `step` and
    /// selects it alone, or with `extend` the range from the anchor to it replaces the selection
    /// (single mode ignores `extend`). `None` when the selection stays as it is.
    pub fn select<K: Clone + PartialEq>(
        &self,
        selection: &Selection<K>,
        order: &(impl DisplayOrder<K> + ?Sized),
        step: Step,
        page: usize,
        extend: bool,
    ) -> Option<SelectionChange<K>> {
        let at = self.target(selection, order, step, page)?;
        let row = order.key(at).clone();
        if extend && self.mode == SelectionMode::Multiple {
            Self::change(selection, SelectionPick::Range, Some(row), Self::ranged(selection, order, at))
        } else {
            Self::change(selection, SelectionPick::Replace, Some(row.clone()), Selection::only(row))
        }
    }

    /// Control+Up or Down: the key of the row the lead would move to, for the host to make the
    /// lead without changing the selection. `None` when the lead stays.
    pub fn move_lead<K: Clone + PartialEq>(
        &self,
        selection: &Selection<K>,
        order: &(impl DisplayOrder<K> + ?Sized),
        step: Step,
        page: usize,
    ) -> Option<K> {
        let at = self.target(selection, order, step, page)?;
        let key = order.key(at);
        (selection.lead.as_ref() != Some(key)).then(|| key.clone())
    }

    /// Space: toggles the lead's row in multiple mode, selects it in single mode. `None` with no
    /// lead shown.
    pub fn toggle_lead<K: Clone + PartialEq>(&self, selection: &Selection<K>, order: &(impl DisplayOrder<K> + ?Sized)) -> Option<SelectionChange<K>> {
        let at = selection.lead.as_ref().and_then(|lead| order.position(lead))?;
        self.click(selection, order, at, Modifiers { command: self.mode == SelectionMode::Multiple, ..Modifiers::NONE })
    }

    fn toggled<K: Clone + PartialEq>(selection: &Selection<K>, row: K) -> Selection<K> {
        let mut new = selection.clone();
        new.anchor = Some(row.clone());
        if let Some(index) = new.items.iter().position(|k| *k == row) {
            if new.items.len() > 1 {
                new.items.remove(index);
                if new.lead.as_ref() == Some(&row) {
                    new.lead = new.items.last().cloned();
                }
            }
        } else {
            new.items.push(row.clone());
            new.lead = Some(row);
        }
        new
    }

    fn ranged<K: Clone + PartialEq>(selection: &Selection<K>, order: &(impl DisplayOrder<K> + ?Sized), at: usize) -> Selection<K> {
        let shown = |key: &Option<K>| key.as_ref().and_then(|k| order.position(k));
        let from = shown(&selection.anchor).or_else(|| shown(&selection.lead)).unwrap_or(at);
        let (first, last) = (from.min(at), from.max(at));
        Selection {
            items: (first..=last).filter(|&p| !order.is_section(p)).map(|p| order.key(p).clone()).collect(),
            lead: Some(order.key(at).clone()),
            anchor: Some(order.key(from).clone()),
        }
    }

    fn change<K: PartialEq>(old: &Selection<K>, pick: SelectionPick, row: Option<K>, new: Selection<K>) -> Option<SelectionChange<K>> {
        (new != *old).then_some(SelectionChange { pick, row, selection: new })
    }
}
