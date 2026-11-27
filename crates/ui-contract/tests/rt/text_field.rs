//! A text field on the real trait over the text step and ui-rt's editing engine (`PartsCx::text` with the plain text
//! codec), and its trials: typing runs only the field, a caret blink repaints and builds nothing, Paste goes through
//! the platform clipboard, and a masked field never shows its text to the access tree. Relations only, no times.

use craft_behavior::{PartKey, PlainText};
use craft_ui_contract::contract::overlay::{Overlay, OverlayUi, Overlays as _};
use craft_ui_contract::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Input, MeasureCx, MemoKey, MemoProps, MemoView, PartsCx, Response, Role, Show, Ui,
};
use craft_ui_contract::primitives::{TextKey, TextOptions};
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::testing::Harness;
use craft_ui_rt::{Key, Mods, Rect, Sense, Vec2};
use craft_ui_test_kit::contract::PlainStyle;

use crate::skeleton::Chip;
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
    holds: bool,
    /// The field shows a dropdown with one row while this is set, and reports the row's rect there.
    #[key(skip)]
    list: Option<Rc<Cell<Rect>>>,
}

impl<'a> TextField<'a> {
    /// A single-line field editing `value`.
    pub fn new(value: Bind<'a, String>) -> TextField<'a> {
        TextField { common: Common::new(), value, multiline: false, password: false, holds: false, list: None }
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

    /// A press on a part declared `holds_drafts`, or in a popup above the field, keeps its focus and its draft.
    pub fn holds_drafts(mut self) -> Self {
        self.holds = true;
        self
    }

    /// The field shows a dropdown with one row, as a completion list does, reporting the row's rect to `row`.
    pub fn list(mut self, row: Rc<Cell<Rect>>) -> Self {
        self.list = Some(row);
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
        Sense::HOVER | Sense::CLICK | Sense::FOCUS
    }

    fn text_input(&self) -> Option<craft_ui_rt::ImePurpose> {
        Some(if self.password { craft_ui_rt::ImePurpose::Password } else { craft_ui_rt::ImePurpose::Normal })
    }

    fn bind(&mut self) -> Option<BindMut<'_, String>> {
        Some(self.value.reborrow())
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let value = cx.value().cloned().unwrap_or_default();
        let options = TextOptions { lines: self.multiline.then_some(3), mask: self.password, holds_drafts: self.holds, ..TextOptions::default() };
        cx.text(TEXT, rect, &PlainText, &value, &options);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if self.list.is_some() {
            cx.popup("list").open(cx);
        }
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let Some(row) = self.list.clone() else { return };
        ui.show(cx.popup("list"), Overlay::dropdown(), move |ui, _| row.set(Chip::new("Completion").show(ui).rect));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}

/// The field's view: keyed by the text it edits and its masking; it counts its runs.
struct FieldView<'a> {
    value: &'a mut String,
    password: bool,
    runs: Rc<Cell<usize>>,
}

impl MemoProps for FieldView<'_> {
    fn key(&self, key: &mut MemoKey) {
        key.feed_hash(self.value.as_str());
        key.feed_hash(&self.password);
    }
}

fn field_view(props: &mut FieldView<'_>, ui: &mut Ui) -> Rect {
    props.runs.set(props.runs.get() + 1);
    let field = TextField::new(Bind::bound(&mut *props.value));
    let field = if props.password { field.password() } else { field };
    field.show(ui).response.rect
}

/// A view with no input but its run counter, which feeds nothing.
struct Runs(Rc<Cell<usize>>);

impl MemoProps for Runs {
    fn key(&self, _: &mut MemoKey) {}
}

fn other_view(runs: &mut Runs, _: &mut Ui) {
    runs.0.set(runs.0.get() + 1);
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
        let props = FieldView { value: &mut value, password, runs: Rc::clone(&fr) };
        f.set(MemoView::new("field", props, field_view).show(ui).expect("in view").value);
        *t.borrow_mut() = value;
        let _ = MemoView::new("other", Runs(Rc::clone(&or)), other_view).show(ui);
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

/// A field whose own shape scrubs its value by a drag (it senses a drag on its root): a press that travels is a scrub,
/// and only a click without travel enters the text.
#[derive(Component)]
pub struct ScrubField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, String>,
}

