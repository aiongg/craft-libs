//! `Tab`: which side the close button is on, and that the close button and the middle click close the tab while a
//! click on its body does not.

use craft_host::target::Os;
use craft_ui::contract::{Props as _, Show as _};
use craft_ui::tabs::{CloseSide, Tab, TabKind, TabPartEvent, close_side};
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Modifiers, PointerButton, Pos2};
use std::cell::Cell;
use std::rc::Rc;

/// The close button is before the title on macOS and after it elsewhere, in reading order, so a mirrored strip puts
/// it on the other physical side.
#[trial]
fn the_close_side_follows_the_platform_and_the_direction() {
    assert_eq!(close_side(Os::Mac, false), CloseSide::Left);
    assert_eq!(close_side(Os::Mac, true), CloseSide::Right);
    assert_eq!(close_side(Os::Windows, false), CloseSide::Right);
    assert_eq!(close_side(Os::Linux, true), CloseSide::Left);
}

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

/// A press on the close button closes the tab and a press on the tab does not.
#[trial]
fn a_press_on_the_close_button_closes_and_one_on_the_tab_does_not() {
    let (mut fixture, seen) = rig(TabKind::Document);
    let tab = fixture.by_label("Doc").expect("the tab is named");
    fixture.click(&tab);
    assert_eq!(seen.get(), None);
    let close = fixture.by_label("Close Doc").expect("the close button is named");
    fixture.click(&close);
    assert_eq!(seen.get(), Some(TabPartEvent::Close));
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
