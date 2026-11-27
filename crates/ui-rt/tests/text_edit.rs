//! Trials of the text editing engine: typing, word moves and deletes by each platform's chords,
//! click counts, drag selection at the view's edge, coalesced undo, masking and the limit.

use craft_behavior::keys::intent;
use craft_behavior::{EditCommand, EditorInput, Intent, KeyRole};
use craft_ui_rt::record::FixedText;
use craft_ui_rt::text_edit::{EditConfig, EditLayout, Geometry, TextEdit};
use craft_ui_rt::{FontSpec, Key, Mods, Os, Pos2, Rect, Rgba, TextEngine, TextSpec, Vec2};

/// Each character is 5 points wide and rows 12.5 tall (FixedText at size 10).
const SIZE: f32 = 10.0;

fn layout_of(edit: &TextEdit, text: &str, config: &EditConfig, wrap: Option<f32>) -> EditLayout {
    let shown = edit.shown(text, config);
    let spec = TextSpec {
        text: shown.text.clone().into(),
        font: FontSpec { slot: craft_theme::FontSlot::Ui, size: SIZE },
        line_height: None,
        color: Rgba([0, 0, 0, 255]),
        max_width: wrap,
        overflow: craft_ui_rt::Overflow::Wrap,
    };
    let mut engine = FixedText::default();
    let layout = engine.layout(&spec, 1.0);
    let metrics = engine.metrics(&layout);
    EditLayout::new(layout, &metrics, &shown)
}

fn geometry<'a>(layout: &'a EditLayout, view: Rect, edit: &TextEdit) -> Geometry<'a> {
    Geometry { layout, view, origin: view.min, scroll: edit.scroll() }
}

/// The editor input `key` with `mods` makes on `os`, through the platform's chords.
fn key(os: Os, key: Key, mods: Mods) -> EditorInput {
    let role = KeyRole::new().text(true).moves(true).deletes(true);
    match intent(os, key, mods, role).expect("a text key") {
        Intent::Move(mv, by) => EditorInput::Move(mv, by),
        Intent::Delete { forward, by, shift, .. } => EditorInput::delete_with_shift(forward, by, shift),
        Intent::Edit(command) => EditorInput::Edit(command),
        other => panic!("not an editor key: {other:?}"),
    }
}

fn typed(edit: &mut TextEdit, text: &mut String, config: &EditConfig, s: &str) {
    for c in s.chars() {
        edit.apply(text, &EditorInput::Text(c.to_string().into()), config, None);
    }
}

#[test]
fn typing_word_moves_and_deletes_follow_the_platform() {
    let word = |os: Os| if os == Os::Mac { Mods { alt: true, ..Mods::NONE } } else { Mods { cmd: true, ..Mods::NONE } };
    for os in [Os::Mac, Os::Windows, Os::Linux] {
        let config = EditConfig::line(os);
        let mut text = String::new();
        let mut edit = TextEdit::new(&text);
        typed(&mut edit, &mut text, &config, "one two three");
        assert_eq!(text, "one two three", "{os:?}: typing inserts at the caret");
        edit.apply(&mut text, &key(os, Key::Left, word(os)), &config, None);
        assert_eq!(edit.caret(), 8, "{os:?}: a word move back stops at the word's start");
        edit.apply(&mut text, &key(os, Key::Left, word(os)), &config, None);
        assert_eq!(edit.caret(), 4);
        edit.apply(&mut text, &key(os, Key::Right, word(os)), &config, None);
        // macOS stops at the end of the word, the others at the start of the next.
        assert_eq!(edit.caret(), if os == Os::Mac { 7 } else { 8 }, "{os:?}: a forward word move");
        edit.apply(&mut text, &key(os, Key::Backspace, word(os)), &config, None);
        assert_eq!(text, if os == Os::Mac { "one  three" } else { "one three" }, "{os:?}: a word delete back removes the word");
    }
}

#[test]
fn double_click_selects_a_word_and_triple_a_line() {
    let config = EditConfig { multiline: true, ..EditConfig::line(Os::Linux) };
    let text = String::from("alpha beta\ngamma delta");
    let mut edit = TextEdit::new(&text);
    let layout = layout_of(&edit, &text, &config, None);
    let view = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0));
    // "beta" spans x 30..50 on the first row.
    let on_beta = Pos2::new(37.0, 5.0);
    edit.press(&text, &geometry(&layout, view, &edit), on_beta, 2, false);
    edit.release();
    assert_eq!(&text[edit.selection()], "beta", "a double click selects the word under it");
    edit.press(&text, &geometry(&layout, view, &edit), on_beta, 3, false);
    edit.release();
    assert_eq!(&text[edit.selection()], "alpha beta", "a triple click selects the row");
    edit.press(&text, &geometry(&layout, view, &edit), Pos2::new(12.0, 18.0), 1, false);
    assert_eq!(edit.selection(), 13..13, "a click on the second row places the caret there");
}