impl Component for ScrubField<'_> {
    const ROLE: Role = Role::TEXT_INPUT;
    const INPUT: Input = Input::Text;
    type Style = PlainStyle;
    type Output = EditResponse<String>;

    fn style(_: &StyleCx<'_>, _: ScrubFieldStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(12.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::DRAG
    }

    fn bind(&mut self) -> Option<BindMut<'_, String>> {
        Some(self.value.reborrow())
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let value = cx.value().cloned().unwrap_or_default();
        cx.text(TEXT, rect, &PlainText, &value, &TextOptions::default());
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}

#[test]
fn a_scrub_leaves_the_field_without_key_focus() {
    let text = Rc::new(RefCell::new(String::from("12")));
    let t = Rc::clone(&text);
    let field = Rc::new(Cell::new(Rect::NOTHING));
    let f = Rc::clone(&field);
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        let mut value = t.borrow().clone();
        f.set(ScrubField { common: Common::new(), value: Bind::bound(&mut value) }.show(ui).response.rect);
        *t.borrow_mut() = value;
    });
    h.run();
    h.drag_at(field.get().center(), Vec2::new(40.0, 0.0));
    h.run();
    assert!(h.focused().is_none(), "the press that scrubbed gives the key focus back, as the field always did");
    h.click_at(field.get().center());
    h.run();
    assert!(h.focused().is_some(), "a click without travel enters the text");
}

/// Two fields; the first records each run's `left_by_tab`.
#[test]
fn tab_off_a_field_reports_left_by_tab() {
    let left = Rc::new(Cell::new(None));
    let l = Rc::clone(&left);
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        let (mut a, mut b) = (String::new(), String::new());
        let first = TextField::new(Bind::bound(&mut a)).show(ui);
        if first.left_by_tab.is_some() {
            l.set(first.left_by_tab);
        }
        TextField::new(Bind::bound(&mut b)).show(ui);
    });
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(left.get(), None, "focus arriving is no Tab off the field");
    h.type_text("x");
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(left.get(), Some(true), "Tab forward off the field reports its direction");
}

#[test]
fn a_wheel_over_a_multi_line_field_is_the_field_s_alone() {
    let text = Rc::new(RefCell::new(String::new()));
    let t = Rc::clone(&text);
    let (outer, field) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(Rect::NOTHING)));
    let (o, f) = (Rc::clone(&outer), Rc::clone(&field));
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        o.set(o.get() + 1);
        let mut value = t.borrow().clone();
        f.set(TextField::new(Bind::bound(&mut value)).multiline().show(ui).response.rect);
        *t.borrow_mut() = value;
    });
    h.run();
    h.key_press(Key::Tab);
    h.type_text("1\n2\n3\n4\n5\n6\n7\n8");
    h.run();
    let before = outer.get();
    h.driver
        .platform
        .inject(h.window(), craft_ui_platform_headless::Input::Wheel { pos: field.get().center(), delta: Vec2::new(0.0, 30.0), mods: Mods::NONE });
    let report = h.step_now();
    assert!(report.frame.runs >= 1, "the field takes the wheel over its text");
    assert_eq!(outer.get(), before, "the view around runs nothing for a wheel inside the field");
    h.type_text("9");
    h.run();
    assert!(text.borrow().ends_with("89"), "the wheel scrolled the text and left the caret where it was");
}

/// A menu bar stand-in: a stop that registers as the window's menu bar and counts the menu keys it gets.
#[derive(Component)]
pub struct Bar {
    #[common]
    common: Common,
    #[key(skip)]
    keys: Rc<Cell<usize>>,
}

impl Component for Bar {
    const ROLE: Role = Role::MENU_BAR;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: BarStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(10.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        use craft_ui_contract::contract::Focus as _;
        cx.register_menu_bar();
        let keys = cx.take_menu_keys().len();
        self.keys.set(self.keys.get() + keys);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn f10_focuses_the_menu_bar_and_escape_gives_the_field_back_its_selection() {
    let text = Rc::new(RefCell::new(String::from("word")));
    let keys = Rc::new(Cell::new(0));
    let (t, k) = (Rc::clone(&text), Rc::clone(&keys));
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        Bar { common: Common::new(), keys: Rc::clone(&k) }.show(ui);
        let mut value = t.borrow().clone();
        TextField::new(Bind::bound(&mut value)).show(ui);
        *t.borrow_mut() = value;
    });
    h.run();
    h.key_press(Key::Tab);
    h.key_press(Key::Tab);
    h.run();
    h.key(Key::A, Mods { cmd: true, ..Mods::NONE });
    h.run();
    let field = h.focused().expect("the field holds the focus");
    h.key_press(Key::F10);
    h.run();
    assert_eq!(keys.get(), 1, "F10 reaches the menu bar as a menu key");
    assert_ne!(h.focused().map(|t| t.id), Some(field.id), "the bar takes the focus");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.focused().map(|t| t.id), Some(field.id), "Escape on the bar gives the focus back to the field");
    h.type_text("x");
    h.run();
    assert_eq!(*text.borrow(), "x", "the field kept its whole-text selection while the bar held the focus");
}

