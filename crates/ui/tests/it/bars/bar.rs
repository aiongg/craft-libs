//! `Bar`: the end region, scroll, status liveness and the bar items that share its roving group.

use craft_ui::bars::{AlignBar, Bar, BarItem, BarItems, BarKind, DistributeButtons, StatusItem, StatusReadout, StatusValues};
use craft_ui::buttons::{Button, IconButton};
use craft_ui::contract::{Bind, Politeness, Props as _, Show as _, ShowContainer as _};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers, Vec2};
use std::cell::Cell;
use std::rc::Rc;

fn options_bar() -> Fixture {
    let mut fixture = Fixture::new(|ui| {
        Bar::new("Options", BarKind::Options).commit_cancel("Commit", "Cancel").show(ui, |ui, _| {
            IconButton::new(craft_behavior::IconId::new("check")).label("First").id_salt("first").show(ui);
            IconButton::new(craft_behavior::IconId::new("check")).label("Last").id_salt("last").show(ui);
        });
    });
    fixture.settle();
    fixture
}

fn tab(fixture: &mut Fixture) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
}

fn focused_is(fixture: &Fixture, label: &str) -> bool {
    let target = fixture.by_label(label).expect("node shown");
    fixture.focused().is_some_and(|id| NodeKey::of(id) == target.node)
}

/// Tab from the last control of an options bar reaches Commit, and Commit and Cancel are the bar's last controls
/// at its end edge.
#[trial]
fn tab_from_the_last_control_reaches_commit_at_the_end_edge() {
    let mut fixture = options_bar();
    tab(&mut fixture);
    assert!(focused_is(&fixture, "First"), "the bar is one Tab stop and arrives at its first member");
    for _ in 0..2 {
        fixture.key(Key::ArrowRight, Modifiers::NONE);
    }
    fixture.idle(2);
    assert!(focused_is(&fixture, "Cancel"));
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    assert!(focused_is(&fixture, "Commit"), "Commit is the last member of the bar's group");
    let (commit, last) = (fixture.by_label("Commit").unwrap(), fixture.by_label("Last").unwrap());
    let cancel = fixture.by_label("Cancel").unwrap();
    assert!(last.rect.right() <= cancel.rect.left() && cancel.rect.right() <= commit.rect.left(), "cancel then commit follow the members");
    let bar = fixture.by_role(craft_ui::contract::Role::GROUP);
    let edge = bar.iter().map(|bar| bar.rect.right()).fold(f32::MIN, f32::max);
    assert!((edge - commit.rect.right()).abs() < commit.rect.width(), "Commit sits at the bar's end edge");
}

/// A bar narrower than its members scrolls to the focused one.
#[trial]
fn a_narrow_bar_scrolls_to_reveal_the_focused_control() {
    let mut fixture = options_bar();
    fixture.room(Vec2::new(60.0, 200.0));
    fixture.settle();
    tab(&mut fixture);
    for _ in 0..3 {
        fixture.key(Key::ArrowRight, Modifiers::NONE);
    }
    fixture.idle(3);
    let focused = fixture.by_label("Last").expect("shown");
    let shown = fixture.shown();
    assert!(shown.contains(focused.rect.center()), "the focused control is inside the bar's visible rect");
}

/// `BarItems` and an `IconButton` in one bar share one Tab stop.
#[trial]
fn bar_items_and_a_button_are_one_tab_stop() {
    let mut fixture = Fixture::new(|ui| {
        Bar::new("Tools", BarKind::Tool).show(ui, |ui, _| {
            BarItems::new(vec![BarItem::new(EditCommands::Undo), BarItem::new(EditCommands::Redo)]).show(ui);
            IconButton::new(craft_behavior::IconId::new("check")).label("Extra").show(ui);
        });
        Button::new("After").show(ui);
    });
    fixture.settle();
    tab(&mut fixture);
    let first = fixture.focused();
    tab(&mut fixture);
    assert_ne!(fixture.focused(), first, "the second Tab leaves the bar");
    assert!(focused_is(&fixture, "After"), "the second Tab reaches the control after the bar");
    assert!(!focused_is(&fixture, "Extra") && !focused_is(&fixture, "Redo"), "no member past the first is a Tab stop");
}

/// The status bar is a polite live region; the zoom readout in it is not.
#[trial]
fn the_status_bar_is_live_and_the_zoom_readout_is_not() {
    let (bar, zoom) = (Rc::new(Cell::<Option<Id>>::new(None)), Rc::new(Cell::<Option<Id>>::new(None)));
    let (shown_bar, shown_zoom) = (bar.clone(), zoom.clone());
    let mut fixture = Fixture::new(move |ui| {
        let values = StatusValues::default();
        let mut percent = Some(100.0);
        let shown = Bar::new("Status", BarKind::Status)
            .show(ui, |ui, _| StatusReadout::new(Bind::bound(&mut percent), StatusItem::DocumentSizes, &values).show(ui));
        shown_bar.set(Some(shown.output.response.id));
        shown_zoom.set(shown.inner.and_then(|readout| readout.zoom).map(|zoom| zoom.response.id));
    });
    fixture.settle();
    let tree = fixture.tree();
    assert_eq!(tree.live(bar.get().unwrap()), Some(Politeness::Polite));
    assert_eq!(tree.live(zoom.get().unwrap()), None);
}

fn fired(fixture: &Fixture) -> Vec<String> {
    take_output(fixture.ctx()).fired.iter().map(|fired| fired.command.id.to_string()).collect()
}

/// An align command runs from its button, and from its row in the overflow menu.
#[trial]
fn an_align_command_runs_from_its_button_and_from_the_overflow_menu() {
    let mut fixture = Fixture::new(|ui| {
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            AlignBar::new().distribute(DistributeButtons::Menu).show(ui);
        });
    });
    fixture.settle();
    let button = fixture.by_label("Align Left Edges").expect("the align buttons show");
    fixture.click(&button);
    fixture.idle(2);
    assert!(fired(&fixture).contains(&"align.align_left_edges".to_owned()));

    let more = fixture.by_label("More").expect("the menu part shows");
    fixture.click(&more);
    fixture.idle(2);
    let row = fixture.by_label("Distribute Top Edges").expect("the menu lists the distribute commands");
    fixture.click(&row);
    fixture.idle(2);
    assert!(fired(&fixture).contains(&"align.distribute_top_edges".to_owned()));
}
