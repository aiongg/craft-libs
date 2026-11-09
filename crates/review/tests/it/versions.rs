//! The version history panel through the fixture.

use std::cell::RefCell;
use std::rc::Rc;

use craft_review::ui::{VersionAction, VersionHistoryPanel, VersionPreviewBanner};
use craft_review::versions::{Branch, BranchId, BranchState, Version, VersionId};
use craft_ui::contract::Role;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Rect, vec2};

const NOW: i64 = 10_000;

fn version(id: u64, title: &str, autosave: bool) -> Version {
    Version {
        id: VersionId(id),
        title: title.into(),
        description: String::new(),
        created: i64::try_from(id).unwrap_or(0) * 1_000,
        author: "Ana".into(),
        autosave,
    }
}

/// The history row whose name starts with `title`: a row's name carries its date and author after the title.
fn row_named(fixture: &Fixture, title: &str) -> Target {
    let tree = fixture.tree();
    let found = [Role::TREE_ITEM, Role::LIST_ITEM]
        .into_iter()
        .flat_map(|role| fixture.by_role(role))
        .find(|row| tree.name_of(row.node).is_some_and(|name| name.starts_with(title)));
    found.unwrap_or_else(|| panic!("no row `{title}`"))
}

fn panel() -> (Fixture, Rc<RefCell<Vec<VersionAction>>>) {
    let actions = Rc::new(RefCell::new(Vec::new()));
    let seen = actions.clone();
    let versions = vec![version(1, "Draft", false), version(2, "", true), version(3, "", true), version(4, "Final", false)];
    let branches = vec![Branch { id: BranchId(1), name: "Dark mode".into(), state: BranchState::Merged, created: 500, author: "Bo".into() }];
    let mut fixture = Fixture::new(move |ui| seen.borrow_mut().extend(VersionHistoryPanel::new(&versions, &branches, NOW).show(ui).actions));
    fixture.room(vec2(360.0, 700.0));
    fixture.settle();
    (fixture, actions)
}

/// Save version opens a form; Title, a description and Enter report Save.
#[trial]
fn save_form_reports_title_and_description() {
    let (mut fixture, actions) = panel();
    let save = fixture.by_label("Save version").expect("Save version");
    fixture.click(&save);
    let title = fixture.by_label("Title").expect("the title field");
    fixture.click(&title);
    fixture.type_text("Beta");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(matches!(actions.borrow().as_slice(), [VersionAction::Save { title, description, .. }] if title == "Beta" && description.is_empty()));
}

/// Create branch opens a name field; Enter creates the branch.
#[trial]
fn the_branch_form_creates_a_branch() {
    let (mut fixture, actions) = panel();
    let create = fixture.by_label("Create branch").expect("Create branch");
    fixture.click(&create);
    let name = fixture.by_label("Branch name").expect("the name field");
    fixture.click(&name);
    fixture.type_text("Mobile");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(matches!(actions.borrow().as_slice(), [VersionAction::CreateBranch { name, .. }] if name == "Mobile"));
}

/// A merged branch names its state and has a menu.
#[trial]
fn a_branch_row_names_its_state_and_has_a_menu() {
    let (mut fixture, actions) = panel();
    assert!(fixture.by_label("Dark mode").is_some());
    let more = fixture.by_label("More (Dark mode)").expect("the branch menu button");
    fixture.click(&more);
    let open = fixture.by_label("Open").expect("Open");
    fixture.click(&open);
    assert_eq!(*actions.borrow(), [VersionAction::OpenBranch(BranchId(1))]);
}

/// The "More" button of the row named `row`: the part of that row, not the first node of that name.
fn more_of(fixture: &Fixture, row: &str) -> Target {
    let row = row_named(fixture, row);
    let tree = fixture.tree();
    let key =
        tree.children_of(row.node).into_iter().find(|key| tree.name_of(*key) == Some("More")).unwrap_or_else(|| panic!("the row has no More part"));
    fixture.target(key).expect("the part has bounds")
}

fn pick(fixture: &mut Fixture, row: &str, item: &str) {
    let more = more_of(fixture, row);
    fixture.click(&more);
    let entry = fixture.by_label(item).unwrap_or_else(|| panic!("no menu item `{item}`"));
    fixture.click(&entry);
}

/// A saved version's menu offers its entries and acts on that row.
#[trial]
fn a_row_menu_acts_on_its_own_row() {
    let (mut fixture, actions) = panel();
    let more = more_of(&fixture, "Final");
    fixture.click(&more);
    for item in ["Restore this version", "Rename", "Edit description", "Duplicate", "Delete"] {
        assert!(fixture.by_label(item).is_some(), "the menu offers `{item}`");
    }
    let duplicate = fixture.by_label("Duplicate").expect("Duplicate");
    fixture.click(&duplicate);
    pick(&mut fixture, "Draft", "Restore this version");
    assert_eq!(*actions.borrow(), [VersionAction::Duplicate(VersionId(4)), VersionAction::Restore(VersionId(1))]);
}

