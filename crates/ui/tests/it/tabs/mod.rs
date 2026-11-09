//! `Tab`: which side the close button is on, and that the close button and the middle click close the tab while a
//! click on its body does not.

use craft_ui::contract::Show as _;
use craft_ui::tabs::{Tab, TabKind, TabPartEvent};
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Modifiers, PointerButton, Pos2};
use std::cell::Cell;
use std::rc::Rc;

fn rig(kind: TabKind) -> (Fixture, Rc<Cell<Option<TabPartEvent>>>) {
    let seen = Rc::new(Cell::new(None));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = Tab::new("Doc").kind(kind).closable(true).show(ui);
        if out.event.is_some() {
            shown.set(out.event);
        }
    });
    fixture.settle();
    (fixture, seen)
}

fn middle_click(fixture: &mut Fixture, at: Pos2) {
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Middle, pressed, modifiers: Modifiers::NONE };
    fixture.frame(vec![Event::PointerMoved(at)]);
    fixture.frame(vec![button(true)]);
    fixture.frame(vec![button(false)]);
    fixture.frame(Vec::new());
}

/// A middle click closes a document tab; panel tabs close from the host's menu instead.
#[trial]
fn a_middle_click_closes_a_document_tab_only() {
    let (mut documents, seen) = rig(TabKind::Document);
    let at = documents.by_label("Doc").expect("the tab is named").center();
    middle_click(&mut documents, at);
    assert_eq!(seen.get(), Some(TabPartEvent::Close));
    let (mut panels, seen) = rig(TabKind::Panel);
    let at = panels.by_label("Doc").expect("the tab is named").center();
    middle_click(&mut panels, at);
    assert_eq!(seen.get(), None);
}

// The strip and the document tabs.
mod strip;
