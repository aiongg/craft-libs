//! The collaboration widgets through the fixture, over a scripted transport.

use std::cell::RefCell;
use std::rc::Rc;

use craft_review::collab::{CollabRequest, CollabTransport, JoinState, PEOPLE_COLORS, Participant, ParticipantId, VoteScope, VoteSession};
use craft_review::ui::{JoinField, PeopleBar, VotePill};
use craft_ui::testing::{Fixture, pointer, trial};

use crate::revisions::named;

#[derive(Clone)]
struct Scripted {
    people: Vec<Participant>,
    join: JoinState,
    vote: Option<VoteSession>,
    remove: bool,
}

impl CollabTransport for Scripted {
    fn me(&self) -> ParticipantId {
        ParticipantId(1)
    }

    fn participants(&self) -> Vec<Participant> {
        self.people.clone()
    }

    fn vote(&self) -> Option<VoteSession> {
        self.vote.clone()
    }

    fn join_state(&self) -> JoinState {
        self.join.clone()
    }

    fn can_remove(&self) -> bool {
        self.remove
    }
}

fn person(id: u64, name: &str) -> Participant {
    Participant {
        id: ParticipantId(id),
        name: name.into(),
        color: PEOPLE_COLORS[id as usize % PEOPLE_COLORS.len()],
        cursor: None,
        selection: Vec::new(),
        chat: None,
        viewer: false,
        spotlight: false,
    }
}

fn session() -> Scripted {
    Scripted { people: vec![person(1, "Ana Reyes"), person(2, "Bo Chen")], join: JoinState::Idle, vote: None, remove: false }
}

type Requests = Rc<RefCell<Vec<CollabRequest>>>;

fn bar(transport: Scripted) -> (Fixture, Requests) {
    let requests: Requests = Rc::default();
    let seen = requests.clone();
    let mut fixture = Fixture::new(move |ui| seen.borrow_mut().extend(PeopleBar::new(&transport).show(ui).requests));
    fixture.settle();
    (fixture, requests)
}

/// A click on someone follows them, on yourself toggles Spotlight.
#[trial]
fn people_bar_follows_and_spotlights() {
    let (mut fixture, requests) = bar(session());
    let bo = fixture.by_label("Bo Chen").expect("Bo's avatar");
    fixture.click(&bo);
    let me = fixture.by_label("Ana Reyes").expect("your avatar");
    fixture.click(&me);
    assert_eq!(*requests.borrow(), [CollabRequest::Follow(Some(ParticipantId(2))), CollabRequest::Spotlight(true)]);
}

/// Remove is offered in the context menu only when the transport allows it.
#[trial]
fn people_bar_removes_only_when_allowed() {
    for (allowed, expected) in [(false, 0), (true, 1)] {
        let (mut fixture, _) = bar(Scripted { remove: allowed, ..session() });
        let bo = fixture.by_label("Bo Chen").expect("Bo's avatar");
        let at = bo.center();
        fixture.frame(vec![pointer::moved(at)]);
        fixture.frame(vec![pointer::secondary(at, true)]);
        fixture.frame(vec![pointer::secondary(at, false)]);
        fixture.idle(2);
        assert_eq!(named(&fixture, "Remove Bo Chen from session").len(), expected);
    }
}

fn join(state: JoinState) -> (Fixture, Requests) {
    let requests: Requests = Rc::default();
    let seen = requests.clone();
    let transport = Scripted { join: state, ..session() };
    let mut typed = String::new();
    let mut fixture = Fixture::new(move |ui| seen.borrow_mut().extend(JoinField::new(&transport, &mut typed).show(ui).requests));
    fixture.settle();
    (fixture, requests)
}

/// A failed join shows its reason and Try Again asks to retry.
#[trial]
fn a_failed_join_reports_retry() {
    let (mut fixture, requests) = join(JoinState::Failed("No session with that code".into()));
    assert!(fixture.by_label("No session with that code").is_some());
    let retry = fixture.by_label("Try Again").expect("Try Again");
    fixture.click(&retry);
    assert_eq!(*requests.borrow(), [CollabRequest::RetryJoin]);
}

/// While a join is pending the field shows Joining… with Cancel, and Join waits.
#[trial]
fn join_field_waits_while_a_join_is_pending() {
    let (mut fixture, requests) = join(JoinState::Joining);
    assert!(fixture.by_label("Joining…").is_some());
    let cancel = fixture.by_label("Cancel").expect("Cancel");
    fixture.click(&cancel);
    assert_eq!(*requests.borrow(), [CollabRequest::CancelJoin]);
}

fn pill(vote: Option<VoteSession>) -> (Fixture, Requests) {
    let requests: Requests = Rc::default();
    let seen = requests.clone();
    let transport = Scripted { vote, ..session() };
    let mut fixture = Fixture::new(move |ui| seen.borrow_mut().extend(VotePill::new(&transport).show(ui).requests));
    fixture.settle();
    (fixture, requests)
}

/// The pill opens a session popover whose Start vote reports the draft; while a vote runs End vote ends it.
#[trial]
fn vote_pill_starts_and_ends_votes() {
    let (mut fixture, requests) = pill(None);
    let open = fixture.by_label("Voting session").expect("the session button");
    fixture.click(&open);
    let start = fixture.by_label("Start vote").expect("Start vote");
    fixture.click(&start);
    assert_eq!(*requests.borrow(), [CollabRequest::StartVote { per_person: 3, scope: VoteScope::All }]);

    let running = VoteSession { per_person: 3, scope: VoteScope::All, ended: false, used: 1, tallies: Vec::new(), has_history: false };
    let (mut fixture, requests) = pill(Some(running));
    assert!(fixture.by_label("2 votes left").is_some());
    let end = fixture.by_label("End vote").expect("End vote");
    fixture.click(&end);
    assert_eq!(*requests.borrow(), [CollabRequest::EndVote]);
}
