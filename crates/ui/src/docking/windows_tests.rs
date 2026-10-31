//! Document window bookkeeping.

use egui::{Rect, pos2, vec2};

use super::windows::{DocumentGroup, DocumentWindows};

fn windows(documents: &[&'static str]) -> DocumentWindows<&'static str> {
    let mut w = DocumentWindows::default();
    w.docked = vec![DocumentGroup { id: 1, documents: documents.to_vec(), shown: documents.first().copied() }];
    w
}

const RECT: Rect = Rect::from_min_max(pos2(40.0, 40.0), pos2(340.0, 240.0));

#[test]
// Float then consolidate returns every document to one docked group with the active one shown.
fn float_and_consolidate_round_trip() {
    let mut w = windows(&["a", "b", "c"]);
    let id = w.float(&"b", RECT).unwrap();
    assert_eq!(w.docked[0].documents, ["a", "c"]);
    assert_eq!(w.floating[0].0.id, id);
    assert_ne!(id, w.docked[0].id);
    assert!(w.float(&"zzz", RECT).is_none());
    w.new_window_for(&"a", RECT);
    w.float_all(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0)), vec2(300.0, 200.0));
    assert!(w.docked.is_empty());
    assert_eq!(w.floating.len(), 3, "a document open twice floats once");
    assert_eq!(w.floating[1].1.min, pos2(28.0, 28.0));
    w.consolidate(Some(&"c"));
    assert!(w.floating.is_empty());
    assert_eq!(w.docked.len(), 1);
    assert_eq!(w.docked[0].documents, ["a", "c", "b"]);
    assert_eq!(w.docked[0].shown, Some("c"));
}

#[test]
// split_into(2) with three documents gives groups of two and one, in order.
fn split_into_groups_in_order() {
    let mut w = windows(&["a", "b", "c"]);
    w.docked[0].shown = Some("c");
    w.split_into(2);
    let groups: Vec<_> = w.docked.iter().map(|g| (g.documents.clone(), g.shown)).collect();
    assert_eq!(groups, [(vec!["a", "b"], Some("a")), (vec!["c"], Some("c"))]);
    assert_ne!(w.docked[0].id, w.docked[1].id);
}

#[test]
// A floating group opens and floats in an empty layout, moves when its rect differs, and a docked group's panel closes.
fn to_actions_bridges_floating_groups_to_the_dock() {
    use super::{Action, Floating, Layout};
    let mut w = windows(&["a", "b"]);
    let id = w.float(&"b", RECT).unwrap();
    let actions = w.to_actions(&Layout::<u64>::default(), |g| g);
    assert_eq!(actions.len(), 2);
    assert!(matches!(actions[0], Action::Open { panel, anchor: None } if panel == id));
    assert!(matches!(actions[1], Action::Float { panel, rect } if panel == id && rect == [40.0, 40.0, 300.0, 200.0]));

    let mut layout = Layout::<u64>::default();
    layout.floating.push(Floating::single(vec![id], 0, [0.0, 0.0, 300.0, 200.0]));
    layout.floating.push(Floating::single(vec![w.docked[0].id], 0, [0.0, 0.0, 10.0, 10.0]));
    let actions = w.to_actions(&layout, |g| g);
    assert!(matches!(actions[0], Action::MoveFloating { panel, .. } if panel == id));
    assert!(matches!(actions[1], Action::Close { panel } if panel == w.docked[0].id));
}
