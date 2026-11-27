//! A text field on the real trait over the text step and ui-rt's editing engine (`PartsCx::text` with the plain text
//! codec), and its trials: typing runs only the field, a caret blink repaints and builds nothing, Paste goes through
//! the platform clipboard, and a masked field never shows its text to the access tree. Relations only, no times.

use craft_behavior::{PartKey, PlainText};
use craft_ui_contract::contract::{Bind, BindMut, Common, Component, Cx, EditResponse, Input, MeasureCx, MemoView, PartsCx, Role, Show, Ui};
use craft_ui_contract::primitives::{TextKey, TextOptions};
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::testing::Harness;
use craft_ui_rt::{Key, Mods, Rect, Sense, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const TEXT: TextKey<String> = TextKey::new(PartKey::new("text"));

/// A field editing a string: typing, the input method, the pointer, the edit commands and the engine's undo.
#[derive(Component)]
pub struct TextField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, String>,
    multiline: bool,
    password: bool,
}

impl<'a> TextField<'a> {
    /// A single-line field editing `value`.
    pub fn new(value: Bind<'a, String>) -> TextField<'a> {
        TextField { common: Common::new(), value, multiline: false, password: false }
    }

    /// Enter makes a line break and Up and Down move between rows.
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// The field masks its text and asks the platform for a password session.
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Shows the field.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> EditResponse<String> {
        Show::show(self, ui)
    }
}

impl Component for TextField<'_> {
    const ROLE: Role = Role::TEXT_INPUT;
    const INPUT: Input = Input::Text;
    type Style = PlainStyle;
    type Output = EditResponse<String>;

    fn style(_: &StyleCx<'_>, _: TextFieldStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let lines = if self.multiline { 3.0 } else { 1.0 };
        Vec2::new(12.0 * cx.tokens.size.button, lines * cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn text_input(&self) -> Option<craft_ui_rt::ImePurpose> {
        Some(if self.password { craft_ui_rt::ImePurpose::Password } else { craft_ui_rt::ImePurpose::Normal })
    }

    fn bind(&mut self) -> Option<BindMut<'_, String>> {
        Some(self.value.reborrow())
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let value = cx.value().cloned().unwrap_or_default();
        let options = TextOptions { lines: self.multiline.then_some(3), mask: self.password, ..TextOptions::default() };
        cx.text(TEXT, rect, &PlainText, &value, &options);
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}

/// A window with a field in a view and a second view beside it; the trial counts both views' runs.
struct Window {
    h: Harness<()>,
    text: Rc<RefCell<String>>,
    field_runs: Rc<Cell<usize>>,
    other_runs: Rc<Cell<usize>>,
    field: Rc<Cell<Rect>>,
}

fn window(password: bool) -> Window {
    let text = Rc::new(RefCell::new(String::new()));
    let (field_runs, other_runs, field) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)), Rc::new(Cell::new(Rect::NOTHING)));
    let (t, fr, or, f) = (Rc::clone(&text), Rc::clone(&field_runs), Rc::clone(&other_runs), Rc::clone(&field));
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        let mut value = t.borrow().clone();
        let rect = MemoView::new(value.clone()).show(ui, |ui, _| {
            fr.set(fr.get() + 1);
            let field = TextField::new(Bind::bound(&mut value));
            let field = if password { field.password() } else { field };
            field.show(ui).response.rect
        });
        f.set(rect);
        *t.borrow_mut() = value;
        MemoView::new(()).show(ui, |_, _| or.set(or.get() + 1));
    });
    h.run();
    h.key_press(Key::Tab);
    h.run();
    Window { h, text, field_runs, other_runs, field }
}

#[test]
fn typing_runs_only_the_field() {
    let mut w = window(false);
    let other = w.other_runs.get();
    w.h.type_text("a");
    w.h.run();
    assert_eq!(*w.text.borrow(), "a", "the typed character reaches the binding");
    assert_eq!(w.other_runs.get(), other, "the other view never runs for a typed character");
    let field_runs = w.field_runs.get();
    w.h.type_text("b");
    w.h.run();
    assert_eq!(*w.text.borrow(), "ab");
    assert!(w.field_runs.get() <= field_runs + 1, "the field's view reruns at most once, for its changed value");
}

#[test]
fn a_caret_blink_repaints_and_builds_nothing() {
    let mut w = window(false);
    w.h.type_text("a");
    w.h.run();
    let (field_runs, other_runs) = (w.field_runs.get(), w.other_runs.get());
    // Past the solid half period: the caret's first blink edge.
    let report = w.h.step_dt(0.6);
    assert_eq!(report.frame.runs, 0, "a blink edge runs no node");
    assert!(report.frame.painted >= 1, "the field repaints its caret");
    assert_eq!((w.field_runs.get(), w.other_runs.get()), (field_runs, other_runs));
    assert!(w.field.get().width() > 0.0);
}

#[test]
fn paste_reaches_the_field_through_the_platform_clipboard() {
    let mut w = window(false);
    w.h.driver.platform.set_clipboard_text("pasted\ntext");
    w.h.key(Key::V, Mods { cmd: true, ..Mods::NONE });
    w.h.run();
    assert_eq!(*w.text.borrow(), "pasted text", "a single line pastes the clipboard's text, its line break a space");
}

#[test]
fn a_masked_field_never_shows_its_text_to_the_access_tree() {
    let mut w = window(true);
    w.h.type_text("secret");
    w.h.run();
    assert_eq!(*w.text.borrow(), "secret");
    let tree = format!("{:?}", w.h.nodes());
    assert!(!tree.contains("secret"), "the access tree holds no part of a masked text: {tree}");
}
