use egui::{Key, Modifiers};

use super::list_selection::{ListKey, SelectionPick as Pick, Step, TreeMove, first_child, parent, tree_key};
use super::{ListSelection, RowInfo, Selection, SelectionMode};

fn flat(n: u32) -> Vec<RowInfo<u32>> {
    (0..n).map(RowInfo::leaf).collect()
}

/// A tree, as displayed: 0, group 1 (open) with 2 and group 3 (closed), then 4, group 5 (open)
/// with 6 and 7.
fn tree() -> Vec<RowInfo<u32>> {
    vec![
        RowInfo::leaf(0),
        RowInfo::group(1, true),
        RowInfo::leaf(2).at_depth(1),
        RowInfo::group(3, false).at_depth(1),
        RowInfo::leaf(4),
        RowInfo::group(5, true),
        RowInfo::leaf(6).at_depth(1),
        RowInfo::leaf(7).at_depth(1),
    ]
}

const CMD: Modifiers = Modifiers { command: true, ..Modifiers::NONE };
const SHIFT: Modifiers = Modifiers::SHIFT;

fn sel(items: &[u32], lead: u32, anchor: u32) -> Selection<u32> {
    Selection { items: items.to_vec(), lead: Some(lead), anchor: Some(anchor) }
}

fn click(rules: ListSelection, s: &Selection<u32>, rows: &[RowInfo<u32>], at: usize, m: Modifiers) -> Selection<u32> {
    rules.click(s, rows, at, m).map_or_else(|| s.clone(), |c| c.selection)
}

#[test]
fn a_click_replaces_the_selection_and_sets_lead_and_anchor() {
    let rows = flat(5);
    let rules = ListSelection::default();
    let change = rules.click(&sel(&[1, 2], 2, 1), &rows[..], 4, Modifiers::NONE).unwrap();
    assert_eq!((change.pick, change.row), (Pick::Replace, Some(4)));
    assert_eq!(change.selection, sel(&[4], 4, 4));
    assert_eq!(rules.click(&sel(&[4], 4, 4), &rows[..], 4, Modifiers::NONE), None);
    assert_eq!(rules.click(&Selection::new(), &rows[..], 9, Modifiers::NONE), None);
}

#[test]
fn a_command_click_toggles_and_moves_the_anchor() {
    let rows = flat(5);
    let rules = ListSelection::default();
    let added = rules.click(&sel(&[1], 1, 1), &rows[..], 3, CMD).unwrap();
    assert_eq!(added.pick, Pick::Toggle);
    assert_eq!(added.selection, sel(&[1, 3], 3, 3));
    let removed = click(rules, &added.selection, &rows, 1, CMD);
    assert_eq!(removed, sel(&[3], 3, 1));
}

#[test]
fn a_toggle_never_removes_the_last_row() {
    let rows = flat(3);
    let rules = ListSelection::default();
    let one = sel(&[1], 1, 0);
    let after = click(rules, &one, &rows, 1, CMD);
    assert_eq!(after.items, vec![1]);
    assert_eq!(after.anchor, Some(1));
}

#[test]
fn removing_the_lead_makes_the_last_remaining_row_the_lead() {
    let rows = flat(6);
    let rules = ListSelection::default();
    let after = click(rules, &sel(&[4, 1, 2], 2, 2), &rows, 2, CMD);
    assert_eq!(after.items, vec![4, 1]);
    assert_eq!(after.lead, Some(1));
    let other = click(rules, &sel(&[4, 1, 2], 2, 2), &rows, 4, CMD);
    assert_eq!(other.lead, Some(2));
}

#[test]
fn a_shift_click_selects_the_range_from_the_anchor_and_keeps_the_anchor() {
    let rows = flat(8);
    let rules = ListSelection::default();
    let start = sel(&[2], 2, 2);
    let down = rules.click(&start, &rows[..], 5, SHIFT).unwrap();
    assert_eq!(down.pick, Pick::Range);
    assert_eq!(down.selection, sel(&[2, 3, 4, 5], 5, 2));
    let up = click(rules, &down.selection, &rows, 0, SHIFT);
    assert_eq!(up, sel(&[0, 1, 2], 0, 2));
}

#[test]
fn a_shift_click_without_an_anchor_starts_at_the_lead_then_the_row() {
    let rows = flat(6);
    let rules = ListSelection::default();
    let lead_only = Selection { items: vec![1], lead: Some(1), anchor: None };
    assert_eq!(click(rules, &lead_only, &rows, 3, SHIFT), sel(&[1, 2, 3], 3, 1));
    assert_eq!(click(rules, &Selection::new(), &rows, 3, SHIFT), sel(&[3], 3, 3));
    let gone = sel(&[9], 9, 9);
    assert_eq!(click(rules, &gone, &rows, 2, SHIFT), sel(&[2], 2, 2));
}

