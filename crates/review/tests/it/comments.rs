//! The Comments panel through the fixture.

use std::cell::RefCell;
use std::rc::Rc;

use craft_review::ui::{CommentEdit, CommentsPanel, CommentsPanelState};
use craft_review::{AnnotationKind, CommentAnchor, CommentMessage, CommentThread, ReviewStatus, ThreadId};
use craft_ui::testing::{Fixture, trial};

const NOW: i64 = 4_600;

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
    now: i64,
}

struct Rig {
    fx: Fixture,
    app: Rc<RefCell<App>>,
}

fn rig(threads: Vec<CommentThread>, selected: Option<u64>) -> Rig {
    let mut state = CommentsPanelState::default();
    state.selected = selected.map(ThreadId);
    let app = Rc::new(RefCell::new(App { threads, state, edits: Vec::new(), now: NOW }));
    let shown = Rc::clone(&app);
    let mut fx = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let App { threads, state, edits, now } = &mut *app;
        let out = CommentsPanel::new("Comments", threads, state).now(*now).show(ui);
        edits.extend(out.edits);
    });
    fx.room(egui::vec2(380.0, 700.0));
    fx.settle();
    Rig { fx, app }
}

impl Rig {
    fn click(&mut self, label: &str) {
        let target = self.fx.by_label(label).unwrap_or_else(|| panic!("no `{label}` on screen"));
        self.fx.click(&target);
        self.fx.settle();
    }

    fn present(&self, text: &str) -> bool {
        let card = format!("{text}, ");
        let tree = self.fx.tree();
        self.fx.by_label(text).is_some()
            || self.fx.texts().iter().any(|painted| painted.text == text)
            || self
                .fx
                .by_role(craft_ui::contract::Role::TREE_ITEM)
                .iter()
                .any(|row| tree.name_of(row.node).is_some_and(|name| name.starts_with(&card)))
    }

    fn type_into(&mut self, label: &str, text: &str) {
        self.click(label);
        self.fx.type_text(text);
        self.fx.settle();
    }

    fn edits(&self) -> Vec<CommentEdit> {
        self.app.borrow().edits.clone()
    }
}

#[trial]
fn header_counts_comments_and_replies() {
    let rig = rig(vec![thread(1, 1, 0.0, "first", 2), thread(2, 1, 9.0, "second", 1)], None);
    assert!(rig.present("Comments 2"));
    assert!(rig.present("3 replies"));
}

#[trial]
fn reply_count_uses_the_singular() {
    let rig = rig(vec![thread(1, 1, 0.0, "first", 1)], None);
    assert!(rig.present("1 reply"));
    assert!(rig.present("Comments 1"));
}

#[trial]
fn posting_and_replying_report_edits() {
    let mut rig = rig(vec![thread(1, 1, 0.0, "first", 0)], Some(1));
    rig.type_into("Add a comment", "a new note");
    rig.click("Post");
    assert!(matches!(rig.edits().as_slice(), [CommentEdit::Add { text, .. }] if text == "a new note"), "{:?}", rig.edits());
    assert!(rig.app.borrow().state.new_comment.is_empty());

    rig.app.borrow_mut().edits.clear();
    rig.type_into("Add a reply", "agreed");
    assert_eq!(rig.app.borrow().state.reply, "agreed");
    rig.app.borrow_mut().state.reply.clear();
    rig.app.borrow_mut().edits.clear();
}

