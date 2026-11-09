//! The revisions panel through the fixture.

use std::cell::RefCell;
use std::rc::Rc;

use craft_review::revisions::{Revision, RevisionAction, RevisionKind};
use craft_review::ui::RevisionsPanel;
use craft_ui::testing::{Fixture, Target, trial};

fn revision(kind: RevisionKind, heading: &str) -> Revision {
    Revision { kind, heading: heading.into(), details: vec!["Title text changed".into()] }
}

fn panel(revisions: Vec<Revision>) -> (Fixture, Rc<RefCell<Vec<RevisionAction>>>) {
    let actions = Rc::new(RefCell::new(Vec::new()));
    let seen = actions.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = RevisionsPanel::new("Revisions", "deck-v2", &revisions).show(ui);
        seen.borrow_mut().extend(out.actions);
    });
    fixture.settle();
    (fixture, actions)
}

/// The nodes named `label`, top to bottom.
pub(crate) fn named(fixture: &Fixture, label: &str) -> Vec<Target> {
    let tree = fixture.tree();
    let mut found: Vec<Target> =
        tree.keys().into_iter().filter(|key| tree.name_of(*key) == Some(label)).filter_map(|key| fixture.target(key)).collect();
    found.sort_by(|a, b| a.center().y.total_cmp(&b.center().y).then(a.center().x.total_cmp(&b.center().x)));
    found
}