#[test]
fn a_drag_past_the_edge_selects_and_scrolls() {
    let config = EditConfig::line(Os::Linux);
    let text = String::from("abcdefghijklmnopqrstuvwxyz");
    let mut edit = TextEdit::new(&text);
    edit.select(&text, 0, 0);
    let layout = layout_of(&edit, &text, &config, None);
    // The view shows 10 of the 26 characters.
    let view = Rect::from_min_size(Pos2::ZERO, Vec2::new(50.0, 12.5));
    edit.press(&text, &geometry(&layout, view, &edit), Pos2::new(1.0, 5.0), 1, false);
    let mut scrolled = 0;
    for _ in 0..3 {
        if edit.drag(&text, &geometry(&layout, view, &edit), Pos2::new(70.0, 5.0), &config) {
            scrolled += 1;
        }
    }
    assert_eq!(scrolled, 3, "every drag frame past the edge scrolls");
    assert!(edit.scroll().x > 0.0, "the text scrolled toward the pointer");
    assert_eq!(edit.selection().start, 0, "the selection keeps the press's end");
    assert!(edit.selection().end > 10, "the selection reaches past what the view showed at the press");
    let max = geometry(&layout, view, &edit).max_scroll().x;
    for _ in 0..20 {
        edit.drag(&text, &geometry(&layout, view, &edit), Pos2::new(70.0, 5.0), &config);
    }
    edit.release();
    assert_eq!(edit.scroll().x, max, "the scroll stops at the text's end");
}

#[test]
fn undo_takes_back_a_typing_run_in_one_step_and_redo_restores_it() {
    let config = EditConfig::line(Os::Linux);
    let mut text = String::from("start");
    let mut edit = TextEdit::new(&text);
    typed(&mut edit, &mut text, &config, " here");
    assert_eq!(edit.history(), (true, false));
    let undo = edit.apply(&mut text, &EditorInput::Edit(EditCommand::Undo), &config, None);
    assert!(undo.text_changed);
    assert_eq!(text, "start", "one undo takes back the whole typing run");
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Redo), &config, None);
    assert_eq!(text, "start here", "redo restores it");
    assert_eq!(edit.caret(), text.len(), "the caret lands after the restored text");
    // A move ends the run: the next typing is its own step.
    edit.apply(&mut text, &key(Os::Linux, Key::Home, Mods::NONE), &config, None);
    typed(&mut edit, &mut text, &config, ">");
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Undo), &config, None);
    assert_eq!(text, "start here");
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Undo), &config, None);
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Undo), &config, None);
    let none = edit.apply(&mut text, &EditorInput::Edit(EditCommand::Undo), &config, None);
    assert_eq!(text, "start");
    assert_eq!(none.unhandled, Some(EditCommand::Undo), "an undo with no step is left for the app's command");
}

#[test]
fn a_masked_field_copies_nothing_and_the_limit_holds() {
    let config = EditConfig { mask: true, limit: Some(4), ..EditConfig::line(Os::Linux) };
    let mut text = String::new();
    let mut edit = TextEdit::new(&text);
    typed(&mut edit, &mut text, &config, "secret");
    assert_eq!(text, "secr", "typing stops at the limit");
    assert!(!edit.shown(&text, &config).text.contains('s'), "the shown text is masked");
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::SelectAll), &config, None);
    let copy = edit.apply(&mut text, &EditorInput::Edit(EditCommand::Copy), &config, None);
    let cut = edit.apply(&mut text, &EditorInput::Edit(EditCommand::Cut), &config, None);
    assert_eq!((copy.copy, cut.copy), (None, None), "a masked text never reaches the clipboard");
    assert_eq!(text, "secr");
    let paste = edit.paste(&mut text, "a\nb", &EditConfig::line(Os::Linux));
    assert!(paste.text_changed);
    assert_eq!(text, "a b", "a single line pastes a line break as a space");
}

#[test]
fn undo_restores_the_selection_stored_with_the_step() {
    let config = EditConfig::line(Os::Linux);
    let mut text = String::from("one two three");
    let mut edit = TextEdit::new(&text);
    // Select "two" from its end back to its start and cut it.
    edit.select(&text, 7, 4);
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Cut), &config, None);
    assert_eq!(text, "one  three");
    // Typing after a caret move is its own step; undo takes it back, then the cut.
    edit.select(&text, 0, 0);
    typed(&mut edit, &mut text, &config, ">");
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Undo), &config, None);
    assert_eq!((text.as_str(), edit.caret(), edit.anchor()), ("one  three", 0, 0), "the typing's step brings back its caret");
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Undo), &config, None);
    assert_eq!(text, "one two three");
    assert_eq!((edit.caret(), edit.anchor()), (4, 7), "the cut word comes back selected as it was, caret on the same side");
    edit.apply(&mut text, &EditorInput::Edit(EditCommand::Redo), &config, None);
    assert_eq!(text, "one  three");
    assert_eq!(edit.caret(), edit.anchor(), "redo restores the state undo left, an empty selection");
}
