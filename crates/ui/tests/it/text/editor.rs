//! The editor contract on `CodeEditor`: it applies one ordered input stream to its document, a read-only editor
//! receives only inputs that change no text, and its visible lines become AccessKit run nodes.

use craft_behavior::TextPos;
use craft_ui::contract::{Props as _, Show as _};
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::examples::code_editor::{CodeDocument, CodeEditor};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::text::{run_node, text_selection_request};
use craft_ui::testing::{Frames, trial};
use egui::{Event, Id, Key, Modifiers, Pos2, Rect};
use std::cell::Cell;

const AT: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(210.0, 90.0) };

struct Rig {
    frames: Frames,
    doc: CodeDocument,
    id: Cell<Id>,
    read_only: bool,
}

impl Rig {
    fn new(text: &str) -> Rig {
        let mut rig = Rig { frames: Frames::new(), doc: CodeDocument::new(text), id: Cell::new(Id::NULL), read_only: false };
        for _ in 0..3 {
            rig.frame(Vec::new());
        }
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let (doc, id, read_only) = (&mut self.doc, &self.id, self.read_only);
        self.frames.frame(events, |ui| show(ui, doc, id, read_only));
    }

    fn focus(&mut self) {
        let (doc, id, read_only) = (&mut self.doc, &self.id, self.read_only);
        click(&mut self.frames, AT.center(), &mut |ui| show(ui, doc, id, read_only));
    }
}

fn show(ui: &mut egui::Ui, doc: &mut CodeDocument, id: &Cell<Id>, read_only: bool) {
    let out = CodeEditor::new(doc).label("Source").read_only(read_only).id_salt("code").show_at(ui, AT);
    id.set(out.response.id);
}

/// Text, Backspace, Tab and Enter reach a code editor as one input stream applied in `update`; a read-only editor
/// still moves its caret but never changes its text.
#[trial]
fn editor_input_reaches_a_code_editor_and_read_only_filtering_drops_edits() {
    let mut rig = Rig::new("fn");
    rig.focus();
    rig.frame(vec![Event::Text("x".into())]);
    assert_eq!(rig.doc.text(), "fnx", "typed text reaches the editor in the pass it is typed");
    rig.frame(vec![key_press(Key::Backspace, Modifiers::NONE)]);
    assert_eq!(rig.doc.text(), "fn", "Backspace arrives as a delete");
    rig.frame(vec![key_press(Key::Tab, Modifiers::NONE)]);
    assert!(rig.doc.text().starts_with("fn ") && rig.doc.text().len() > "fn ".len(), "Tab inserts in a code editor instead of moving focus");
    rig.frame(vec![key_press(Key::Enter, Modifiers::NONE)]);
    assert_eq!(rig.doc.caret().line, 1, "Enter splits the line");

    let (text, caret) = (rig.doc.text(), rig.doc.caret());
    rig.read_only = true;
    rig.frame(vec![Event::Text("y".into())]);
    rig.frame(vec![key_press(Key::Tab, Modifiers::NONE)]);
    rig.frame(vec![key_press(Key::Backspace, Modifiers::NONE)]);
    assert_eq!(rig.doc.text(), text, "a read-only editor changes no text");
    rig.frame(vec![key_press(Key::ArrowUp, Modifiers::NONE)]);
    assert_ne!(rig.doc.caret(), caret, "moves still arrive");
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
    rig.frame(vec![text_selection_request(id, (run, 3), (run, 5))]);
    assert_eq!(rig.doc.selection(), Some((TextPos::new(0, 3), TextPos::new(0, 5))));
}
