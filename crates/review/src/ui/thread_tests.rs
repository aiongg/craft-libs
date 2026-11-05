//! Pins, popover and composer.

use egui::{Pos2, pos2};

use super::thread::{cluster, insert_mention, mention_matches, mention_query};

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

// cluster groups by the running centre.
#[test]
fn cluster_joins_points_near_a_group_centre() {
    let points: [Pos2; 4] = [pos2(0.0, 0.0), pos2(20.0, 0.0), pos2(40.0, 0.0), pos2(200.0, 0.0)];
    // 20 joins the group at 0 (centre 0, distance 20 < 24); 40 is 30 from the centre 10.
    assert_eq!(cluster(&points, 24.0), vec![vec![0, 1], vec![2], vec![3]]);
    assert_eq!(cluster(&points, 1.0), vec![vec![0], vec![1], vec![2], vec![3]]);
    assert!(cluster(&[], 24.0).is_empty());
}

// mention_query ranges and mention_matches prefix rules.
#[test]
fn mention_query_and_matches() {
    assert_eq!(mention_query("hi @An", 6), Some((3, 6)));
    assert_eq!(mention_query("@", 1), Some((0, 1)));
    assert_eq!(mention_query("mail@An", 7), None, "an @ after a letter is no mention");
    assert_eq!(mention_query("hi @An ", 7), None, "the query doesn't end in a space");
    assert_eq!(mention_query("hi @a\nb", 7), None, "the query stays on one line");
    assert_eq!(mention_query("héé @Zoë", 8), Some((6, 11)), "the cursor counts chars, the range bytes");
    assert_eq!(mention_query(&format!("@{}", "x".repeat(41)), 42), None, "at most 40 chars");

    let people = names(&["Ana María", "Bo", "Me", "ana", "Mario Ruiz", "C1", "C2", "C3", "C4"]);
    assert_eq!(mention_matches(&people, "mar", "Me"), ["Ana María", "Mario Ruiz"], "a word of the name, any case");
    assert_eq!(mention_matches(&people, "m", "Me"), ["Ana María", "Mario Ruiz"], "never me");
    assert_eq!(mention_matches(&people, "", "Me").len(), 6, "six rows at most");
}