/// A stop with one part a press takes that is declared `holds_drafts`, as a canvas's page is.
#[derive(Component)]
pub struct Pad {
    #[common]
    common: Common,
    #[key(skip)]
    rect: Rc<Cell<Rect>>,
}

impl Component for Pad {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PadStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(10.0 * cx.tokens.size.button, 3.0 * cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        self.rect.set(rect);
        cx.part(PartKey::new("page"), rect).sense(Sense::CLICK).holds_drafts();
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_press_on_a_draft_holding_part_keeps_the_field_s_focus_and_draft() {
    let text = Rc::new(RefCell::new(String::new()));
    let pad = Rc::new(Cell::new(Rect::NOTHING));
    let (t, p) = (Rc::clone(&text), Rc::clone(&pad));
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        let mut value = t.borrow().clone();
        TextField::new(Bind::bound(&mut value)).holds_drafts().show(ui);
        *t.borrow_mut() = value;
        Pad { common: Common::new(), rect: Rc::clone(&p) }.show(ui);
        Chip::new("Other").show(ui);
    });
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let field = h.focused().expect("Tab focuses the field");
    h.type_text("ab");
    h.run();
    h.click_at(pad.get().center());
    h.run();
    assert_eq!(h.focused().map(|t| t.id), Some(field.id), "a press on the holding part, which takes presses, leaves the field its focus");
    assert_eq!(*text.borrow(), "ab", "the press keeps the draft");
    let other = h.by_label("Other").expect("the chip");
    h.click(&other);
    h.run();
    assert_ne!(h.focused().map(|t| t.id), Some(field.id), "a press on a plain stop takes the focus");
}

#[test]
fn a_press_in_a_popup_above_the_focused_field_holds_its_draft() {
    let text = Rc::new(RefCell::new(String::new()));
    let row = Rc::new(Cell::new(Rect::NOTHING));
    let (t, r) = (Rc::clone(&text), Rc::clone(&row));
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        let mut value = t.borrow().clone();
        TextField::new(Bind::bound(&mut value)).holds_drafts().list(Rc::clone(&r)).show(ui);
        *t.borrow_mut() = value;
    });
    h.run();
    h.key_press(Key::Tab);
    h.type_text("ab");
    h.run();
    let field = h.focused().expect("the field holds the focus");
    assert!(row.get().is_positive(), "the dropdown shows its row");
    h.click_at(row.get().center());
    h.run();
    assert_eq!(h.focused().map(|t| t.id), Some(field.id), "a press on the popup's row, a stop, leaves the field its focus");
    assert_eq!(*text.borrow(), "ab", "the press keeps the draft");
}

/// A window with the menu bar stand-in and a field (`holds`: one that holds drafts), off a Mac.
fn bar_and_field(holds: bool) -> (Harness<()>, Rc<Cell<usize>>) {
    let keys = Rc::new(Cell::new(0));
    let k = Rc::clone(&keys);
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        Bar { common: Common::new(), keys: Rc::clone(&k) }.show(ui);
        let mut value = String::new();
        let field = TextField::new(Bind::bound(&mut value));
        if holds { field.holds_drafts() } else { field }.show(ui);
    });
    h.os(craft_host::target::Os::Windows);
    h.run();
    h.key_press(Key::Tab);
    h.key_press(Key::Tab);
    h.run();
    (h, keys)
}

fn alt(h: &mut Harness<()>, down: bool) {
    let mods = if down { Mods { alt: true, ..Mods::NONE } } else { Mods::NONE };
    h.send(craft_ui_platform_headless::Input::Modifiers(mods));
}

