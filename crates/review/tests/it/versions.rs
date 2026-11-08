//! The version history panel through the fixture.

use std::cell::RefCell;
use std::rc::Rc;

use craft_review::ui::{VersionAction, VersionHistoryPanel};
use craft_review::versions::{Branch, BranchId, BranchState, Version, VersionId};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

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

fn panel() -> (Fixture, Rc<RefCell<Vec<VersionAction>>>) {
    let actions = Rc::new(RefCell::new(Vec::new()));
    let seen = actions.clone();
    let versions = vec![version(1, "Draft", false), version(2, "", true), version(3, "", true), version(4, "Final", false)];
    let branches = vec![Branch { id: BranchId(1), name: "Dark mode".into(), state: BranchState::Merged, created: 500, author: "Bo".into() }];
    let mut fixture = Fixture::new(move |ui| seen.borrow_mut().extend(VersionHistoryPanel::new(&versions, &branches, NOW).show(ui).actions));
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
    assert_eq!(*actions.borrow(), [VersionAction::Save { title: "Beta".into(), description: String::new() }]);
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
    assert_eq!(*actions.borrow(), [VersionAction::CreateBranch { name: "Mobile".into() }]);
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
