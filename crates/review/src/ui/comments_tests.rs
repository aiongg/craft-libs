//! The comments panel (egui_kittest).

use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::comments::{CommentEdit, CommentsPanel, CommentsPanelState};
use crate::annotate::AnnotationKind;
use crate::comments::{CommentAnchor, CommentMessage, CommentThread, ReviewStatus, ThreadId};

fn message(author: &str, text: &str) -> CommentMessage {
    CommentMessage { author: author.into(), created: 1_000, modified: None, text: text.into(), reactions: Vec::new(), read: true }
}

fn thread(id: u64, page: usize, y: f64, text: &str, replies: usize) -> CommentThread {
    let mut messages = vec![message("Ana", text)];
    messages.extend((0..replies).map(|n| message("Bo", &format!("reply {n}"))));
    CommentThread {
        id: ThreadId(id),
        anchor: CommentAnchor::Page { page, at: [0.0, y] },
        kind: Some(AnnotationKind::Note),
        color: None,
        messages,
        status: ReviewStatus::None,
        marked: false,
        locked: false,
        resolved: false,
    }
}

struct App {
    threads: Vec<CommentThread>,
    state: CommentsPanelState,
    edits: Vec<CommentEdit>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = CommentsPanel::new("comments", &app.threads, &mut app.state).show(ui);
    app.edits.extend(out.edits);
}

fn harness(threads: Vec<CommentThread>, selected: Option<u64>) -> Harness<'static, App> {
    let state = CommentsPanelState { selected: selected.map(ThreadId), ..CommentsPanelState::default() };
    let mut harness = Harness::<App>::builder()
        .with_size([360.0, 640.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, App { threads, state, edits: Vec::new() });
    harness.run();
    harness
}

// the header shows Comments N and N replies.
#[test]
fn header_counts_comments_and_replies() {
    let harness = harness(vec![thread(1, 1, 0.0, "first", 2), thread(2, 1, 9.0, "second", 1)], None);
    harness.get_by_label("Comments 2");
    harness.get_by_label("3 replies");
}

// Add a comment and a reply report Add and Reply.
#[test]
fn posting_and_replying_report_edits() {
    let mut harness = harness(vec![thread(1, 1, 0.0, "first", 0)], Some(1));
    harness.get_by_role_and_label(Role::TextInput, "Add a comment").focus();
    harness.run();
    harness.type_text("a new note");
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
    assert_eq!(harness.state().edits, [CommentEdit::Add { text: "a new note".into() }]);
    assert!(harness.state().state.new_comment.is_empty());

    harness.state_mut().edits.clear();
    harness.get_by_role_and_label(Role::MultilineTextInput, "Add a reply").focus();
    harness.run();
    harness.type_text("agreed");
    harness.get_by_role_and_label(Role::Button, "Post").click();
    harness.run();
    assert_eq!(harness.state().edits, [CommentEdit::Reply { thread: ThreadId(1), text: "agreed".into() }]);
}

// the card menu reports SetStatus and Delete; locked threads hide it.
#[test]
fn selected_card_menu_sets_status_and_deletes() {
    let mut harness = harness(vec![thread(1, 1, 0.0, "first", 0)], Some(1));
    harness.get_by_label("More").click();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();
    assert_eq!(harness.state().edits, [CommentEdit::Delete { thread: ThreadId(1), index: 0 }]);

    harness.state_mut().edits.clear();
    harness.get_by_label("More").click();
    harness.run();
    harness.get_by_label("Set status").click();
    harness.run();
    harness.get_by_label("Accepted").click();
    harness.run();
    assert_eq!(harness.state().edits, [CommentEdit::SetStatus { thread: ThreadId(1), status: ReviewStatus::Accepted }]);

    let mut locked = thread(1, 1, 0.0, "first", 0);
    locked.locked = true;
    let harness = self::harness(vec![locked], Some(1));
    assert!(harness.query_by_label("More").is_none());
    harness.get_by_label("Locked");
}

// arrow keys select the previous and next card.
#[test]
fn up_and_down_move_between_cards() {
    let mut harness = harness(vec![thread(1, 1, 0.0, "first", 0), thread(2, 1, 5.0, "second", 0), thread(3, 1, 9.0, "third", 0)], Some(1));
    harness.get_by_role_and_label(Role::ListItem, "Ana: first").focus();
    harness.run();
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::ArrowDown));
    harness.run();
    assert_eq!(harness.state().state.selected, Some(ThreadId(2)));
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::ArrowDown));
    harness.run();
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::ArrowUp));
    harness.run();
    assert_eq!(harness.state().state.selected, Some(ThreadId(2)));
    assert_eq!(harness.state().edits, [CommentEdit::Select(ThreadId(2)), CommentEdit::Select(ThreadId(3)), CommentEdit::Select(ThreadId(2))]);
}

// one reply reads "1 reply", and the title is a placeholder word.
#[test]
fn reply_count_uses_the_singular() {
    let harness = harness(vec![thread(1, 1, 0.0, "first", 1)], None);
    harness.get_by_label("1 reply");
    harness.get_by_label("Comments 1");
}

// a page heading shows the count before filtering and folds its cards; the card of a folded group is
// back after Expand, and `reveal` unfolds the group of a thread the app selected.
#[test]
fn page_groups_fold_and_count() {
    let threads = vec![thread(1, 1, 0.0, "first", 0), thread(2, 1, 9.0, "second", 0), thread(3, 2, 0.0, "third", 0)];
    let mut harness = harness(threads, None);
    harness.get_by_label("Page 1");
    harness.get_by_label("2");
    harness.get_all_by_label("Collapse").next().expect("a fold button").click();
    harness.run();
    assert!(harness.query_by_label("Ana: first").is_none());
    harness.get_by_label("Ana: third");
    let second = harness.state().threads[1].clone();
    let App { state, .. } = harness.state_mut();
    state.reveal(&second);
    harness.run();
    harness.get_by_label("Ana: first");
}

// a filter that hides every type leaves "No comments match."; Show all brings the cards back.
#[test]
fn filter_hides_threads_and_reports_no_match() {
    let mut harness = harness(vec![thread(1, 1, 0.0, "first", 0)], None);
    harness.state_mut().state.filter.hidden_types.push(Some(AnnotationKind::Note));
    harness.run();
    harness.get_by_label("No comments match.");
    assert!(harness.query_by_label("Ana: first").is_none());
    harness.state_mut().state.filter.clear();
    harness.run();
    harness.get_by_label("Ana: first");
}

// the search ignores case and the clear button empties it.
#[test]
fn search_filters_and_clears() {
    let mut harness = harness(vec![thread(1, 1, 0.0, "Alpha", 0), thread(2, 1, 5.0, "Beta", 0)], None);
    harness.get_by_role_and_label(Role::SearchInput, "Search comments").focus();
    harness.run();
    harness.type_text("BETA");
    harness.run();
    assert!(harness.query_by_label("Ana: Alpha").is_none());
    harness.get_by_label("Ana: Beta");
    harness.get_by_label("Clear search").click();
    harness.run();
    harness.get_by_label("Ana: Alpha");
}
