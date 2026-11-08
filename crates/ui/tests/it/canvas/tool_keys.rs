//! Enter and Escape reach a tool holding a pending edit before the canvas has focus (Photoshop commits a transform
//! with Enter without a click on the canvas first); a focused field beside the canvas keeps its own Enter.

use craft_behavior::CancelReason;
use craft_ui::canvas::{Canvas, CanvasTool, ToolAction, ToolContext, ToolHost, ToolKey, ToolPath, ToolPointer};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::layout_components::Stack;
use craft_ui::primitives::Draw;
use craft_ui::testing::{Fixture, ToolFixture, trial};
use craft_ui::text::TextField;
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Answer {
    Commit,
    Discard,
}

/// A transform box opened from a menu: it holds its edit until Enter commits it or Escape discards it.
struct Transform {
    pending: bool,
    answers: Vec<Answer>,
}

impl Default for Transform {
    fn default() -> Self {
        Transform { pending: true, answers: Vec::new() }
    }
}

impl CanvasTool for Transform {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn key(&mut self, key: &ToolKey, _: &mut ToolContext<'_>) -> bool {
        let answer = match key {
            ToolKey::Submit => Answer::Commit,
            ToolKey::Cancel => Answer::Discard,
            _ => return false,
        };
        if !self.pending {
            return false;
        }
        self.pending = false;
        self.answers.push(answer);
        true
    }

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Free Transform command")
    }
}

#[trial]
fn escape_discards_a_pending_edit_while_nothing_has_focus() {
    let mut canvas = ToolFixture::new("Document", Transform::default());
    canvas.fixture().settle();
    canvas.fixture().key(Key::Escape, Modifiers::NONE);
    assert_eq!(canvas.tool().answers, [Answer::Discard], "Escape reaches the tool before the canvas is clicked");
}

#[trial]
fn enter_commits_a_pending_edit_while_nothing_has_focus() {
    let mut canvas = ToolFixture::new("Document", Transform::default());
    canvas.fixture().settle();
    canvas.fixture().key(Key::Enter, Modifiers::NONE);
    assert_eq!(canvas.tool().answers, [Answer::Commit], "Enter reaches the tool before the canvas is clicked");
}

#[trial]
fn a_focused_field_beside_the_canvas_keeps_its_enter() {
    let tool = Rc::new(RefCell::new(Transform::default()));
    let held = tool.clone();
    let mut name = String::from("Layer 1");
    let mut fixture = Fixture::new(move |ui| {
        Stack::new().fill().show(ui, |ui, _| {
            TextField::new("Name", Bind::bound(&mut name)).show(ui);
            let canvas = Canvas::new("Document").show(ui, |_, _| {});
            ToolHost::new().run(ui, &canvas.output, &mut *held.borrow_mut());
        });
    });
    fixture.settle();
    let field = fixture.by_label("Name").expect("the field shows");
    fixture.click(&field).settle();
    fixture.key(Key::Enter, Modifiers::NONE);
    let tool = tool.borrow();
    assert!(tool.answers.is_empty() && tool.pending, "Enter in the focused field is the field's: {:?}", tool.answers);
}