// insert_mention replaces the query and returns the char cursor.
#[test]
fn insert_mention_returns_the_cursor() {
    let mut text = "ça @An now".to_owned();
    let cursor = insert_mention(&mut text, (4, 7), "Ana María");
    assert_eq!(text, "ça @Ana María  now");
    assert_eq!(cursor, "ça @Ana María ".chars().count());
    let mut same = "abc".to_owned();
    assert_eq!(insert_mention(&mut same, (2, 9), "x"), 3);
    assert_eq!(insert_mention(&mut same, (2, 1), "x"), 3);
    assert_eq!(same, "abc");
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod screens {
    use craft_ui::testing::{HarnessExt as _, symbols};
    use craft_ui::theme::{Base, install};
    use egui::accesskit::Role;
    use egui::{Key, KeyboardShortcut, Modifiers, Rect, Ui, pos2, vec2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    use super::super::comments::CommentEdit;
    use super::super::thread::{CommentComposer, CommentPin, CommentPins, PinMove, ThreadPopover};
    use crate::comments::{CommentAnchor, CommentMessage, CommentThread, ReviewStatus, ThreadId};
    use craft_ui::edit::EditEvent;

    fn harness<S>(show: impl FnMut(&mut Ui, &mut S) + 'static, state: S) -> Harness<'static, S> {
        let mut harness = Harness::<S>::builder()
            .with_size([480.0, 400.0])
            .with_context_setup(|ctx| {
                install(ctx, &Base::suite_default());
                symbols::install(ctx);
            })
            .build_ui_state(show, state);
        harness.run();
        harness
    }

    struct Composer {
        draft: String,
        posted: Vec<String>,
        closed: usize,
    }

    fn show_composer(ui: &mut Ui, app: &mut Composer) {
        let out = CommentComposer::new("composer", Some(pos2(40.0, 40.0)), &mut app.draft).show(ui);
        app.posted.extend(out.posted);
        app.closed += usize::from(out.closed);
    }

    fn composer() -> Harness<'static, Composer> {
        harness(show_composer, Composer { draft: String::new(), posted: Vec::new(), closed: 0 })
    }

    // Cmd+Enter posts, Esc cancels and closes.
    #[test]
    fn composer_posts_with_command_enter_and_cancels_with_escape() {
        let mut harness = composer();
        harness.type_text("looks off");
        harness.press(KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter));
        assert_eq!(harness.state().posted, ["looks off"]);
        assert_eq!((harness.state().closed, harness.state().draft.as_str()), (1, ""));

        let mut harness = composer();
        harness.get_by_role_and_label(Role::Button, "Cancel");
        harness.type_text("never mind");
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
        assert!(harness.state().posted.is_empty());
        assert_eq!((harness.state().closed, harness.state().draft.as_str()), (1, ""));
    }

    struct Popover {
        thread: CommentThread,
        draft: String,
        edits: Vec<CommentEdit>,
        closed: bool,
    }

    fn show_popover(ui: &mut Ui, app: &mut Popover) {
        let people = vec!["Ana María".to_owned(), "Bo".to_owned(), "Me".to_owned()];
        let anchor = Rect::from_min_size(pos2(20.0, 20.0), vec2(28.0, 28.0));
        let out = ThreadPopover::new("popover", &app.thread, anchor, "Me", &people, &mut app.draft).show(ui);
        app.edits.extend(out.edits);
        app.closed |= out.closed;
    }

    fn popover() -> Harness<'static, Popover> {
        let message = CommentMessage {
            author: "Ana María".into(),
            created: 1_000,
            modified: None,
            text: "Check @Bo".into(),
            reactions: Vec::new(),
            read: true,
        };
        let thread = CommentThread {
            id: ThreadId(7),
            anchor: CommentAnchor::Canvas { at: [0.0, 0.0], node: None },
            kind: None,
            color: None,
            messages: vec![message],
            status: ReviewStatus::None,
            marked: false,
            locked: false,
            resolved: false,
        };
        harness(show_popover, Popover { thread, draft: String::new(), edits: Vec::new(), closed: false })
    }

    // Esc with the focus in the reply field closes the popover (the field passes the press on).
    #[test]
    fn escape_in_the_reply_field_closes_the_popover() {
        let mut harness = popover();
        harness.type_text("draft");
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
        assert!(harness.state().closed);
        assert_eq!(harness.state().draft, "draft", "the draft stays");
    }

    // Esc closes the mention list first and the popover on the next press.
    #[test]
    fn escape_closes_the_mention_list_before_the_popover() {
        let mut harness = popover();
        harness.type_text("hi @bo");
        harness.get_by_role_and_label(Role::ListBox, "Suggestions");
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
        assert!(!harness.state().closed);
        assert!(harness.query_by_role_and_label(Role::ListBox, "Suggestions").is_none());
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
        assert!(harness.state().closed);
    }

    // a click on a suggestion completes the mention and keeps the popover open.
    #[test]
    fn clicking_a_suggestion_completes_the_mention() {
        let mut harness = popover();
        harness.type_text("hi @an");
        harness.get_by_role_and_label(Role::ListBoxOption, "Ana María").click();
        harness.run();
        assert_eq!(harness.state().draft, "hi @Ana María ");
        assert!(!harness.state().closed);
    }

    // the "…" menu marks unread, copies the link and deletes your own thread.
    #[test]
    fn more_menu_holds_unread_link_and_delete_for_your_thread() {
        let mut harness = popover();
        harness.get_by_label("More").click();
        harness.run();
        assert!(harness.query_by_label("Delete thread").is_none(), "someone else's thread");
        harness.get_by_label("Copy link").click();
        harness.run();
        assert_eq!(harness.state().edits, [CommentEdit::CopyLink { thread: ThreadId(7) }]);

        let mut harness = popover();
        harness.state_mut().thread.messages[0].author = "Me".into();
        harness.run();
        harness.get_by_label("More").click();
        harness.run();
        harness.get_by_label("Delete thread").click();
        harness.run();
        assert_eq!(harness.state().edits, [CommentEdit::Delete { thread: ThreadId(7), index: 0 }]);
    }

    // the popover completes @mentions with Enter, posts the reply with the draft, resolves and closes with Esc.
    #[test]
    fn popover_completes_mentions_replies_and_resolves() {
        let mut harness = popover();
        harness.type_text("hi @bo");
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
        assert_eq!(harness.state().draft, "hi @Bo ");
        harness.get_by_role_and_label(Role::Button, "Post").click();
        harness.run();
        assert_eq!(harness.state().edits, [CommentEdit::Reply { thread: ThreadId(7), text: "hi @Bo".into() }]);

        harness.state_mut().edits.clear();
        harness.get_by_label("Resolve").click();
        harness.run();
        assert_eq!(harness.state().edits, [CommentEdit::Resolve { thread: ThreadId(7), resolved: true }]);

        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
        assert!(harness.state().closed);
    }

    struct Pins {
        pins: Vec<(ThreadId, egui::Pos2, &'static str)>,
        open: Option<ThreadId>,
        opened: Vec<ThreadId>,
        closed: Vec<ThreadId>,
        moves: Vec<EditEvent<PinMove>>,
    }

    fn show_pins(ui: &mut Ui, app: &mut Pins) {
        let pins: Vec<CommentPin> = app
            .pins
            .iter()
            .map(|&(id, point, author)| CommentPin {
                id,
                point,
                author,
                text: "Check the margin",
                color: [200, 80, 40],
                unread: id == ThreadId(1),
                resolved: id == ThreadId(3),
            })
            .collect();
        let out = CommentPins::new("pins", &pins, "Me").open(app.open).show(ui);
        app.opened.extend(out.opened);
        app.closed.extend(out.closed);
        app.moves.extend(out.moved);
    }

    // a click opens a pin's thread; your own pin drags as one gesture; near pins gather into a count.
    #[test]
    fn pins_open_drag_and_cluster() {
        let pins = vec![(ThreadId(1), pos2(100.0, 100.0), "Ana"), (ThreadId(2), pos2(300.0, 100.0), "Me"), (ThreadId(3), pos2(310.0, 104.0), "Bo")];
        let mut harness = harness(show_pins, Pins { pins, open: None, opened: Vec::new(), closed: Vec::new(), moves: Vec::new() });
        harness.get_by_label("2 comments");
        harness.get_by_label("Comment by Ana: Check the margin").click();
        harness.run();
        assert_eq!(harness.state().opened, [ThreadId(1)]);
        assert!(harness.state().closed.is_empty());

        // A click on the open pin closes it instead of opening it again.
        harness.state_mut().open = Some(ThreadId(1));
        harness.state_mut().opened.clear();
        harness.run();
        harness.get_by_label("Comment by Ana: Check the margin").click();
        harness.run();
        assert_eq!(harness.state().closed, [ThreadId(1)]);
        assert!(harness.state().opened.is_empty());

        let mut harness = harness_with_own_pin();
        harness.drag(pos2(110.0, 85.0), pos2(150.0, 85.0), Modifiers::NONE);
        let moves = &harness.state().moves;
        assert!(matches!(moves.first(), Some(EditEvent::Begin { .. })), "{moves:?}");
        assert!(matches!(moves.last(), Some(EditEvent::Commit { value, .. }) if value.thread == ThreadId(2) && value.to.x > 110.0), "{moves:?}");
    }

    fn harness_with_own_pin() -> Harness<'static, Pins> {
        harness(
            show_pins,
            Pins { pins: vec![(ThreadId(2), pos2(100.0, 100.0), "Me")], open: None, opened: Vec::new(), closed: Vec::new(), moves: Vec::new() },
        )
    }
}
