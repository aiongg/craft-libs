//! Enter and Escape reach a tool holding a pending edit before the canvas has focus (Photoshop commits a transform
//! with Enter without a click on the canvas first); a focused field beside the canvas keeps its own Enter. On the
//! focused canvas, Backspace and Delete reach a tool with Alt held or not (Photoshop's polygonal lasso removes its
//! last point while Alt holds the freehand switch) while it keeps the keyboard, and are the app's otherwise (Fill);
//! Enter and Escape reach a tool as `Submit` and `Cancel` with Alt carried, and Shift+Delete is no delete
//! (Photoshop opens Fill, which the app binds).

use craft_behavior::CancelReason;
use craft_host::target::Os;
use craft_ui::canvas::{Canvas, CanvasTool, ToolAction, ToolContext, ToolHost, ToolKey, ToolPath, ToolPointer};
use craft_ui::contract::{Bind, CommandSet, Commands as _, Role, Show as _, ShowContainer as _};
use craft_ui::layout_components::Stack;
use craft_ui::primitives::Draw;
use craft_ui::sections::Section;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, ToolFixture, trial};
use craft_ui::text::TextField;
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
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
            let _ = ToolHost::new().run(ui, &canvas.output, &mut *held.borrow_mut());
        });
    });
    fixture.settle();
    let field = fixture.by_label("Name").expect("the field shows");
    fixture.click(&field).settle();
    fixture.key(Key::Enter, Modifiers::NONE);
    let tool = tool.borrow();
    assert!(tool.answers.is_empty() && tool.pending, "Enter in the focused field is the field's: {:?}", tool.answers);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    Delete { forward: bool },
    Submit { alt: bool },
    Cancel { alt: bool },
    Kept,
}

/// A polygonal outline being drawn: it takes every key the host offers, and keeps the keyboard while `keeps` (its
/// outline is open).
struct Outline {
    keeps: bool,
    seen: Vec<Seen>,
}

impl CanvasTool for Outline {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        if self.keeps {
            cx.keep_keys();
        }
    }

    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        let seen = match key {
            ToolKey::Delete { forward, .. } => Seen::Delete { forward: *forward },
            ToolKey::Submit => Seen::Submit { alt: cx.mods.alt },
            ToolKey::Cancel => Seen::Cancel { alt: cx.mods.alt },
            ToolKey::Kept(_) => Seen::Kept,
            _ => return false,
        };
        self.seen.push(seen);
        true
    }

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Polygonal Lasso's selection form")
    }
}

/// An outline tool on `os` whose canvas holds the keyboard.
fn focused_outline(os: Os, keeps: bool) -> ToolFixture<Outline> {
    let mut canvas = ToolFixture::new("Document", Outline { keeps, seen: Vec::new() });
    canvas.fixture().os(os).settle();
    let target = canvas.canvas();
    canvas.fixture().access(AccessActionKind::Focus, &target).settle();
    canvas
}

#[trial]
fn a_tool_with_an_outline_open_takes_backspace_and_delete_with_alt_held_on_every_platform() {
    for os in [Os::Linux, Os::Windows, Os::Mac] {
        let mut canvas = focused_outline(os, true);
        canvas.fixture().hold(Modifiers::ALT).settle();
        canvas.fixture().key(Key::Backspace, Modifiers::NONE).key(Key::Delete, Modifiers::NONE);
        canvas.fixture().hold(Modifiers::NONE).settle();
        let want = [Seen::Delete { forward: false }, Seen::Delete { forward: true }];
        assert_eq!(canvas.tool().seen, want, "{os:?}: Backspace and Delete with Alt held reach the tool");
    }
}

#[trial]
fn enter_and_escape_reach_a_tool_that_keeps_the_keyboard_as_submit_and_cancel() {
    let mut canvas = focused_outline(Os::Linux, true);
    canvas.fixture().key(Key::Enter, Modifiers::NONE).key(Key::Escape, Modifiers::NONE).key(Key::A, Modifiers::NONE);
    let want = [Seen::Submit { alt: false }, Seen::Cancel { alt: false }, Seen::Kept];
    assert_eq!(canvas.tool().seen, want, "the kept keys leave Enter and Escape to the intents");
    canvas.fixture().key(Key::Backspace, Modifiers::NONE);
    assert_eq!(canvas.tool().seen.last(), Some(&Seen::Delete { forward: false }), "Backspace stays a delete while the tool keeps the keyboard");
}

#[trial]
fn alt_enter_and_alt_escape_reach_a_tool_with_alt_held() {
    for keeps in [false, true] {
        let mut canvas = focused_outline(Os::Linux, keeps);
        canvas.fixture().hold(Modifiers::ALT).settle();
        canvas.fixture().key(Key::Enter, Modifiers::NONE).key(Key::Escape, Modifiers::NONE);
        canvas.fixture().hold(Modifiers::NONE).settle();
        let want = [Seen::Submit { alt: true }, Seen::Cancel { alt: true }];
        assert_eq!(canvas.tool().seen, want, "keeps the keyboard: {keeps}; Alt+Enter submits and Alt+Escape cancels with Alt");
    }
}

#[trial]
fn shift_delete_reaches_no_delete() {
    for keeps in [false, true] {
        let mut canvas = focused_outline(Os::Linux, keeps);
        canvas.fixture().key(Key::Delete, Modifiers::SHIFT).key(Key::Backspace, Modifiers::SHIFT);
        assert!(canvas.tool().seen.is_empty(), "keeps the keyboard: {keeps}; Shift+Delete is the app's: {:?}", canvas.tool().seen);
    }
}

/// The app's Fill with the foreground colour, bound where Photoshop binds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "fillTrial", scope = "trial.fill")]
#[non_exhaustive]
enum FillCommand {
    #[command(label = "Fill with Foreground", default = "Alt+Backspace", modal = "always")]
    Foreground,
}

#[trial]
fn alt_backspace_fills_with_no_outline_open_and_removes_a_point_with_one() {
    for keeps in [false, true] {
        let tool = Rc::new(RefCell::new(Outline { keeps, seen: Vec::new() }));
        let fills = Rc::new(Cell::new(0));
        let (held, counted) = (tool.clone(), fills.clone());
        let mut fixture = Fixture::new(move |ui| {
            Section::fixed("Document").command_scope(<FillCommand as CommandSet>::SCOPE).show(ui, |ui, cx| {
                while cx.take_command::<FillCommand>().is_some() {
                    counted.set(counted.get() + 1);
                }
                let canvas = Canvas::new("Document").show(ui, |_, _| {});
                let _ = ToolHost::new().run(ui, &canvas.output, &mut *held.borrow_mut());
            });
        });
        fixture.settle();
        let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the canvas shows");
        fixture.access(AccessActionKind::Focus, &canvas).settle();
        fixture.key(Key::Backspace, Modifiers::ALT).settle();
        let seen = tool.borrow().seen.clone();
        if keeps {
            assert_eq!((seen.as_slice(), fills.get()), ([Seen::Delete { forward: false }].as_slice(), 0), "an open outline takes the key");
        } else {
            assert_eq!((seen.as_slice(), fills.get()), ([].as_slice(), 1), "with no outline open the key is the app's");
        }
    }
}