/// Rename in the menu edits the title in the row and Enter reports it with the description kept.
#[trial]
fn rename_edits_the_title_in_place() {
    let (mut fixture, actions) = panel();
    pick(&mut fixture, "Final", "Rename");
    fixture.settle();
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text("Release");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(matches!(
        actions.borrow().as_slice(),
        [VersionAction::Rename { id: VersionId(4), title, description, .. }] if title == "Release" && description.is_empty()
    ));
}

/// Edit description opens the form with the version's text; Enter reports title and description.
#[trial]
fn edit_description_reports_the_new_text() {
    let mut final_version = version(4, "Final", false);
    final_version.description = "old".into();
    let actions = Rc::new(RefCell::new(Vec::new()));
    let seen = actions.clone();
    let versions = [final_version];
    let mut fixture = Fixture::new(move |ui| seen.borrow_mut().extend(VersionHistoryPanel::new(&versions, &[], NOW).show(ui).actions));
    fixture.settle();
    pick(&mut fixture, "Final", "Edit description");
    let field = fixture.by_label("Description").expect("the description field");
    fixture.click(&field);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text("new words");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(matches!(
        actions.borrow().as_slice(),
        [VersionAction::Rename { id: VersionId(4), title, description, .. }] if title == "Final" && description == "new words"
    ));
}

/// Delete turns the row into a confirmation; Cancel restores the row and reports nothing, the
/// Delete button there reports the delete.
#[trial]
fn delete_asks_in_the_row_and_confirms() {
    let (mut fixture, actions) = panel();
    pick(&mut fixture, "Final", "Delete");
    let cancel = fixture.by_label("Cancel").expect("Cancel");
    fixture.click(&cancel);
    assert!(actions.borrow().is_empty());
    assert!(fixture.by_label("Delete \u{201c}Final\u{201d}? This can't be undone.").is_none());
    assert!(fixture.by_role(Role::TREE_ITEM).iter().any(|row| fixture.tree().name_of(row.node).is_some_and(|name| name.starts_with("Final"))));

    pick(&mut fixture, "Final", "Delete");
    assert!(actions.borrow().is_empty(), "the menu item only asks");
    assert!(fixture.by_label("Delete \u{201c}Final\u{201d}? This can't be undone.").is_some());
    let confirm = fixture.by_label("Delete").expect("the confirming Delete");
    fixture.click(&confirm);
    assert_eq!(*actions.borrow(), [VersionAction::Delete(VersionId(4))]);
    assert!(fixture.by_label("Delete \u{201c}Final\u{201d}? This can't be undone.").is_none());
}

/// A run of autosaves is one row that counts them and opens and closes on a click.
#[trial]
fn autosaves_fold_into_a_group_that_opens() {
    let (mut fixture, _) = panel();
    let rows = |fixture: &Fixture| fixture.by_role(Role::TREE_ITEM).len();
    let folded = rows(&fixture);
    let group = fixture.by_label("2 autosave versions").expect("the group row counts its autosaves");
    fixture.click(&group);
    assert_eq!(rows(&fixture), folded + 2, "opening shows the autosaves");
    let group = fixture.by_label("2 autosave versions").expect("the group row");
    fixture.click(&group);
    assert_eq!(rows(&fixture), folded, "a second click folds them again");
}

/// Escape on the preview banner goes back to the current version.
#[trial]
fn escape_leaves_the_preview() {
    let actions = Rc::new(RefCell::new(Vec::new()));
    let seen = actions.clone();
    let shown = version(4, "Final", false);
    let mut fixture = Fixture::new(move |ui| {
        let canvas = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(600.0, 400.0));
        seen.borrow_mut().extend(VersionPreviewBanner::new(&shown, NOW).show(ui, canvas).action);
    });
    fixture.settle();
    assert!(fixture.by_label("Back to current version").is_some());
    fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(*actions.borrow(), [VersionAction::Current]);
}

/// A long description wraps under the title instead of running off the row.
#[trial]
fn a_long_description_wraps_under_the_title() {
    let long = "A long note about what changed in this version, written so that it cannot fit on one line of a narrow panel.";
    let mut draft = version(1, "Draft", false);
    draft.description = long.into();
    let mut fixture = Fixture::new(move |ui| {
        let versions = [draft.clone()];
        let _ = VersionHistoryPanel::new(&versions, &[], NOW).show(ui);
    });
    fixture.room(vec2(260.0, 500.0));
    fixture.settle();
    let texts = fixture.texts();
    let title = texts.iter().find(|painted| painted.text == "Draft").expect("the title is painted");
    let note = texts.iter().find(|painted| painted.text == long).expect("the description is painted whole");
    assert!(!note.elided, "the description wraps, it is not cut");
    assert!(note.rect.min.y >= title.rect.min.y, "the description is under the title");
}
