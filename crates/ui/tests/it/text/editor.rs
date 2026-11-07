//! The editor contract: a code editor that draws its own caret reads one ordered input stream, a read-only editor
//! receives only inputs that change no text, and its visible lines become AccessKit run nodes.

use crate::contract::kit::PlainStyle;
use craft_behavior::{EditorInput, NavMove, PartKey, Sense, TextPos};
use craft_ui::contract::{Common, Component, Cx, Input, MeasureCx, PartsCx, Props as _, Role, Show as _};
use craft_ui::primitives::{EditorLine, EditorPurpose, EditorRun, EditorState};
use craft_ui::runtime::focus::KeyUse;
use craft_ui::style::StyleCx;
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::text::{run_node, text_selection_request};
use craft_ui::testing::{Frames, trial};
use egui::{Event, Id, Key, Modifiers, Pos2, Rect, Response, Vec2};
use std::cell::{Cell, RefCell};

const EDITOR: PartKey = PartKey::new("editor");
const AT: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(210.0, 50.0) };
/// The advance of every character of the editor's monospace layout, in points.
const ADVANCE: f32 = 8.0;
const LINE_HEIGHT: f32 = 16.0;

/// The editor's document: one line, a caret as a byte offset and a revision bumped on every edit.
struct Doc {
    text: String,
    caret: usize,
    revision: u64,
}

impl Doc {
    fn new(text: &str) -> Doc {
        Doc { text: text.to_owned(), caret: text.len(), revision: 0 }
    }

    fn insert(&mut self, text: &str) {
        self.text.insert_str(self.caret, text);
        self.caret += text.len();
        self.revision += 1;
    }

    fn backspace(&mut self) {
        if let Some((at, _)) = self.text[..self.caret].char_indices().next_back() {
            self.text.remove(at);
            self.caret = at;
            self.revision += 1;
        }
    }
}

/// A code editor that applies `editor_input` to its own document and reports its line.
#[derive(craft_ui::contract::Component)]
struct CodeEditor<'a> {
    #[common]
    common: Common,
    doc: &'a mut Doc,
    seen: &'a RefCell<Vec<EditorInput>>,
    shown_id: &'a Cell<Id>,
}

impl Component for CodeEditor<'_> {
    const ROLE: Role = Role::MULTILINE_TEXT_INPUT;
    const INPUT: Input = Input::Editor;
    const KEYS: KeyUse = KeyUse::CODE;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: CodeEditorStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        AT.size()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.editor(EDITOR, rect, EditorPurpose::Code);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.shown_id.set(cx.id);
        let inputs: Vec<EditorInput> = cx.editor_input().collect();
        for input in inputs {
            self.seen.borrow_mut().push(input.clone());
            match input {
                EditorInput::Text(text) => self.doc.insert(&text),
                EditorInput::Tab => self.doc.insert("\t"),
                EditorInput::Delete { forward: false, .. } => self.doc.backspace(),
                EditorInput::Select { focus, .. } => self.doc.caret = focus.byte,
                _ => {}
            }
        }
        let text = self.doc.text.clone();
        let positions: Vec<f32> = (0..text.chars().count()).map(|at| at as f32 * ADVANCE).collect();
        let widths = vec![ADVANCE; positions.len()];
        let runs = [EditorRun::new(0..text.len(), false, &positions, &widths)];
        let rect = Rect::from_min_size(cx.rect.min, Vec2::new(cx.rect.width(), LINE_HEIGHT));
        let lines = [EditorLine::new(&text, rect, LINE_HEIGHT, &runs)];
        let caret_x = text[..self.doc.caret].chars().count() as f32 * ADVANCE;
        let caret = Rect::from_min_size(rect.min + Vec2::new(caret_x, 0.0), Vec2::new(1.0, LINE_HEIGHT));
        cx.editor_state(EditorState::new(caret, &lines, TextPos::new(0, self.doc.caret), self.doc.revision));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct Rig {
    frames: Frames,
    doc: Doc,
    seen: RefCell<Vec<EditorInput>>,
    id: Cell<Id>,
    read_only: bool,
}

