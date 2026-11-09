//! The collaboration widgets through the fixture, over a scripted transport.

use std::cell::RefCell;
use std::rc::Rc;

use craft_review::collab::{CollabRequest, CollabTransport, JoinState, PEOPLE_COLORS, Participant, ParticipantId, VoteScope, VoteSession};
use craft_review::ui::{CursorChatBox, JoinField, PeopleBar, ReviewCommand, VotePill};
use craft_theme::{contrast_ratio, text_contrast_min};
use craft_ui::canvas::{Canvas, ToolHost};
use craft_ui::contract::Show as _;
use craft_ui::contract::{Commands as _, ShowContainer as _};
use craft_ui::layout_components::Stack;
use craft_ui::testing::{Fixture, Frames, pointer, trial};

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
    for allowed in [false, true] {
        let (mut fixture, _) = bar(Scripted { remove: allowed, ..session() });
        let bo = fixture.by_label("Bo Chen").expect("Bo's avatar");
        let at = bo.center();
        fixture.frame(vec![pointer::moved(at)]);
        fixture.frame(vec![pointer::secondary(at, true)]);
        fixture.frame(vec![pointer::secondary(at, false)]);
        fixture.idle(2);
        assert_eq!(!named(&fixture, "Remove Bo Chen from session").is_empty(), allowed);
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

/// Vote on offers Selection only while something is selected.
#[trial]
fn vote_scope_offers_selection_only_with_a_selection() {
    for (has_selection, offered) in [(false, false), (true, true)] {
        let transport = session();
        let mut fixture = Fixture::new(move |ui| {
            let _ = VotePill::new(&transport).has_selection(has_selection).show(ui);
        });
        fixture.settle();
        let open = fixture.by_label("Voting session").expect("the session button");
        fixture.click(&open);
        let scope = fixture.by_label("Vote on").expect("the scope field");
        fixture.click(&scope);
        assert!(fixture.by_label("Sticky notes").is_some(), "the other scopes are there");
        assert_eq!(fixture.by_label("Selection").is_some(), offered, "has_selection {has_selection}");
    }
}

/// The cursor chat command opens the cursor chat; typing in it sends each change and Escape closes it with an empty chat.
#[trial]
fn cursor_chat_sends_each_change_and_closes_with_escape() {
    let requests: Requests = Rc::default();
    let seen = requests.clone();
    let state = Rc::new(RefCell::new((String::new(), false)));
    let shared = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut shared = shared.borrow_mut();
        let (mut text, mut open) = std::mem::take(&mut *shared);
        Stack::new().command_scope(craft_ui::runtime::commands::CommandScope::APP).show(ui, |ui, cx| {
            while cx.take_command::<ReviewCommand>().is_some() {
                open = true;
            }
            let out = CursorChatBox::new(&mut text, &mut open).show(ui, Some(egui::pos2(60.0, 60.0)));
            seen.borrow_mut().extend(out.requests);
        });
        *shared = (text, open);
    });
    fixture.settle();
    assert!(!state.borrow().1, "the box is closed until the command runs");
    craft_ui::runtime::commands::fire(fixture.ctx(), ReviewCommand::CursorChat, craft_ui::runtime::commands::FireSource::Key);
    fixture.settle();
    assert!(state.borrow().1, "the / command opens the box");
    fixture.type_text("hi");
    assert_eq!(state.borrow().0, "hi");
    assert_eq!(requests.borrow().last(), Some(&CollabRequest::Chat("hi".into())));
    fixture.key(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!state.borrow().1);
    assert_eq!(requests.borrow().last(), Some(&CollabRequest::Chat(String::new())));
}

/// The colour of the first painted text of `show` after it settles.
fn first_ink(show: &mut dyn FnMut(&mut egui::Ui)) -> egui::Color32 {
    let mut frames = Frames::new();
    frames.idle(2, &mut *show);
    frames
        .frame_shapes(&mut *show)
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) => text.galley.job.sections.first().map(|section| section.format.color),
            _ => None,
        })
        .expect("a text is painted")
}

/// The initials of an avatar and the name tag at a remote cursor read on every colour a person can take.
#[trial]
fn initials_and_name_tags_read_on_every_people_colour() {
    let least = text_contrast_min(craft_theme::Contrast::Standard);
    for color in PEOPLE_COLORS {
        let fill = craft_theme::Rgba::rgb(color[0], color[1], color[2]);
        let mut avatar = move |ui: &mut egui::Ui| {
            let _ = craft_review::ui::Avatar::new("Ana Reyes").fill(color).show(ui);
        };
        let ratio = contrast_ratio(craft_theme::Rgba::from(first_ink(&mut avatar)), fill);
        assert!(ratio >= least, "the initials read on {color:?}: {ratio} against {least}");

        let mut room = Scripted { people: vec![person(1, "Ana Reyes"), person(2, "Bo Chen")], ..session() };
        room.people[1].color = color;
        room.people[1].cursor = Some([60.0, 60.0]);
        let mut tag = move |ui: &mut egui::Ui| {
            let area = ui.available_rect_before_wrap();
            let view = craft_behavior::CanvasView::new(area, kurbo::Point::new(0.0, 0.0), 1.0);
            Canvas::new("Document").view(view).show(ui, |ui, cx| {
                craft_review::ui::RemoteCursors::new("Cursors", &room).show_at(ui, cx.rect);
            });
        };
        let ratio = contrast_ratio(craft_theme::Rgba::from(first_ink(&mut tag)), fill);
        assert!(ratio >= least, "the name tag reads on {color:?}: {ratio} against {least}");
    }
}

