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

// The ink on a person's colour follows FigCraft's luminance rule: dark above 170, else light.
#[test]
fn ink_on_a_person_colour_follows_the_luminance_rule() {
    use craft_ui::theme::{Base, current, install};
    use egui::Color32;

    use super::presence::ink_on;
    use super::style::ReviewStyle;

    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let style = ReviewStyle::of(&current(&ctx));
    assert_ne!(style.ink_dark, style.ink_light);
    assert_eq!(ink_on(&style, Color32::from_rgb(0xF2, 0xC9, 0x4C)), style.ink_dark, "a light yellow gets the dark ink");
    assert_eq!(ink_on(&style, Color32::from_rgb(0x0D, 0x99, 0xFF)), style.ink_light, "luminance 123 stays light");
    assert_eq!(ink_on(&style, Color32::from_rgb(170, 170, 170)), style.ink_light, "170 is not above 170");
    assert_eq!(ink_on(&style, Color32::from_rgb(171, 171, 171)), style.ink_dark);
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod screens {
    use craft_ui::testing::{HarnessExt as _, symbols};
    use craft_ui::theme::{Base, install};
    use egui::accesskit::Role;
    use egui::{Color32, Key, KeyboardShortcut, Modifiers, Rect, Shape, Ui};
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
        can_remove: bool,
        has_selection: bool,
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
        fn can_remove(&self) -> bool {
            self.can_remove
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
            can_remove: false,
            has_selection: false,
        }
    }

    const CANVAS: Rect = Rect::from_min_max(egui::pos2(20.0, 100.0), egui::pos2(440.0, 280.0));

    fn show_people(ui: &mut Ui, session: &mut Session) {
        let out = PeopleBar::new("people", session).following(session.following).show(ui, CANVAS);
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

    /// Every rectangle with a visible stroke or fill in `out`, as (rect, stroke colour, fill).
    fn rects(out: &egui::FullOutput) -> Vec<(Rect, Color32, Color32)> {
        fn walk(shape: &Shape, found: &mut Vec<(Rect, Color32, Color32)>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, found)),
                Shape::Rect(rect) => found.push((rect.rect, rect.stroke.color, rect.fill)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        out.shapes.iter().for_each(|clipped| walk(&clipped.shape, &mut found));
        found
    }

    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, mut body: impl FnMut(&mut Ui)) -> egui::FullOutput {
        let input =
            egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(480.0, 300.0))), events, ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| body(ui));
        out.textures_delta.clear();
        out
    }

    fn key_press(key: Key) -> egui::Event {
        egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: Modifiers::NONE }
    }

    // Esc stops following; the frame is drawn round the canvas the host passed, in the person's colour.
    #[test]
    fn escape_stops_following_and_the_frame_hugs_the_canvas() {
        let mut harness = harness(show_people, Session { following: Some(ParticipantId(2)), ..session() });
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
        assert_eq!(harness.state().requests, [CollabRequest::Follow(None)]);
        assert_eq!(harness.state().following, None);

        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        let session = Session { following: Some(ParticipantId(2)), ..session() };
        let out = frame(&ctx, Vec::new(), |ui| {
            let _ = PeopleBar::new("people", &session).following(session.following).show(ui, CANVAS);
        });
        let blue = Color32::from_rgb(0x0D, 0x99, 0xFF);
        assert!(rects(&out).iter().any(|(rect, stroke, _)| *rect == CANVAS.shrink(1.0) && *stroke == blue), "frame round the canvas");
    }

    // Your own bubble is drawn in your colour.
    #[test]
    fn your_chat_bubble_takes_your_colour() {
        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        let (mut typed, mut open) = (String::new(), false);
        let color = [0xF2, 0x48, 0x22];
        let mut run = |events| {
            frame(&ctx, events, |ui| {
                let _ = CursorChatBox::new("chat", &mut typed, &mut open, color).show(ui, Some(egui::pos2(60.0, 60.0)));
            })
        };
        let _ = run(vec![key_press(Key::Slash)]);
        let out = run(Vec::new());
        // The area fades in, so the fill is the colour at some opacity.
        let person = Color32::from_rgb(0xF2, 0x48, 0x22);
        let close = |fill: Color32| {
            let want = person.gamma_multiply(f32::from(fill.a()) / 255.0);
            fill.a() > 0 && fill.to_array().iter().zip(want.to_array()).all(|(a, b)| a.abs_diff(b) <= 2)
        };
        assert!(rects(&out).iter().any(|(_, _, fill)| close(*fill)), "the box is filled in the person's colour: {:?}", rects(&out));
    }

    // Right-click on someone offers Remove only when the transport allows it.
    #[test]
    fn people_bar_removes_only_when_allowed() {
        let mut harness = harness(show_people, Session { can_remove: true, ..session() });
        harness.get_by_role_and_label(Role::Button, "Follow Bo Li").click_secondary();
        harness.run();
        harness.get_by_label("Remove Bo Li from session").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::Remove(ParticipantId(2))]);

        let mut harness = self::harness(show_people, session());
        harness.get_by_role_and_label(Role::Button, "Follow Bo Li").click_secondary();
        harness.run();
        assert!(harness.query_by_label("Remove Bo Li from session").is_none());
    }

    // The chevron lists you first and everyone else, marks who is followed, and says when nobody
    // else is here; choosing someone follows them.
    #[test]
    fn people_menu_lists_everyone_and_the_empty_note() {
        let mut harness = harness(show_people, Session { following: Some(ParticipantId(2)), ..session() });
        harness.get_by_role_and_label(Role::Button, "People in this file").click();
        harness.run();
        assert_eq!(harness.get_all_by_label("Ana Ruiz (you)").count(), 2, "the avatar and the menu row");
        let row = harness.get_by_label("Bo Li");
        assert_eq!(row.accesskit_node().description().as_deref(), Some("Following"));
        row.click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::Follow(None)], "choosing the followed person stops following");

        let mut harness = self::harness(show_people, Session { participants: vec![person(1, "Ana Ruiz")], ..session() });
        harness.get_by_role_and_label(Role::Button, "People in this file").click();
        harness.run();
        harness.get_by_label("Nobody else is here yet");

        let mut harness = self::harness(show_people, Session { participants: Vec::new(), ..session() });
        harness.get_by_role_and_label(Role::Button, "People in this file").click();
        harness.run();
        harness.get_by_label("Not in a live session");
    }

    // Try Again asks the app to retry; the field keeps no copy of the code.
    #[test]
    fn a_failed_join_reports_retry() {
        let mut harness = harness(show_join, Session { join: JoinState::Failed("No such session".into()), ..session() });
        harness.get_by_role_and_label(Role::Button, "Try Again").click();
        harness.run();
        assert_eq!(harness.state().requests, [CollabRequest::RetryJoin]);
    }

    fn show_vote(ui: &mut Ui, session: &mut Session) {
        let out = VotePill::new("vote", session).has_selection(session.has_selection).show(ui);
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

    // Vote on offers Selection only while something is selected.
    #[test]
    fn vote_scope_offers_selection_only_with_a_selection() {
        for (has_selection, offered) in [(false, false), (true, true)] {
            let mut harness = harness(show_vote, Session { has_selection, ..session() });
            harness.get_by_role_and_label(Role::Button, "Voting session").click();
            harness.run();
            harness.get_by_role_and_label(Role::ComboBox, "Vote on").click();
            harness.run();
            assert!(harness.query_by_label("Sticky notes").is_some(), "the other scopes are there");
            assert_eq!(harness.query_by_label("Selection").is_some(), offered, "has_selection {has_selection}");
        }
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
        harness.get_by_role_and_label(Role::PasswordInput, "Join code or invite link").click();
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
        assert_eq!(harness.state().requests, [CollabRequest::Join("ABC-123".into())]);
        assert!(harness.state().typed.is_empty());
    }

    // The join field masks what is typed and keeps it out of the accessibility tree.
    #[test]
    fn join_field_hides_the_code_from_accesskit() {
        let harness = harness(show_join, Session { typed: "SECRET-CODE".into(), ..session() });
        let node = harness.get_by_role_and_label(Role::PasswordInput, "Join code or invite link");
        let value = format!("{:?}", node.accesskit_node().value());
        assert!(!value.contains("SECRET"), "{value}");
    }

    fn show_chat(ui: &mut Ui, session: &mut Session) {
        let mut typed = std::mem::take(&mut session.typed);
        let mut open = session.chat_open;
        let out = CursorChatBox::new("chat", &mut typed, &mut open, [0x0D, 0x99, 0xFF]).show(ui, Some(egui::pos2(60.0, 60.0)));
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
