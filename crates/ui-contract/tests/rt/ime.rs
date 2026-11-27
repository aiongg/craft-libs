//! Trials of the IME engine on ui-rt's input method session, with an editor part over a string: a preedit and its
//! commit insert once; a click during a composition commits it, ends the platform's composition and lands; an app
//! change sends the surrounding text once and an input method commit sends none; an undo the editor has no step for
//! runs the app's command; typing runs only the field. Relations only, no times.

use crate::skeleton::IconButton;
use craft_behavior::{EditCommand, EditorInput, IconId, PartKey, TextPos};
use craft_keymap::{Keymap, Platform, Scopes};
use craft_ui_contract::contract::{Common, Component, Cx, Input as InputKind, MeasureCx, PartsCx, Response, Role, Show, Ui};
use craft_ui_contract::primitives::{EditorPurpose, EditorState, EditorSurrounding};
use craft_ui_contract::runtime::commands::{CommandBoard, EditCommands};
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::testing::Harness;
use craft_ui_platform_headless::Input;
use craft_ui_rt::{ImePurpose, ImeRequest, Key, Rect, Sense, Vec2, pos2};
use craft_ui_test_kit::contract::PlainStyle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const EDIT: PartKey = PartKey::new("edit");

/// The editor's document, shared with the trial.
#[derive(Default)]
struct Doc {
    text: String,
    preedit: Option<String>,
    undo: Vec<String>,
    /// Presses the editor took.
    presses: usize,
    /// The trial's app edit, applied at the editor's next run.
    app_edit: Option<String>,
}

#[derive(Component)]
struct Editor {
    #[common]
    common: Common,
    #[key(skip)]
    doc: Rc<RefCell<Doc>>,
}

