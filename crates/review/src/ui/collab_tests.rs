//! Collaboration widgets.

use crate::collab::{CHAT_FADE_S, CHAT_HOLD_S, chat_alpha, glide, initials};

// chat_alpha holds 5 s and fades 1 s; reduce motion ends at once.
#[test]
fn chat_fades_after_five_seconds() {
    assert_eq!(chat_alpha(0.0, false), 1.0);
    assert_eq!(chat_alpha(CHAT_HOLD_S - 0.01, false), 1.0);
    assert!((chat_alpha(CHAT_HOLD_S + CHAT_FADE_S / 2.0, false) - 0.5).abs() < 1e-6);
    assert_eq!(chat_alpha(CHAT_HOLD_S + CHAT_FADE_S, false), 0.0);

    assert_eq!(chat_alpha(CHAT_HOLD_S + CHAT_FADE_S / 2.0, true), 1.0, "no fade with reduced motion");
    assert_eq!(chat_alpha(CHAT_HOLD_S + CHAT_FADE_S, true), 0.0);
}

// A cursor closes in on its target, jumps with reduce motion; avatars prefer letters and digits.
#[test]
fn cursors_glide_and_avatars_take_initials() {
    let next = glide([0.0, 0.0], [10.0, 0.0], 0.05, false);
    assert!(next[0] > 0.0 && next[0] < 10.0 && next[1] == 0.0, "{next:?}");
    assert_eq!(glide([0.0, 0.0], [10.0, 4.0], 0.05, true), [10.0, 4.0]);
    assert_eq!(glide([0.0, 0.0], [10.0, 0.0], 5.0, false), glide([0.0, 0.0], [10.0, 0.0], 0.1, false), "dt is clamped to 0.1 s");

    assert_eq!(initials("Ada Lovelace"), "AL");
    assert_eq!(initials("Claude (agent)"), "C", "a word starting with a bracket is skipped");
    assert_eq!(initials("(x) (y)"), "((", "without letters the first characters stand in");
    assert_eq!(initials("  "), "?");
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod screens {
    use craft_ui::testing::{HarnessExt as _, symbols};
    use craft_ui::theme::{Base, install};
    use egui::accesskit::Role;
    use egui::{Key, KeyboardShortcut, Modifiers, Ui};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    use super::super::{CursorChatBox, JoinField, PeopleBar, VotePill};
    use crate::collab::{CollabRequest, CollabTransport, JoinState, Participant, ParticipantId, VoteScope, VoteSession};

    struct Session {
        participants: Vec<Participant>,
        vote: Option<VoteSession>,
        join: JoinState,
        following: Option<ParticipantId>,
        typed: String,
        chat_open: bool,
        requests: Vec<CollabRequest>,
    }

    impl CollabTransport for Session {
        fn me(&self) -> ParticipantId {
            ParticipantId(1)
        }
        fn participants(&self) -> Vec<Participant> {
            self.participants.clone()
        }
        fn vote(&self) -> Option<VoteSession> {
            self.vote.clone()
        }
        fn join_state(&self) -> JoinState {
            self.join.clone()
        }
    }

    fn person(id: u64, name: &str) -> Participant {
        Participant {
            id: ParticipantId(id),
            name: name.into(),
            color: [0x0D, 0x99, 0xFF],
            cursor: None,
            selection: Vec::new(),
            chat: None,
            viewer: false,
            spotlight: false,
        }
    }

    fn harness(show: impl FnMut(&mut Ui, &mut Session) + 'static, session: Session) -> Harness<'static, Session> {
        let mut harness = Harness::<Session>::builder()
            .with_size([480.0, 300.0])
            .with_context_setup(|ctx| {
                install(ctx, &Base::suite_default());
                symbols::install(ctx);
            })
            .build_ui_state(show, session);
        harness.run();
        harness
    }

    fn session() -> Session {
        Session {
            participants: vec![person(1, "Ana Ruiz"), person(2, "Bo Li")],
            vote: None,
            join: JoinState::Idle,
            following: None,
            typed: String::new(),
            chat_open: false,
            requests: Vec::new(),
        }
    }

    fn show_people(ui: &mut Ui, session: &mut Session) {
        let out = PeopleBar::new("people", session).following(session.following).show(ui);
        session.following = out.requests.iter().fold(session.following, |now, request| match request {
            CollabRequest::Follow(who) => *who,
            _ => now,
        });
        session.requests.extend(out.requests);
    }

    // a click on an avatar calls follow, on yours spotlight.
    #[test]
    fn people_bar_follows_and_spotlights() {
        let mut harness = harness(show_people, session());
        harness.get_by_role_and_label(Role::Button, "Follow Bo Li").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::Follow(Some(ParticipantId(2)))]);

        harness.state_mut().requests.clear();
        harness.get_by_role_and_label(Role::Button, "Stop following").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::Follow(None)]);

        harness.state_mut().requests.clear();
        harness.get_by_role_and_label(Role::Button, "Ana Ruiz (you)").click();
        harness.run();
        harness.get_by_label("Spotlight me").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::Spotlight(true)]);
    }

    fn show_vote(ui: &mut Ui, session: &mut Session) {
        let out = VotePill::new("vote", session).show(ui);
        session.requests.extend(out.requests);
    }

    // Start vote and End vote call the transport.
    #[test]
    fn vote_pill_starts_and_ends_votes() {
        let mut harness = harness(show_vote, session());
        harness.get_by_role_and_label(Role::Button, "Voting session").click();
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Start vote").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::StartVote { per_person: 3, scope: VoteScope::All }]);

        harness.state_mut().requests.clear();
        harness.state_mut().vote = Some(VoteSession { per_person: 3, used: 1, ..VoteSession::default() });
        harness.run();
        harness.get_by_label("2 votes left");
        harness.get_by_role_and_label(Role::Button, "End vote").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::EndVote]);
    }

    fn show_join(ui: &mut Ui, session: &mut Session) {
        let mut typed = std::mem::take(&mut session.typed);
        let out = JoinField::new("join", session, &mut typed).show(ui);
        session.typed = typed;
        session.requests.extend(out.requests);
    }

    // Join is disabled while pending; Enter joins and clears.
    #[test]
    fn join_field_waits_while_a_join_is_pending() {
        let mut harness = harness(show_join, Session { join: JoinState::Joining, typed: "ABC-123".into(), ..session() });
        assert!(harness.get_by_role_and_label(Role::Button, "Join").accesskit_node().is_disabled());
        harness.get_by_role_and_label(Role::Button, "Cancel").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::CancelJoin]);

        harness.state_mut().requests.clear();
        harness.state_mut().join = JoinState::Idle;
        harness.run();
        harness.get_by_role_and_label(Role::TextInput, "Join code or invite link").click();
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
        assert_eq!(harness.state().requests, [CollabRequest::Join("ABC-123".into())]);
        assert!(harness.state().typed.is_empty());
    }

    fn show_chat(ui: &mut Ui, session: &mut Session) {
        let mut typed = std::mem::take(&mut session.typed);
        let mut open = session.chat_open;
        let out = CursorChatBox::new("chat", &mut typed, &mut open).show(ui, Some(egui::pos2(60.0, 60.0)));
        session.typed = typed;
        session.chat_open = open;
        session.requests.extend(out.requests);
    }

    // `/` opens the chat at the pointer, typing sends each change, Esc closes it with an empty chat.
    #[test]
    fn cursor_chat_opens_with_slash_and_closes_with_escape() {
        let mut harness = harness(show_chat, session());
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Slash));
        harness.run();
        assert!(harness.state().chat_open);
        harness.type_text("hi");
        assert_eq!(harness.state().typed, "hi");
        assert_eq!(harness.state().requests.last(), Some(&CollabRequest::Chat("hi".into())));

        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
        assert!(!harness.state().chat_open);
        assert_eq!(harness.state().requests.last(), Some(&CollabRequest::Chat(String::new())));
    }
}
