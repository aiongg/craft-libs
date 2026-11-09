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

/// The per-row buttons and the All buttons report their actions; each card names its kind.
#[trial]
fn accept_and_reject_report_their_row() {
    let (mut fixture, actions) = panel(vec![revision(RevisionKind::Changed, "Slide 3 changed"), revision(RevisionKind::Deleted, "Slide 4 deleted")]);
    assert!(fixture.by_label("Accept All (2)").is_some());
    assert!(fixture.by_label("Changed").is_some(), "a card names its kind");
    assert!(fixture.by_label("Deleted").is_some());
    let accept = named(&fixture, "Accept")[1].clone();
    fixture.click(&accept);
    let reject = named(&fixture, "Reject")[0].clone();
    fixture.click(&reject);
    let go = fixture.by_label("Slide 3 changed").expect("the heading link");
    fixture.click(&go);
    let all = fixture.by_label("Reject All (2)").expect("Reject All");
    fixture.click(&all);
    assert_eq!(*actions.borrow(), [RevisionAction::Accept(1), RevisionAction::Reject(0), RevisionAction::Go(0), RevisionAction::RejectAll]);
}

/// An empty list shows End Review.
#[trial]
fn no_changes_offers_end_review() {
    let (mut fixture, actions) = panel(Vec::new());
    assert!(fixture.by_label("No differences left.").is_some());
    let end = fixture.by_label("End Review").expect("End Review");
    fixture.click(&end);
    assert_eq!(*actions.borrow(), [RevisionAction::EndReview]);
}
