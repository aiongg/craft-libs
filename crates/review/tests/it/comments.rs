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
}

/// A click on a card's "..." opens the menu under that button; Escape closes it and focus returns to the button.
#[trial]
fn card_menu_hangs_from_its_button_and_escape_returns_focus() {
    let mut rig = rig(vec![thread(1, 1, 0.0, "first", 0)], Some(1));
    let button = rig.fx.by_label("More").expect("the selected card shows its menu button");
    rig.fx.click(&button);
    rig.fx.settle();
    let menu = rig.fx.by_role(craft_ui::contract::Role::MENU).into_iter().next().expect("the click opens the menu");
    assert!(menu.rect.top() >= button.rect.center().y, "the menu hangs under the button: {:?} under {:?}", menu.rect, button.rect);
    rig.fx.key(egui::Key::Escape, egui::Modifiers::NONE);
    rig.fx.settle();
    assert!(rig.fx.by_role(craft_ui::contract::Role::MENU).is_empty(), "Escape closes the menu");
    let (_, part) = rig.fx.focused_stop().expect("focus stays in the card");
    assert!(part.is_some_and(|part| part.key.name() == "more"), "focus returns to the button: {part:?}");
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