#[test]
fn a_range_skips_the_children_of_closed_groups() {
    let rows = tree();
    let rules = ListSelection::default();
    // Display order 0, 1, 2, 3 (closed), 4: nothing of 3's children is listed.
    let after = click(rules, &sel(&[1], 1, 1), &rows, 4, SHIFT);
    assert_eq!(after.items, vec![1, 2, 3, 4]);
}

#[test]
fn command_wins_over_shift() {
    let rows = flat(5);
    let rules = ListSelection::default();
    let both = Modifiers { command: true, shift: true, ..Modifiers::NONE };
    assert_eq!(rules.click(&sel(&[0], 0, 0), &rows[..], 3, both).unwrap().pick, Pick::Toggle);
}

#[test]
fn single_mode_replaces_on_every_click() {
    let rows = flat(5);
    let rules = ListSelection::new(SelectionMode::Single);
    let s = sel(&[1], 1, 1);
    for m in [CMD, SHIFT, Modifiers::NONE] {
        let change = rules.click(&s, &rows[..], 3, m).unwrap();
        assert_eq!((change.pick, change.selection), (Pick::Replace, sel(&[3], 3, 3)));
    }
}

#[test]
fn a_click_on_empty_space_clears_where_empty_is_allowed() {
    let s = sel(&[1, 2], 2, 1);
    let change = ListSelection::default().click_empty(&s).unwrap();
    assert_eq!((change.pick, change.row, change.selection), (Pick::Clear, None, Selection::new()));
    assert_eq!(ListSelection::default().click_empty(&Selection::<u32>::new()), None);
    assert_eq!(ListSelection::default().allow_empty(false).click_empty(&s), None);
}

#[test]
fn arrows_move_the_lead_and_select_it_alone_without_wrapping() {
    let rows = flat(4);
    let rules = ListSelection::default();
    let s = sel(&[1, 2], 2, 1);
    let down = rules.select(&s, &rows[..], Step::Next, 3, false).unwrap();
    assert_eq!((down.pick, down.selection), (Pick::Replace, sel(&[3], 3, 3)));
    let at_end = sel(&[3], 3, 3);
    assert_eq!(rules.select(&at_end, &rows[..], Step::Next, 3, false), None);
    let at_start = sel(&[0], 0, 0);
    assert_eq!(rules.select(&at_start, &rows[..], Step::Previous, 3, false), None);
    assert_eq!(click(rules, &at_end, &rows, 3, Modifiers::NONE), at_end);
}

#[test]
fn with_no_lead_down_goes_to_the_first_row_and_up_to_the_last() {
    let rows = flat(4);
    let rules = ListSelection::default();
    let none = Selection::<u32>::new();
    assert_eq!(rules.select(&none, &rows[..], Step::Next, 3, false).unwrap().selection.lead, Some(0));
    assert_eq!(rules.select(&none, &rows[..], Step::Previous, 3, false).unwrap().selection.lead, Some(3));
    assert_eq!(rules.select(&none, &flat(0)[..], Step::Next, 3, false), None);
}

#[test]
fn pages_and_ends_stop_at_the_ends() {
    let rows = flat(10);
    let rules = ListSelection::default();
    let at = |lead: u32, step, page| rules.target(&sel(&[lead], lead, lead), &rows[..], step, page);
    assert_eq!(at(2, Step::PageDown, 4), Some(6));
    assert_eq!(at(8, Step::PageDown, 4), Some(9));
    assert_eq!(at(3, Step::PageUp, 4), Some(0));
    assert_eq!(at(5, Step::PageUp, 4), Some(1));
    assert_eq!(at(5, Step::First, 4), Some(0));
    assert_eq!(at(5, Step::Last, 4), Some(9));
    assert_eq!(at(5, Step::PageDown, 0), Some(6));
}

#[test]
fn shift_with_a_step_extends_the_range_from_the_anchor() {
    let rows = flat(10);
    let rules = ListSelection::default();
    let s = sel(&[4], 4, 4);
    let one = rules.select(&s, &rows[..], Step::Next, 3, true).unwrap();
    assert_eq!((one.pick, one.selection.clone()), (Pick::Range, sel(&[4, 5], 5, 4)));
    let back = rules.select(&one.selection, &rows[..], Step::Previous, 3, true).unwrap();
    assert_eq!(back.selection, sel(&[4], 4, 4));
    let past = rules.select(&back.selection, &rows[..], Step::Previous, 3, true).unwrap();
    assert_eq!(past.selection, sel(&[3, 4], 3, 4));
    let to_end = rules.select(&s, &rows[..], Step::Last, 3, true).unwrap();
    assert_eq!(to_end.selection.items, vec![4, 5, 6, 7, 8, 9]);
    let single = ListSelection::new(SelectionMode::Single).select(&s, &rows[..], Step::Next, 3, true).unwrap();
    assert_eq!((single.pick, single.selection), (Pick::Replace, sel(&[5], 5, 5)));
}

