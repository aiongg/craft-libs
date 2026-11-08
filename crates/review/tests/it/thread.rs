//! Pins, the thread popover and the composer through the fixture, and the pure mention and cluster rules.

use std::cell::RefCell;
use std::rc::Rc;

use craft_review::ui::{
    CommentComposer, CommentEdit, CommentPin, CommentPins, PinMove, ThreadPopover, cluster, insert_mention, mention_matches, mention_query,
};
use craft_review::{CommentAnchor, CommentMessage, CommentThread, ReviewStatus, ThreadId};
use craft_ui::buttons::Button;
use craft_ui::contract::{Role, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, pos2, vec2};

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[trial]
fn cluster_joins_points_near_a_group_centre() {
    let points: [Pos2; 4] = [pos2(0.0, 0.0), pos2(20.0, 0.0), pos2(40.0, 0.0), pos2(200.0, 0.0)];
    assert_eq!(cluster(&points, 24.0), vec![vec![0, 1], vec![2], vec![3]]);
    assert_eq!(cluster(&points, 1.0), vec![vec![0], vec![1], vec![2], vec![3]]);
    assert!(cluster(&[], 24.0).is_empty());
}

#[trial]
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

#[trial]
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

struct Composer {
    draft: String,
    posted: Vec<String>,
    cancelled: usize,
}

fn composer() -> (Fixture, Rc<RefCell<Composer>>) {
    let app = Rc::new(RefCell::new(Composer { draft: String::new(), posted: Vec::new(), cancelled: 0 }));
    let shown = Rc::clone(&app);
    let mut fx = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let Composer { draft, posted, cancelled } = &mut *app;
        let out = CommentComposer::new(Some(pos2(40.0, 40.0)), draft).show(ui);
        posted.extend(out.posted);
        *cancelled += usize::from(out.cancelled);
    });
    fx.room(vec2(480.0, 400.0));
    fx.settle();
    (fx, app)
}

/// Command+Enter posts from the multi-line field and Escape in it cancels.
#[trial]
fn composer_posts_with_command_enter_and_cancels_with_escape() {
    let (mut fx, app) = composer();
    focus_field(&mut fx);
    fx.type_text("looks off");
    fx.key(Key::Enter, Modifiers::COMMAND);
    fx.settle();
    assert_eq!(app.borrow().posted, ["looks off"]);
    assert_eq!(app.borrow().draft, "");

    let (mut fx, app) = composer();
    focus_field(&mut fx);
    fx.type_text("never mind");
    fx.key(Key::Escape, Modifiers::NONE);
    fx.settle();
    assert!(app.borrow().posted.is_empty());
    assert_eq!((app.borrow().cancelled, app.borrow().draft.as_str()), (1, ""));
}

struct Popover {
    thread: CommentThread,
    draft: String,
    edits: Vec<CommentEdit>,
    closed: bool,
}

fn popover(author: &str) -> (Fixture, Rc<RefCell<Popover>>) {
    let message =
        CommentMessage { author: author.into(), created: 1_000, modified: None, text: "Check @Bo".into(), reactions: Vec::new(), read: true };
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
    let app = Rc::new(RefCell::new(Popover { thread, draft: String::new(), edits: Vec::new(), closed: false }));
    let shown = Rc::clone(&app);
    let mut fx = Fixture::new(move |ui| {
        let people = vec!["Ana María".to_owned(), "Bo".to_owned(), "Me".to_owned()];
        let opener = Button::new("Pin").show(ui);
        let mut app = shown.borrow_mut();
        let Popover { thread, draft, edits, closed } = &mut *app;
        let out = ThreadPopover::new(thread, &opener, "Me", &people, draft).now(2_000).show(ui);
        edits.extend(out.edits);
        *closed |= out.closed;
    });
    fx.room(vec2(480.0, 600.0));
    fx.settle();
    (fx, app)
}

/// The popover opens with focus on its first stop; the reply field takes it by a click.
fn focus_reply(fx: &mut Fixture) {
    focus_field(fx);
}

/// A click on the (only) text field on screen.
fn focus_field(fx: &mut Fixture) {
    let field = fx.by_role(Role::MULTILINE_TEXT_INPUT).into_iter().chain(fx.by_role(Role::TEXT_INPUT)).next().expect("a text field on screen");
    fx.click(&field);
}

#[trial]
fn escape_closes_the_mention_list_before_the_popover() {
    let (mut fx, app) = popover("Ana María");
    focus_reply(&mut fx);
    fx.type_text("hi @bo");
    fx.settle();
    assert!(fx.by_label("Bo").is_some(), "the mention list is open");
    fx.key(Key::Escape, Modifiers::NONE);
    fx.settle();
    assert!(!app.borrow().closed);
    fx.key(Key::Escape, Modifiers::NONE);
    fx.settle();
    assert!(app.borrow().closed);
    assert_eq!(app.borrow().draft, "hi @bo", "the draft stays");
}