/// A click on a card's "..." opens the menu under that button; Escape closes it and focus returns to the button.
#[trial]
fn card_menu_hangs_from_its_button_and_escape_returns_focus() {
    let mut rig = rig(vec![thread(1, 1, 0.0, "first", 0)], Some(1));
    let button = rig.fx.by_label("More").expect("the selected card shows its menu button");
    rig.fx.click(&button);
    rig.fx.settle();
    let menu = rig.fx.by_role(craft_ui::contract::Role::MENU).into_iter().next().expect("the click opens the menu");
    assert!(menu.rect.top() >= button.rect.bottom() - 1.0, "the menu hangs under the button: {:?} under {:?}", menu.rect, button.rect);
    rig.fx.key(egui::Key::Escape, egui::Modifiers::NONE);
    rig.fx.settle();
    assert!(rig.fx.by_role(craft_ui::contract::Role::MENU).is_empty(), "Escape closes the menu");
    let (_, part) = rig.fx.focused_stop().expect("focus stays in the card");
    assert!(part.is_some_and(|part| part.key.name() == "more"), "focus returns to the button: {part:?}");
}

#[trial]
fn selected_card_menu_sets_status_and_deletes() {
    let mut rig = rig(vec![thread(1, 1, 0.0, "first", 0)], Some(1));
    rig.click("More");
    rig.click("Delete");
    assert!(matches!(rig.edits().as_slice(), [CommentEdit::Delete { thread: ThreadId(1), index: 0, .. }]), "{:?}", rig.edits());

    rig.app.borrow_mut().edits.clear();
    rig.click("More");
    rig.click("Set status");
    rig.click("Accepted");
    assert!(
        matches!(rig.edits().as_slice(), [CommentEdit::SetStatus { thread: ThreadId(1), status: ReviewStatus::Accepted, .. }]),
        "{:?}",
        rig.edits()
    );

    let mut locked = thread(1, 1, 0.0, "first", 0);
    locked.locked = true;
    let rig = self::rig(vec![locked], Some(1));
    assert!(rig.fx.by_label("More").is_none(), "a locked thread hides the menu");
}

#[trial]
fn page_groups_fold_and_count() {
    let threads = vec![thread(1, 1, 0.0, "first", 0), thread(2, 1, 9.0, "second", 0), thread(3, 2, 0.0, "third", 0)];
    let mut rig = rig(threads, None);
    assert!(rig.present("Page 1"));
    assert!(rig.present("2"), "the heading counts its page's comments");
    rig.click("Page 1");
    assert!(!rig.present("Ana: first"), "a folded group hides its cards");
    assert!(rig.present("Ana: third"));
    let second = rig.app.borrow().threads[1].clone();
    rig.app.borrow_mut().state.reveal(&second);
    rig.fx.settle();
    assert!(rig.present("Ana: first"), "`reveal` unfolds the group");
}

#[trial]
fn filter_hides_threads_and_reports_no_match() {
    let mut rig = rig(vec![thread(1, 1, 0.0, "first", 0)], None);
    rig.app.borrow_mut().state.filter.hidden_types.push(Some(AnnotationKind::Note));
    rig.fx.settle();
    assert!(rig.present("No comments match."));
    assert!(!rig.present("Ana: first"));
    rig.app.borrow_mut().state.filter.clear();
    rig.fx.settle();
    assert!(rig.present("Ana: first"));
}

#[trial]
fn search_filters_and_clears() {
    let mut rig = rig(vec![thread(1, 1, 0.0, "Alpha", 0), thread(2, 1, 5.0, "Beta", 0)], None);
    rig.type_into("Search comments", "BETA");
    assert!(!rig.present("Ana: Alpha"));
    assert!(rig.present("Ana: Beta"));
    rig.app.borrow_mut().state.search.clear();
    rig.fx.settle();
    assert!(rig.present("Ana: Alpha"));
}

#[trial]
fn an_age_under_a_week_reads_relative_and_an_older_one_reads_the_date() {
    let mut rig = rig(vec![thread(1, 1, 0.0, "first", 0)], None);
    let year = |rig: &Rig| rig.fx.texts().iter().any(|painted| painted.text.contains("1970") || painted.text.contains("1969"));
    assert!(!year(&rig), "an hour old reads relative");
    rig.app.borrow_mut().now = 1_000 + 30 * 24 * 3_600;
    rig.fx.settle();
    assert!(year(&rig), "a month old reads as a date");
}