#[test]
fn moving_the_lead_leaves_the_selection_alone() {
    let rows = flat(5);
    let rules = ListSelection::default();
    let s = sel(&[1, 2], 2, 1);
    assert_eq!(rules.move_lead(&s, &rows[..], Step::Next, 3), Some(3));
    assert_eq!(rules.move_lead(&sel(&[4], 4, 4), &rows[..], Step::Next, 3), None);
}

#[test]
fn space_toggles_the_lead_in_multiple_mode_and_selects_it_in_single_mode() {
    let rows = flat(5);
    let off = Selection { items: vec![1], lead: Some(3), anchor: Some(1) };
    let multiple = ListSelection::default().toggle_lead(&off, &rows[..]).unwrap();
    assert_eq!((multiple.pick, multiple.selection.items), (Pick::Toggle, vec![1, 3]));
    let on = sel(&[1, 3], 3, 3);
    assert_eq!(ListSelection::default().toggle_lead(&on, &rows[..]).unwrap().selection.items, vec![1]);
    let single = ListSelection::new(SelectionMode::Single).toggle_lead(&off, &rows[..]).unwrap();
    assert_eq!((single.pick, single.selection), (Pick::Replace, sel(&[3], 3, 3)));
    assert_eq!(ListSelection::default().toggle_lead(&Selection::new(), &rows[..]), None);
}

#[test]
fn keys_map_to_list_actions_with_exact_modifiers() {
    let select = |step, extend| Some(ListKey::Select { step, extend });
    let key = |k, m, mac| ListKey::from_key(k, m, mac);
    assert_eq!(key(Key::ArrowDown, Modifiers::NONE, false), select(Step::Next, false));
    assert_eq!(key(Key::ArrowUp, SHIFT, false), select(Step::Previous, true));
    assert_eq!(key(Key::PageDown, Modifiers::NONE, true), select(Step::PageDown, false));
    assert_eq!(key(Key::Home, Modifiers::NONE, false), select(Step::First, false));
    assert_eq!(key(Key::End, Modifiers::CTRL, false), select(Step::Last, false));
    assert_eq!(key(Key::End, Modifiers { ctrl: true, shift: true, ..Modifiers::NONE }, true), select(Step::Last, true));
    assert_eq!(key(Key::ArrowUp, Modifiers::MAC_CMD, true), select(Step::First, false));
    assert_eq!(key(Key::ArrowDown, Modifiers { mac_cmd: true, shift: true, ..Modifiers::NONE }, true), select(Step::Last, true));
    assert_eq!(key(Key::ArrowDown, Modifiers::MAC_CMD, false), None);
    assert_eq!(key(Key::ArrowDown, Modifiers::CTRL, false), Some(ListKey::MoveLead(Step::Next)));
    assert_eq!(key(Key::ArrowUp, Modifiers::CTRL, true), Some(ListKey::MoveLead(Step::Previous)));
    assert_eq!(key(Key::Space, Modifiers::NONE, false), Some(ListKey::ToggleLead));
    assert_eq!(key(Key::Enter, Modifiers::NONE, true), Some(ListKey::Activate));
    assert_eq!(key(Key::F2, Modifiers::NONE, false), Some(ListKey::Rename));
    assert_eq!(key(Key::A, Modifiers::CTRL, false), None);
    assert_eq!(key(Key::ArrowDown, Modifiers::ALT, false), None);
    assert_eq!(key(Key::Delete, Modifiers::NONE, false), None);
}

#[test]
fn parent_and_first_child_follow_the_depths() {
    let rows = tree();
    let r = &rows[..];
    assert_eq!(parent(r, 2), Some(1));
    assert_eq!(parent(r, 3), Some(1));
    assert_eq!(parent(r, 7), Some(5));
    assert_eq!(parent(r, 4), None);
    assert_eq!(first_child(r, 1), Some(2));
    assert_eq!(first_child(r, 3), None);
    assert_eq!(first_child(r, 0), None);
    assert_eq!(first_child(r, 7), None);
}

#[test]
fn tree_keys_open_close_and_walk_the_levels() {
    let rows = tree();
    let r = &rows[..];
    assert_eq!(tree_key(r, 3, true), Some(TreeMove::Open(3)));
    assert_eq!(tree_key(r, 1, true), Some(TreeMove::To(2)));
    assert_eq!(tree_key(r, 0, true), None);
    assert_eq!(tree_key(r, 1, false), Some(TreeMove::Close(1)));
    assert_eq!(tree_key(r, 3, false), Some(TreeMove::To(1)));
    assert_eq!(tree_key(r, 2, false), Some(TreeMove::To(1)));
    assert_eq!(tree_key(r, 4, false), None);
    assert_eq!(tree_key(r, 99, true), None);
}