#[trial]
fn clicking_a_suggestion_completes_the_mention() {
    let (mut fx, app) = popover("Ana María");
    focus_reply(&mut fx);
    fx.type_text("hi @an");
    fx.settle();
    let row = fx.by_label("Ana María").expect("the suggestion");
    fx.click(&row);
    fx.settle();
    assert_eq!(app.borrow().draft, "hi @Ana María ");
    assert!(!app.borrow().closed);
}

#[trial]
fn more_menu_holds_unread_link_and_delete_for_your_thread() {
    let (mut fx, app) = popover("Ana María");
    let more = fx.by_label("More").expect("More");
    fx.click(&more);
    fx.settle();
    assert!(fx.by_label("Delete thread").is_none(), "someone else's thread");
    let link = fx.by_label("Copy link").expect("Copy link");
    fx.click(&link);
    fx.settle();
    assert!(matches!(app.borrow().edits.as_slice(), [CommentEdit::CopyLink { thread: ThreadId(7), .. }]));

    let (mut fx, app) = popover("Me");
    let more = fx.by_label("More").expect("More");
    fx.click(&more);
    fx.settle();
    let delete = fx.by_label("Delete thread").expect("Delete thread");
    fx.click(&delete);
    fx.settle();
    assert!(matches!(app.borrow().edits.as_slice(), [CommentEdit::Delete { thread: ThreadId(7), index: 0, .. }]));
}

#[trial]
fn popover_completes_mentions_replies_and_resolves() {
    let (mut fx, app) = popover("Ana María");
    focus_reply(&mut fx);
    fx.type_text("hi @bo");
    fx.settle();
    fx.key(Key::Enter, Modifiers::NONE);
    fx.settle();
    assert_eq!(app.borrow().draft, "hi @Bo ");
    let post = fx.by_label("Post").expect("Post");
    fx.click(&post);
    fx.settle();
    assert!(matches!(app.borrow().edits.as_slice(), [CommentEdit::Reply { thread: ThreadId(7), text, .. }] if text == "hi @Bo"));

    app.borrow_mut().edits.clear();
    let resolve = fx.by_label("Resolve").expect("Resolve");
    fx.click(&resolve);
    fx.settle();
    assert!(matches!(app.borrow().edits.as_slice(), [CommentEdit::Resolve { thread: ThreadId(7), resolved: true, .. }]));
}

struct Pins {
    pins: Vec<(ThreadId, Pos2, &'static str)>,
    open: Option<ThreadId>,
    opened: Vec<ThreadId>,
    closed: Vec<ThreadId>,
    moves: Vec<PinMove>,
}

fn pins(pins: Vec<(ThreadId, Pos2, &'static str)>) -> (Fixture, Rc<RefCell<Pins>>) {
    let app = Rc::new(RefCell::new(Pins { pins, open: None, opened: Vec::new(), closed: Vec::new(), moves: Vec::new() }));
    let shown = Rc::clone(&app);
    let mut fx = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let list: Vec<CommentPin> = app
            .pins
            .iter()
            .map(|&(id, point, author)| {
                CommentPin::new(id, point, author, "Check the margin", [200, 80, 40]).unread(id == ThreadId(1)).resolved(id == ThreadId(3))
            })
            .collect();
        let out = CommentPins::new("Pins", &list, "Me").open(app.open).show(ui);
        app.opened.extend(out.opened);
        app.closed.extend(out.closed);
        app.moves.extend(out.moved);
    });
    fx.room(vec2(480.0, 400.0));
    fx.settle();
    (fx, app)
}

#[trial]
fn pins_open_drag_and_cluster() {
    let (mut fx, app) =
        pins(vec![(ThreadId(1), pos2(100.0, 100.0), "Ana"), (ThreadId(2), pos2(300.0, 100.0), "Me"), (ThreadId(3), pos2(310.0, 104.0), "Bo")]);
    assert!(fx.by_label("2 comments").is_some(), "near pins gather into a count");
    let ana = fx.by_label("Comment by Ana: Check the margin").expect("Ana's pin");
    fx.click(&ana);
    fx.settle();
    assert_eq!(app.borrow().opened, [ThreadId(1)]);
    assert!(app.borrow().closed.is_empty());

    app.borrow_mut().open = Some(ThreadId(1));
    app.borrow_mut().opened.clear();
    fx.settle();
    let ana = fx.by_label("Comment by Ana: Check the margin").expect("Ana's pin");
    fx.click(&ana);
    fx.settle();
    assert_eq!(app.borrow().closed, [ThreadId(1)]);
    assert!(app.borrow().opened.is_empty());

    let start = pos2(100.0, 100.0);
    let (mut fx, app) = pins(vec![(ThreadId(2), start, "Me")]);
    let own = fx.by_label("Comment by Me: Check the margin").expect("own pin");
    fx.drag(&own, vec2(40.0, 0.0));
    fx.settle();
    let moves = app.borrow().moves.clone();
    assert!(matches!(moves.last(), Some(PinMove { thread, to, .. }) if *thread == ThreadId(2) && to.x > start.x), "{moves:?}");
}