impl Rig {
    fn new(text: &str) -> Rig {
        let mut rig = Rig { frames: Frames::new(), doc: Doc::new(text), seen: RefCell::default(), id: Cell::new(Id::NULL), read_only: false };
        for _ in 0..3 {
            rig.frame(Vec::new());
        }
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let (doc, seen, id, read_only) = (&mut self.doc, &self.seen, &self.id, self.read_only);
        self.frames.frame(events, |ui| {
            CodeEditor { common: Common::new(), doc: &mut *doc, seen, shown_id: id }.read_only(read_only).id_salt("code").show_at(ui, AT);
        });
    }

    fn focus(&mut self) {
        let (doc, seen, id, read_only) = (&mut self.doc, &self.seen, &self.id, self.read_only);
        click(&mut self.frames, AT.center(), &mut |ui| {
            CodeEditor { common: Common::new(), doc: &mut *doc, seen, shown_id: id }.read_only(read_only).id_salt("code").show_at(ui, AT);
        });
    }
}

/// Text, Tab and Backspace reach a code editor as one input stream and it applies them in `update`; a read-only editor
/// still gets moves but never text, Tab or deletes.
#[trial]
fn editor_input_reaches_a_code_editor_and_read_only_filtering_drops_edits() {
    let mut rig = Rig::new("fn");
    rig.focus();
    rig.frame(vec![Event::Text("x".into())]);
    assert_eq!(rig.doc.text, "fnx", "typed text reaches the editor in the pass it is typed");
    rig.frame(vec![key_press(Key::Tab, Modifiers::NONE)]);
    assert_eq!(rig.doc.text, "fnx\t", "Tab inserts in a code editor instead of moving focus");
    rig.frame(vec![key_press(Key::Backspace, Modifiers::NONE)]);
    assert_eq!(rig.doc.text, "fnx", "Backspace arrives as a delete");

    rig.read_only = true;
    rig.seen.borrow_mut().clear();
    rig.frame(vec![Event::Text("y".into())]);
    rig.frame(vec![key_press(Key::Tab, Modifiers::NONE)]);
    rig.frame(vec![key_press(Key::Backspace, Modifiers::NONE)]);
    rig.frame(vec![key_press(Key::ArrowLeft, Modifiers::NONE)]);
    assert_eq!(rig.doc.text, "fnx", "a read-only editor changes no text");
    let seen = rig.seen.borrow();
    assert!(seen.iter().any(|input| matches!(input, EditorInput::Move(NavMove::Back, _))), "moves still arrive: {seen:?}");
    assert!(seen.iter().all(EditorInput::is_read_only_safe), "nothing that changes text arrives: {seen:?}");
}

/// The visible line is one run node with the editor's text, and `SetTextSelection` on it reaches the editor as a
/// `Select` of line and byte positions.
#[trial]
fn an_editor_reports_run_nodes_and_serves_set_text_selection() {
    let mut rig = Rig::new("fn main");
    rig.focus();
    rig.frame(Vec::new());
    let id = rig.id.get();
    let tree = rig.frames.tree();
    assert_eq!(tree.run_texts(id), ["fn main"], "one run node per run of each visible line");
    let run = run_node(id, 0, 0);
    let caret = tree.text_selection(id).expect("the node reports the caret");
    assert_eq!(caret, ((NodeKey::of(run), 7), (NodeKey::of(run), 7)), "the caret is a collapsed selection at the end");
    rig.seen.borrow_mut().clear();
    rig.frame(vec![text_selection_request(id, (run, 3), (run, 5))]);
    let selected: Vec<_> = rig.seen.borrow().iter().filter(|input| matches!(input, EditorInput::Select { .. })).cloned().collect();
    assert_eq!(selected, [EditorInput::select(TextPos::new(0, 3), TextPos::new(0, 5))]);
    assert_eq!(rig.doc.caret, 5, "the editor moved its caret to the focus");
}