impl Component for Editor {
    const ROLE: Role = Role::BUTTON;
    const INPUT: InputKind = InputKind::Editor;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: EditorStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(120.0, 24.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn text_input(&self) -> Option<ImePurpose> {
        Some(ImePurpose::Normal)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.editor(EDIT, rect, EditorPurpose::Text);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let inputs: Vec<EditorInput> = cx.editor_input().collect();
        let mut doc = self.doc.borrow_mut();
        if cx.response.clicked() {
            doc.presses += 1;
        }
        for input in inputs {
            match input {
                EditorInput::Preedit { text, .. } => doc.preedit = (!text.is_empty()).then(|| text.to_string()),
                EditorInput::ImeCommit => doc.preedit = None,
                EditorInput::Text(text) => {
                    let before = doc.text.clone();
                    doc.undo.push(before);
                    doc.text.push_str(&text);
                }
                EditorInput::Edit(EditCommand::Undo) => {
                    if let Some(before) = doc.undo.pop() {
                        doc.text = before;
                    }
                }
                _ => {}
            }
        }
        if let Some(text) = doc.app_edit.take() {
            doc.text = text;
            cx.ime_application_change();
        }
        let end = doc.text.len();
        let mut state = EditorState::new(cx.rect, &[], TextPos::new(0, end)).marked(doc.preedit.is_some());
        state.can_undo = !doc.undo.is_empty();
        cx.editor_state(state.surrounding(EditorSurrounding::text(&doc.text, end, end)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A window with the editor first, then a button; the board counts the app's Undo.
fn window() -> (Harness<()>, Rc<RefCell<Doc>>, Rc<Cell<usize>>) {
    let board = CommandBoard::new(Keymap::new(Vec::new(), Scopes::new(), Platform::Linux));
    let undos = Rc::new(Cell::new(0));
    let seen = Rc::clone(&undos);
    board.on(EditCommands::Undo, move |_, _| seen.set(seen.get() + 1));
    let doc = Rc::new(RefCell::new(Doc::default()));
    let shown = Rc::clone(&doc);
    let mut h = Harness::builder().with_commands(board).build_ui(move |ui: &mut Ui<'_, '_>| {
        drop(Show::show(Editor { common: Common::new(), doc: Rc::clone(&shown) }, ui));
        IconButton::new(IconId::new("go"), "Go").show(ui);
    });
    h.run();
    h.key_press(Key::Tab);
    (h, doc, undos)
}

fn requests(h: &Harness<()>) -> Vec<ImeRequest> {
    h.driver.platform.ime_for(h.window()).cloned().collect()
}

fn count(h: &Harness<()>, wanted: impl Fn(&ImeRequest) -> bool) -> usize {
    requests(h).iter().filter(|r| wanted(r)).count()
}

#[test]
fn a_preedit_then_its_commit_inserts_once() {
    let (mut h, doc, _) = window();
    h.ime_preedit("ni");
    assert_eq!(doc.borrow().preedit.as_deref(), Some("ni"), "the preedit reaches the editor");
    assert_eq!(doc.borrow().text, "", "a preedit inserts nothing");
    h.ime_commit("你");
    h.run();
    assert_eq!(doc.borrow().text, "你", "the commit inserts its text once");
    assert_eq!(doc.borrow().preedit, None);
}

#[test]
fn a_click_during_a_composition_commits_it_ends_it_and_lands() {
    let (mut h, doc, _) = window();
    h.ime_preedit("ni");
    let resets = count(&h, |r| matches!(r, ImeRequest::Reset));
    h.click_at(pos2(10.0, 10.0));
    h.run();
    assert_eq!(doc.borrow().text, "ni", "the press commits the composition into the editor");
    assert_eq!(doc.borrow().presses, 1, "the click lands, once");
    assert_eq!(count(&h, |r| matches!(r, ImeRequest::Reset)), resets + 1, "the platform's composition ends");
    // The platform's late commit of the ended composition, before its new session, is dropped.
    h.ime_commit("ni");
    h.run();
    assert_eq!(doc.borrow().text, "ni", "nothing is inserted twice");
    h.send(Input::ImeEnabled);
    h.ime_preedit("ma");
    h.ime_commit("ma");
    h.run();
    assert_eq!(doc.borrow().text, "nima", "the new session's input reaches the editor");
}

#[test]
fn an_app_change_sends_the_surrounding_text_once_and_an_ime_commit_sends_none() {
    let (mut h, doc, _) = window();
    let surrounding = |h: &Harness<()>| count(h, |r| matches!(r, ImeRequest::Surrounding { .. }));
    let at_start = surrounding(&h);
    h.ime_preedit("a");
    h.ime_commit("a");
    h.run();
    assert_eq!(doc.borrow().text, "a");
    assert_eq!(surrounding(&h), at_start, "the input method's own commit sends no surrounding text");
    doc.borrow_mut().app_edit = Some("hello".to_owned());
    h.type_text("!");
    h.run();
    assert_eq!(surrounding(&h), at_start + 1, "an app change sends it once");
    assert!(requests(&h).iter().any(|r| matches!(r, ImeRequest::Surrounding { text, .. } if text == "hello")), "with the editor's new text");
}

#[test]
fn an_undo_the_editor_cannot_do_runs_the_app_command() {
    let (mut h, doc, undos) = window();
    h.key(Key::Z, craft_ui_rt::Mods { ctrl: true, ..craft_ui_rt::Mods::NONE });
    h.run();
    assert_eq!(undos.get(), 1, "with no step of its own the editor passes Undo to the app");
    h.type_text("x");
    h.run();
    h.key(Key::Z, craft_ui_rt::Mods { ctrl: true, ..craft_ui_rt::Mods::NONE });
    h.run();
    assert_eq!(doc.borrow().text, "", "with a step the editor undoes it");
    assert_eq!(undos.get(), 1, "and the app's command does not run");
}

#[test]
fn typing_a_character_runs_only_the_field() {
    let (mut h, doc, _) = window();
    h.type_text("a");
    let report = h.last_report();
    assert_eq!(doc.borrow().text, "a");
    assert_eq!(report.frame.runs, 1, "the editor runs; the button and the window root keep their outputs");
}