#[test]
fn a_lone_alt_opens_the_menu_bar_and_alt_with_a_letter_does_not() {
    let (mut h, keys) = bar_and_field(false);
    let field = h.focused().expect("the field holds the focus");
    alt(&mut h, true);
    alt(&mut h, false);
    h.run();
    assert_eq!(keys.get(), 1, "a lone Alt reaches the menu bar as a menu key");
    assert_ne!(h.focused().map(|t| t.id), Some(field.id), "the bar takes the focus");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.focused().map(|t| t.id), Some(field.id), "Escape gives the focus back");
    alt(&mut h, true);
    h.key(Key::Z, Mods { alt: true, ..Mods::NONE });
    alt(&mut h, false);
    h.run();
    assert_eq!(keys.get(), 2, "Alt with a letter is a mnemonic, and its release is no lone Alt");
    assert_eq!(h.focused().map(|t| t.id), Some(field.id), "the field keeps the focus");
}

#[test]
fn a_field_that_holds_drafts_keeps_the_focus_on_a_lone_alt() {
    let (mut h, keys) = bar_and_field(true);
    let field = h.focused().expect("the field holds the focus");
    alt(&mut h, true);
    alt(&mut h, false);
    h.run();
    assert_eq!(keys.get(), 0, "the field takes the lone Alt (and alerts); the bar gets nothing");
    assert_eq!(h.focused().map(|t| t.id), Some(field.id), "the field keeps the focus");
}

/// A menu bar stand-in whose menu key opens a menu with one row; picking the row returns the focus out of the menus.
#[derive(Component)]
pub struct MenuBarWithMenu {
    #[common]
    common: Common,
    #[key(skip)]
    row: Rc<Cell<Rect>>,
}

impl Component for MenuBarWithMenu {
    const ROLE: Role = Role::MENU_BAR;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: MenuBarWithMenuStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(10.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        use craft_ui_contract::contract::Focus as _;
        cx.register_menu_bar();
        if !cx.take_menu_keys().is_empty() {
            cx.popup("menu").open(cx);
        }
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let row = Rc::clone(&self.row);
        ui.show(cx.popup("menu"), Overlay::menu(), move |ui, _| {
            let picked = Chip::new("Pick").show(ui);
            row.set(picked.rect);
            if picked.clicked()
                && let Some(menu) = ui.running_menu()
            {
                ui.return_from_pick(craft_ui_contract::runtime::focus::PickedIn::Menus(menu));
            }
        });
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_menu_pick_returns_the_focus_to_the_stop_the_bar_took_it_from() {
    let row = Rc::new(Cell::new(Rect::NOTHING));
    let r = Rc::clone(&row);
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        MenuBarWithMenu { common: Common::new(), row: Rc::clone(&r) }.show(ui);
        let mut value = String::new();
        TextField::new(Bind::bound(&mut value)).show(ui);
    });
    h.run();
    h.key_press(Key::Tab);
    h.key_press(Key::Tab);
    h.run();
    let field = h.focused().expect("the field holds the focus");
    h.key_press(Key::F10);
    h.run();
    assert!(row.get().is_positive(), "F10 opened the bar's menu");
    h.click_at(row.get().center());
    h.run();
    assert_eq!(h.focused().map(|t| t.id), Some(field.id), "the pick returns the focus past the bar to the field");
}

/// A component whose `update` lays document text out (`Cx::layout_doc`), counting its runs.
#[derive(Component)]
pub struct DocProbe {
    #[common]
    common: Common,
    #[key(skip)]
    runs: Rc<Cell<usize>>,
}

impl Component for DocProbe {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: DocProbeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(10.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
        let ink = craft_ui_contract::style::DataColor::new(egui::Color32::BLACK);
        let runs = [craft_ui_contract::primitives::DocRun::new("probe", craft_ui_contract::fonts::FontRef::proportional(), 12.0, ink)];
        let _ = cx.layout_doc(&craft_ui_contract::primitives::DocText::new(&runs));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn layout_doc_in_update_runs_no_extra_build() {
    let runs = Rc::new(Cell::new(0));
    let r = Rc::clone(&runs);
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| {
        DocProbe { common: Common::new(), runs: Rc::clone(&r) }.show(ui);
        Chip::new("Other").show(ui);
    });
    h.run();
    assert_eq!(runs.get(), 1, "the lent engine asks for no second build of the node");
    let other = h.by_label("Other").expect("the chip");
    h.click(&other);
    h.run();
    assert_eq!(runs.get(), 1, "a press elsewhere runs the node's update no more");
}
