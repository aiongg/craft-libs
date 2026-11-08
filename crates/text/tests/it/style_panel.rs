//! The styles panel: what a click, a key, a drop and the panel menu's commands ask the app to do.

use craft_behavior::Selection;
use craft_text::ui::style_panel::{TextStyleAction, TextStyleCommand, TextStyleKind, TextStyleRow, TextStylesPanel};
use craft_ui::contract::Bind;
use craft_ui::runtime::commands::{self, FireSource};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    actions: Rc<RefCell<Vec<TextStyleAction>>>,
    statuses: Rc<RefCell<Vec<String>>>,
    selection: Rc<RefCell<Selection<u64>>>,
    text_selected: Rc<Cell<bool>>,
}

impl Rig {
    fn new(kind: TextStyleKind, names: &[&str], current: Option<u64>, overridden: bool, highlighting: bool) -> Rig {
        let rows: Vec<TextStyleRow> = names.iter().enumerate().map(|(id, name)| TextStyleRow::new(id as u64, *name)).collect();
        let actions = Rc::new(RefCell::new(Vec::new()));
        let statuses = Rc::new(RefCell::new(Vec::new()));
        let selection = Rc::new(RefCell::new(Selection::default()));
        let text_selected = Rc::new(Cell::new(true));
        let (sink, notes, kept, text) = (actions.clone(), statuses.clone(), selection.clone(), text_selected.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut selected = kept.borrow().clone();
            let out = TextStylesPanel::new("Styles", kind, &rows, Bind::bound(&mut selected))
                .current(current, overridden)
                .target(true)
                .text_selected(text.get())
                .highlighting(highlighting)
                .show(ui);
            *kept.borrow_mut() = selected;
            sink.borrow_mut().extend(out.action);
            notes.borrow_mut().extend(out.status.map(|status| status.as_str().to_owned()));
        });
        fixture.settle();
        Rig { fixture, actions, statuses, selection, text_selected }
    }

    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("`{label}` is shown"));
        self.fixture.click(&target);
    }

    fn take(&self) -> Vec<TextStyleAction> {
        std::mem::take(&mut *self.actions.borrow_mut())
    }
}

/// A click applies, an Alt-click applies and clears overrides, a double click asks for Style Options, and "+"
/// follows an overridden current style.
#[trial]
fn rows_apply_clear_overrides_and_open_options() {
    let mut rig = Rig::new(TextStyleKind::Paragraph, &["Basic Paragraph", "Heading", "Body"], Some(1), true, false);
    assert!(rig.fixture.by_label("Heading, overridden").is_some());

    rig.click("Body");
    assert_eq!(rig.take(), [TextStyleAction::apply(2, false)]);

    rig.fixture.hold(Modifiers::ALT);
    rig.click("Heading, overridden");
    rig.fixture.hold(Modifiers::NONE);
    assert_eq!(rig.take(), [TextStyleAction::apply(1, true)]);

    rig.click("Body");
    rig.click("Body");
    assert_eq!(rig.take().last(), Some(&TextStyleAction::Options(2)));
}

/// Delete and Backspace both delete the picked style while the list has the keyboard.
#[trial]
fn delete_and_backspace_delete_the_picked_style() {
    let mut rig = Rig::new(TextStyleKind::Paragraph, &["Basic Paragraph", "Heading", "Body"], Some(1), false, false);
    rig.click("Body");
    for key in [Key::Delete, Key::Backspace] {
        rig.take();
        rig.fixture.key(key, Modifiers::NONE);
        assert_eq!(rig.take(), [TextStyleAction::Delete(2)], "{key:?}");
    }
}

/// A character style click at a bare caret applies nothing and says "Select text…"; Delete and Redefine do nothing
/// for character style 0.
#[trial]
fn character_style_rules_at_a_caret_and_for_none() {
    let mut rig = Rig::new(TextStyleKind::Character, &["None", "Emphasis"], Some(0), false, false);
    rig.text_selected.set(false);
    rig.click("Emphasis");
    assert!(rig.take().is_empty());
    assert_eq!(*rig.statuses.borrow(), ["Select text to apply a character style"]);

    rig.text_selected.set(true);
    rig.click("None");
    assert_eq!(rig.take(), [TextStyleAction::apply(0, false)]);

    rig.fixture.key(Key::Delete, Modifiers::NONE);
    rig.click("Redefine style by current selection");
    assert!(rig.take().is_empty());
}

/// The panel menu's commands are enabled under the footer's conditions, and a command run from the menu comes back
/// as the footer button's action.
#[trial]
fn panel_menu_follows_the_footers_conditions() {
    let mut rig = Rig::new(TextStyleKind::Character, &["None", "Emphasis"], Some(0), false, false);
    let enabled = |rig: &Rig, command: TextStyleCommand| commands::resolve(rig.fixture.ctx(), command).enabled;
    // Character style None is current: not overridden, not redefinable, not deletable or editable.
    assert!(!enabled(&rig, TextStyleCommand::ClearOverride));
    assert!(!enabled(&rig, TextStyleCommand::Redefine));
    assert!(!enabled(&rig, TextStyleCommand::Delete));
    assert!(!enabled(&rig, TextStyleCommand::Options));
    assert!(enabled(&rig, TextStyleCommand::New));

    rig.click("Emphasis");
    rig.fixture.idle(2);
    assert!(enabled(&rig, TextStyleCommand::Delete));
    rig.take();
    assert!(commands::fire(rig.fixture.ctx(), TextStyleCommand::Options, FireSource::Menu));
    rig.fixture.idle(2);
    assert_eq!(rig.take(), [TextStyleAction::Options(1)]);
}

/// With a highlight, Shift-click selects a range and Ctrl-click toggles one without applying; Delete acts on all of
/// them; a selected row dropped on New duplicates every selected style, an unselected one only itself.
#[trial]
fn highlighted_rows_act_together() {
    let mut rig = Rig::new(TextStyleKind::Paragraph, &["Basic Paragraph", "A", "B", "C", "D"], None, false, true);
    rig.click("A");
    assert_eq!(rig.take(), [TextStyleAction::apply(1, false)]);

    rig.fixture.hold(Modifiers::SHIFT);
    rig.click("C");
    rig.fixture.hold(Modifiers::COMMAND);
    rig.click("D");
    rig.fixture.hold(Modifiers::NONE);
    let mut items = rig.selection.borrow().items.clone();
    items.sort_unstable();
    assert_eq!(items, [1, 2, 3, 4]);
    assert!(rig.take().is_empty());

    rig.fixture.key(Key::Delete, Modifiers::NONE);
    assert_eq!(rig.take(), [TextStyleAction::DeleteHighlighted]);

    let new = rig.fixture.by_label("Create new style (from the selected type)").expect("the New button is shown");
    let from = rig.fixture.by_label("B").expect("row B is shown");
    rig.fixture.drag(&from, new.center() - from.center());
    assert_eq!(rig.take(), [TextStyleAction::DuplicateHighlighted]);

    rig.click("Basic Paragraph");
    rig.take();
    let from = rig.fixture.by_label("D").expect("row D is shown");
    rig.fixture.drag(&from, new.center() - from.center());
    assert_eq!(rig.take(), [TextStyleAction::Duplicate(4)]);
}