/// Escape while following stops following.
#[trial]
fn escape_while_following_stops_following() {
    let following = Rc::new(RefCell::new(Some(ParticipantId(2))));
    let shared = following.clone();
    let mut fixture = Fixture::new(move |ui| {
        let area = ui.available_rect_before_wrap();
        let view = craft_behavior::CanvasView::new(area, kurbo::Point::new(0.0, 0.0), 1.0);
        let mut shared = shared.borrow_mut();
        let on = shared.is_some();
        let canvas = |ui: &mut egui::Ui| {
            Canvas::new("Document").view(view).show(ui, |_, _| {});
        };
        if on {
            if craft_review::ui::FollowFrame::new("Following", PEOPLE_COLORS[1], "Following Bo Chen").show(ui, |ui, _| canvas(ui)).output.stopped {
                *shared = None;
            }
        } else {
            canvas(ui);
        }
    });
    fixture.settle();
    assert!(following.borrow().is_some());
    craft_ui::runtime::commands::fire(fixture.ctx(), craft_review::ui::FollowCommand::StopFollowing, craft_ui::runtime::commands::FireSource::Key);
    fixture.settle();
    assert!(following.borrow().is_none(), "the stop command stops following");
}

/// Escape, the command's default binding, reaches the follow frame while it shows.
#[trial]
fn escape_key_stops_following() {
    let following = Rc::new(RefCell::new(Some(ParticipantId(2))));
    let shared = following.clone();
    let mut fixture = Fixture::new(move |ui| {
        let area = ui.available_rect_before_wrap();
        let view = craft_behavior::CanvasView::new(area, kurbo::Point::new(0.0, 0.0), 1.0);
        let mut shared = shared.borrow_mut();
        let on = shared.is_some();
        let canvas = |ui: &mut egui::Ui| {
            Canvas::new("Document").view(view).show(ui, |_, _| {});
        };
        if on {
            if craft_review::ui::FollowFrame::new("Following", PEOPLE_COLORS[1], "Following Bo Chen").show(ui, |ui, _| canvas(ui)).output.stopped {
                *shared = None;
            }
        } else {
            canvas(ui);
        }
    });
    fixture.settle();
    fixture.key(egui::Key::Escape, egui::Modifiers::NONE);
    fixture.settle();
    assert!(following.borrow().is_none(), "Escape stops following");
}

/// A canvas tool with an edit under way: Escape cancels the edit first.
struct Editing(Rc<std::cell::Cell<bool>>);

impl craft_ui::canvas::CanvasTool for Editing {
    fn press(&mut self, _: &craft_ui::canvas::ToolPointer, _: &mut craft_ui::canvas::ToolContext<'_>) -> craft_ui::canvas::ToolAction {
        craft_ui::canvas::ToolAction::Ignored
    }

    fn drag(&mut self, _: &craft_ui::canvas::ToolPointer, _: &mut craft_ui::canvas::ToolContext<'_>) {}

    fn release(&mut self, _: &craft_ui::canvas::ToolPointer, _: &mut craft_ui::canvas::ToolContext<'_>) {}

    fn cancel(&mut self, _: craft_behavior::CancelReason, _: &mut craft_ui::canvas::ToolContext<'_>) {}

    fn key(&mut self, key: &craft_ui::canvas::ToolKey, _: &mut craft_ui::canvas::ToolContext<'_>) -> bool {
        matches!(key, craft_ui::canvas::ToolKey::Cancel) && self.0.replace(false)
    }

    fn paint(&self, _: &craft_ui::primitives::Draw<'_>, _: &craft_ui::canvas::ToolContext<'_>) {}

    fn command_form(&self) -> craft_ui::canvas::ToolPath {
        craft_ui::canvas::ToolPath::Freehand("the test tool has no keyboard path")
    }
}

/// With nothing focused, Escape cancels a tool's pending edit first and stops following the second time.
#[trial]
fn escape_cancels_a_pending_edit_before_it_stops_following() {
    let following = Rc::new(RefCell::new(Some(ParticipantId(2))));
    let shared = following.clone();
    let pending = Rc::new(std::cell::Cell::new(true));
    let edit = pending.clone();
    let mut fixture = Fixture::new(move |ui| {
        let area = ui.available_rect_before_wrap();
        let view = craft_behavior::CanvasView::new(area, kurbo::Point::new(0.0, 0.0), 1.0);
        let mut shared = shared.borrow_mut();
        if shared.is_none() {
            return;
        }
        let stopped = craft_review::ui::FollowFrame::new("Following", PEOPLE_COLORS[1], "Following Bo Chen")
            .show(ui, |ui, _| {
                let shown = Canvas::new("Document").view(view).show(ui, |_, _| {});
                let _ = ToolHost::new().run(ui, &shown.output, &mut Editing(edit.clone()));
            })
            .output
            .stopped;
        if stopped {
            *shared = None;
        }
    });
    fixture.settle();
    fixture.key(egui::Key::Escape, egui::Modifiers::NONE);
    fixture.settle();
    assert!(!pending.get(), "the first Escape cancels the edit");
    assert!(following.borrow().is_some(), "the first Escape leaves following on");
    fixture.key(egui::Key::Escape, egui::Modifiers::NONE);
    fixture.settle();
    assert!(following.borrow().is_none(), "the second Escape stops following");
}
