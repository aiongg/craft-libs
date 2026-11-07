//! `TreePicker`: each opening shows only the folders above the pick, a folder click opens or closes the folder and
//! picks nothing, and a mixed value shows the mixed word.

use craft_behavior::Mixed;
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::pickers::{TreeEntry, TreePicker};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

const ARCHIVE: u32 = 1;
const Y2024: u32 = 2;
const SUMMER: u32 = 3;
const Y2025: u32 = 4;
const SPRING: u32 = 5;
const INBOX: u32 = 6;

fn albums() -> Vec<TreeEntry<u32>> {
    vec![
        TreeEntry::folder(ARCHIVE, "Archive"),
        TreeEntry::folder(Y2024, "2024").within(ARCHIVE),
        TreeEntry::new(SUMMER, "Summer").within(Y2024),
        TreeEntry::folder(Y2025, "2025").within(ARCHIVE),
        TreeEntry::new(SPRING, "Spring").within(Y2025),
        TreeEntry::new(INBOX, "Inbox"),
    ]
}

struct Rig {
    fixture: Fixture,
    value: Rc<Cell<Option<u32>>>,
}

/// A focused tree picker whose pick is `Summer`.
fn rig() -> Rig {
    let value = Rc::new(Cell::new(Some(SUMMER)));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let entries = albums();
        let mut album = shown.get();
        TreePicker::new("Album", Bind::bound(&mut album), &entries).show(ui);
        shown.set(album);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    Rig { fixture, value }
}

impl Rig {
    fn open(&mut self) {
        self.fixture.key(Key::ArrowDown, Modifiers::ALT);
        self.fixture.idle(4);
    }

    fn close(&mut self) {
        self.fixture.key(Key::Escape, Modifiers::NONE);
        self.fixture.idle(10);
    }

    /// The names of the rows the tree shows, top to bottom: the tree's node list has no visual order.
    fn listed(&self) -> Vec<String> {
        let tree = self.fixture.tree();
        let mut rows: Vec<_> =
            tree.with_role(Role::TREE_ITEM).into_iter().filter_map(|key| Some((tree.bounds_of(key)?.min.y, tree.name_of(key)?))).collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        rows.into_iter().map(|(_, name)| name.to_owned()).collect()
    }

    fn click_row(&mut self, name: &str) {
        let tree = self.fixture.tree();
        let key = tree.with_role(Role::TREE_ITEM).into_iter().find(|key| tree.name_of(*key) == Some(name));
        let target = key.and_then(|key| self.fixture.target(key));
        let target = target.unwrap_or_else(|| panic!("no row named {name}"));
        self.fixture.click(&target);
        self.fixture.idle(4);
    }
}

/// Every opening starts with only the folders above the pick open; a folder opened while browsing is closed again at
/// the next opening. A click on a folder toggles it and picks nothing.
#[trial]
fn each_opening_shows_only_the_folders_above_the_pick() {
    let mut rig = rig();
    rig.open();
    assert_eq!(rig.listed(), ["Archive", "2024", "Summer", "2025", "Inbox"]);
    rig.click_row("2025");
    assert_eq!(rig.listed(), ["Archive", "2024", "Summer", "2025", "Spring", "Inbox"], "the click opened the folder");
    assert_eq!(rig.value.get(), Some(SUMMER), "a folder click picks nothing");
    rig.close();
    rig.open();
    assert_eq!(rig.listed(), ["Archive", "2024", "Summer", "2025", "Inbox"]);
}

/// A mixed value shows the mixed word, not the placeholder of an empty pick.
#[trial]
fn a_mixed_tree_picker_shows_the_mixed_word() {
    let mut fixture = Fixture::new(|ui| {
        let entries = albums();
        let mut album = Mixed::<Option<u32>>::Mixed;
        TreePicker::new("Album", Bind::bound_mixed(&mut album), &entries).show(ui);
    });
    fixture.settle();
    let texts: Vec<String> = fixture.texts().iter().map(|text| text.text.clone()).collect();
    assert!(texts.iter().any(|text| text == "Mixed"), "painted texts: {texts:?}");
    assert!(!texts.iter().any(|text| text == "Choose…"), "painted texts: {texts:?}");
}
