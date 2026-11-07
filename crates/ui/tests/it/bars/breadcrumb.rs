//! `BreadcrumbBar`: how a path splits into crumbs, which crumbs a room shows, the nodes it reports and how a long
//! ancestor elides.

use craft_ui::bars::{BreadcrumbBar, path_crumbs};
use craft_ui::contract::{Entry, Role, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::Vec2;
use std::cell::Cell;
use std::rc::Rc;

const NAMES: [&str; 5] = ["photos", "2026", "trip", "day-one", "selects"];
const HIDDEN: &str = "Show the whole path";
const WIDE: Vec2 = Vec2::new(900.0, 80.0);
const NARROW: Vec2 = Vec2::new(170.0, 80.0);

fn bar(names: &[&str], room: Vec2) -> (Fixture, Rc<Cell<Option<u8>>>) {
    let crumbs: Vec<Entry<u8>> = names.iter().zip(0u8..).map(|(name, at)| Entry::new(at, (*name).to_owned())).collect();
    let picked = Rc::new(Cell::new(None));
    let seen = picked.clone();
    let mut fixture = Fixture::new(move |ui| {
        if let Some(at) = BreadcrumbBar::new("Path", &crumbs).show(ui).picked {
            seen.set(Some(at));
        }
    });
    fixture.room(room).settle();
    (fixture, picked)
}

fn pairs(path: &str) -> Vec<(String, String)> {
    path_crumbs(path).iter().map(|crumb| (crumb.label().as_str().to_owned(), crumb.value().cloned().unwrap_or_default())).collect()
}

fn expected(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(label, key)| ((*label).to_owned(), (*key).to_owned())).collect()
}

/// Each crumb's value is the path up to it in the path's own spelling; empty parts are skipped.
#[trial]
fn path_crumbs_keep_the_spelling_of_the_path() {
    assert_eq!(pairs("/a/b"), expected(&[("a", "/a"), ("b", "/a/b")]));
    assert_eq!(pairs("a//b/"), expected(&[("a", "a"), ("b", "a//b")]), "empty parts are skipped, the spelling kept");
    assert_eq!(pairs("C:\\x"), expected(&[("C:", "C:\\"), ("x", "C:\\x")]), "a bare drive gets its separator back");
    assert_eq!(pairs("C:/x"), expected(&[("C:", "C:/"), ("x", "C:/x")]));
    assert_eq!(pairs("\\\\server\\share\\x"), expected(&[("server", "\\\\server"), ("share", "\\\\server\\share"), ("x", "\\\\server\\share\\x")]));
    assert!(pairs("").is_empty() && pairs("/").is_empty());
}

/// Five short crumbs all show in a wide bar. In a narrow bar the root end moves behind the "…" part, whose menu has
/// one row per hidden crumb, and picking a row reports that crumb.
#[trial]
fn the_root_end_overflows_into_a_menu_that_navigates() {
    let (wide, _) = bar(&NAMES, WIDE);
    assert!(NAMES.iter().all(|name| wide.by_label(name).is_some()), "every crumb shows in a wide bar");
    assert!(wide.by_label(HIDDEN).is_none(), "nothing overflows");

    let (mut narrow, picked) = bar(&NAMES, NARROW);
    assert!(narrow.by_label("selects").is_some(), "the current crumb always shows");
    assert!(narrow.by_label("photos").is_none(), "the root end is hidden");
    let more = narrow.by_label(HIDDEN).expect("the hidden crumbs have a part");
    narrow.click(&more).settle();
    let row = narrow.by_label("photos").expect("the menu has a row for the root");
    narrow.click(&row).settle();
    assert_eq!(picked.get(), Some(0), "the row navigates to its crumb");
}

/// The last crumb is the current page; the bar is a navigation landmark.
#[trial]
fn the_last_crumb_is_current_and_the_bar_is_navigation() {
    let (fixture, _) = bar(&NAMES, WIDE);
    let tree = fixture.tree();
    let last = fixture.by_label("selects").expect("the last crumb is a node");
    let first = fixture.by_label("photos").expect("the first crumb is a node");
    assert!(tree.current_of(last.node), "the last crumb reports current");
    assert!(!tree.current_of(first.node), "an ancestor does not");
    let landmarks = fixture.by_role(Role::NAVIGATION);
    assert_eq!(landmarks.len(), 1, "the bar is the one navigation node");
}

/// An ancestor longer than the crumb cap shows an elided text, and its node keeps the whole name.
#[trial]
fn a_long_ancestor_elides_and_keeps_its_name() {
    let long = "an-ancestor-directory-whose-name-runs-far-past-any-sensible-width-for-one-crumb";
    let (fixture, _) = bar(&[long, "child"], WIDE);
    assert!(fixture.by_label(long).is_some(), "the node is named by the whole text");
    let shown = fixture.texts().iter().find(|text| text.text.starts_with("an-")).expect("the crumb paints text");
    assert!(shown.elided, "the painted text is cut: {}", shown.text);
    assert_eq!(shown.text, long, "an elided run reports its full text");
}
