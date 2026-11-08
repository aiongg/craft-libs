//! The text editor over an in-memory host: one line, one unit per char.

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

use craft_behavior::kurbo::Point;
use craft_text::quotes::QuoteStyle;
use craft_text::ui::editor::{CaretLine, EditKind, HostLine, TextEditor, TextEditorHost, TextSelection, click_count, click_selection, word_boundary};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Rect, pos2, vec2};

#[derive(Default)]
struct Host {
    text: String,
    revision: u64,
    vertical: bool,
}

impl TextEditorHost for Host {
    fn text(&self) -> &str {
        &self.text
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn replace(&mut self, range: Range<usize>, text: &str, _: EditKind) -> bool {
        let byte = |chars: usize| self.text.char_indices().nth(chars).map_or(self.text.len(), |(at, _)| at);
        let (from, to) = (byte(range.start), byte(range.end));
        self.text.replace_range(from..to, text);
        self.revision += 1;
        true
    }

    fn hit(&mut self, at: Point) -> (usize, usize) {
        ((at.x.round().max(0.0) as usize).min(self.text.chars().count()), 0)
    }

    fn caret(&mut self, offset: usize, _: Option<usize>) -> Option<CaretLine> {
        Some(CaretLine::new(Point::new(offset as f64, 0.0), Point::new(offset as f64, 1.0)))
    }

    fn selection_quads(&mut self, _: Range<usize>) -> Vec<[Point; 4]> {
        Vec::new()
    }

    fn underline_segments(&mut self, _: Range<usize>) -> Vec<[Point; 2]> {
        Vec::new()
    }

    fn lines(&mut self) -> Vec<HostLine> {
        let count = self.text.chars().count();
        vec![HostLine::new(self.text.clone(), 0..count, Rect::from_min_size(pos2(0.0, 0.0), vec2(count as f32, 1.0)), 1.0, Vec::new())]
    }

    fn line_range(&mut self, _: usize) -> Range<usize> {
        0..self.text.chars().count()
    }

    fn line_step(&mut self, offset: usize, _: i32) -> usize {
        offset
    }

    fn line_edge(&mut self, offset: usize, end: bool) -> usize {
        let _ = offset;
        if end { self.text.chars().count() } else { 0 }
    }
}

struct Session {
    fixture: Fixture,
    host: Rc<RefCell<Host>>,
    selection: Rc<RefCell<TextSelection>>,
    commit: Rc<Cell<bool>>,
}

fn session(text: &str, quotes: Option<QuoteStyle>) -> Session {
    let host = Rc::new(RefCell::new(Host { text: text.to_owned(), ..Host::default() }));
    let selection = Rc::new(RefCell::new(TextSelection::caret_at(text.chars().count())));
    let commit = Rc::new(Cell::new(false));
    let (shown_host, shown_selection, shown_commit) = (Rc::clone(&host), Rc::clone(&selection), Rc::clone(&commit));
    let mut fixture = Fixture::new(move |ui| {
        let (mut host, mut selection) = (shown_host.borrow_mut(), shown_selection.borrow_mut());
        Canvas::new("Document").show(ui, |ui, body| {
            let bounds = Rect::from_min_size(body.rect.min, vec2(200.0, 20.0));
            let editor = TextEditor::new("Text", &mut *host, Bind::bound(&mut *selection));
            let output = if let Some(style) = quotes { editor.smart_quotes(style) } else { editor }.show_at(ui, bounds);
            if output.commit {
                shown_commit.set(true);
            }
        });
    });
    fixture.settle();
    Session { fixture, host, selection, commit }
}

impl Session {
    fn focus(&mut self) {
        let at = self.fixture.root_id().and_then(|_| None::<egui::Pos2>).unwrap_or(pos2(20.0, 10.0));
        self.fixture.click_at(at);
        self.fixture.settle();
    }

    fn text(&self) -> String {
        self.host.borrow().text.clone()
    }
}

#[trial]
fn typing_inserts_at_the_caret_and_replaces_the_selection() {
    let mut editor = session("hello", None);
    editor.focus();
    *editor.selection.borrow_mut() = TextSelection::new(0, 5);
    editor.fixture.type_text("bye").settle();
    assert_eq!(editor.text(), "bye");
}

#[trial]
fn typed_quotes_follow_the_style() {
    let mut editor = session("", Some(QuoteStyle::ENGLISH));
    editor.focus();
    editor.fixture.type_text("\"hi\"").settle();
    assert_ne!(editor.text(), "\"hi\"", "straight quotes become the language's marks while typing");
}

#[trial]
fn backspace_deletes_by_char_and_selection() {
    let mut editor = session("abc", None);
    editor.focus();
    editor.fixture.key(Key::Backspace, Modifiers::NONE).settle();
    assert_eq!(editor.text(), "ab");
    *editor.selection.borrow_mut() = TextSelection::new(0, 2);
    editor.fixture.key(Key::Backspace, Modifiers::NONE).settle();
    assert_eq!(editor.text(), "");
}

#[trial]
fn shift_arrows_extend_and_arrows_collapse() {
    let mut editor = session("abc", None);
    editor.focus();
    *editor.selection.borrow_mut() = TextSelection::caret_at(0);
    editor.fixture.key(Key::ArrowRight, Modifiers::SHIFT).settle();
    assert_eq!(*editor.selection.borrow(), TextSelection::new(0, 1));
    editor.fixture.key(Key::ArrowRight, Modifiers::NONE).settle();
    assert_eq!(*editor.selection.borrow(), TextSelection::caret_at(1));
}

/// In vertical text the arrows turn with the flow: Down is the next char and Up the previous one, and Left and
/// Right go by line, which this host does not have.
#[trial]
fn in_vertical_text_up_and_down_move_by_char() {
    let mut editor = session("abc", None);
    editor.host.borrow_mut().vertical = true;
    editor.focus();
    *editor.selection.borrow_mut() = TextSelection::caret_at(1);
    editor.fixture.key(Key::ArrowDown, Modifiers::NONE).settle();
    assert_eq!(*editor.selection.borrow(), TextSelection::caret_at(2));
    editor.fixture.key(Key::ArrowUp, Modifiers::NONE).settle();
    assert_eq!(*editor.selection.borrow(), TextSelection::caret_at(1));
    editor.fixture.key(Key::ArrowRight, Modifiers::NONE).settle();
    assert_eq!(*editor.selection.borrow(), TextSelection::caret_at(1), "Right goes to the next column");
}

/// Text the app changes under the editor leaves the selection in the new text: the next move starts from the end.
#[trial]
fn a_change_from_outside_keeps_the_selection_in_the_text() {
    let mut editor = session("hello", None);
    editor.focus();
    *editor.selection.borrow_mut() = TextSelection::caret_at(5);
    editor.host.borrow_mut().replace(0..5, "hi", EditKind::Typed);
    editor.fixture.settle();
    editor.fixture.key(Key::ArrowLeft, Modifiers::NONE).settle();
    assert_eq!(*editor.selection.borrow(), TextSelection::caret_at(1));
}

#[trial]
fn escape_commits_the_session() {
    let mut editor = session("abc", None);
    editor.focus();
    editor.fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(editor.commit.get());
}

#[trial]
fn click_runs_select_word_line_paragraph_all() {
    let text = "one two\nthree";
    assert_eq!((1..=6).map(click_count).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 1], "the sixth press starts over");
    assert_eq!(click_selection(text, 5, None, 2), 4..7);
    assert_eq!(click_selection(text, 5, Some(0..7), 3), 0..7);
    assert_eq!(click_selection(text, 9, None, 4), 8..13);
    assert_eq!(click_selection(text, 9, None, 5), 0..13);
}

#[trial]
fn words_break_at_letters_and_digits() {
    assert_eq!(word_boundary("one, two", 0, true), 3);
    assert_eq!(word_boundary("one, two", 8, false), 5);
}
