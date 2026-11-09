//! `Bar`: the end region, scroll, status liveness and the bar items that share its roving group.

use craft_behavior::CanvasView;
use craft_behavior::canvas::zoom;
use craft_behavior::kurbo::{Point, Size};
use craft_ui::bars::{Bar, BarItem, BarItems, BarKind, BreadcrumbBar, StatusItem, StatusReadout, StatusValues};
use craft_ui::buttons::{Button, IconButton};
use craft_ui::contract::{Bind, Entry, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::pickers::Select;
use craft_ui::runtime::commands::EditCommands;
use craft_ui::runtime::env::Scale;
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::{Cell, RefCell};
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

/// The interface scale of the status trial.
const STATUS_SCALE: f32 = 2.0;

/// The typed 200 % against the shown 100 %.
const TYPED_FACTOR: f64 = 2.0;

const STATUS_DOCUMENT: Size = Size::new(2000.0, 1500.0);

fn status_view(zoom: f64) -> CanvasView {
    CanvasView::new(Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0)), Point::new(1000.0, 750.0), zoom)
}

/// The zoom field shows the device percent, as the Navigator's zoom row does, and a typed zoom is written into the
/// bound view about its centre and reported.
#[trial]
fn the_status_zoom_is_the_device_percent_and_a_typed_zoom_is_written_into_the_view() {
    let view = Rc::new(RefCell::new(status_view(0.5)));
    let reported = Rc::new(Cell::new(false));
    let (held, seen) = (view.clone(), reported.clone());
    let mut fixture = Fixture::new(move |ui| {
        let values = StatusValues::default();
        let shown = StatusReadout::new(Bind::bound(&mut *held.borrow_mut()), STATUS_DOCUMENT, StatusItem::DocumentSizes, &values).show(ui);
        seen.set(seen.get() || shown.view.is_some());
    });
    fixture.scale(Scale::new(STATUS_SCALE));
    fixture.settle();
    assert!(fixture.texts().iter().any(|run| run.text.contains("100")), "zoom 0.5 at 2 pixels per point is 100 %");
    let centre = view.borrow().center;
    let before = zoom::device(&view.borrow(), STATUS_SCALE);
    tab(&mut fixture);
    fixture.type_text("200");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    let doubled = before * TYPED_FACTOR;
    assert_eq!(zoom::device(&view.borrow(), STATUS_SCALE), doubled, "the typed 200 % doubles the shown 100 %");
    assert_eq!(view.borrow().center, centre, "the zoom keeps the view's centre");
    assert!(reported.get(), "the output carries the view edit");
}

/// A select inside a bar opens its list in a layer of its own: the bar's scrolling body does not reach the list, Down
/// and Enter pick the next row, and a click on a row picks it.
#[trial]
fn a_select_in_a_bar_opens_its_list_and_picks_a_row() {
    let value = Rc::new(Cell::new(0u8));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply"), Entry::new(2, "Screen")];
            let mut mode = shown.get();
            Select::new("Blend mode", Bind::bound(&mut mode), &entries).id_salt("mode").show(ui);
            shown.set(mode);
        });
    });
    fixture.settle();
    tab(&mut fixture);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(2);
    assert!(!fixture.by_role(Role::LIST_BOX).is_empty(), "Alt+Down opens the list");
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(10);
    assert_eq!(value.get(), 1, "Down and Enter pick the next row");
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(2);
    let row = fixture.by_label("Screen").expect("the row is shown");
    fixture.click(&row);
    fixture.idle(10);
    assert_eq!(value.get(), 2, "a click picks the row");
}

/// A bar in a column is as tall as its content: the control after it follows directly instead of being pushed to
/// the bottom of the room.
#[trial]
fn a_bar_leaves_the_room_under_it_to_what_follows() {
    let mut fixture = Fixture::new(|ui| {
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            Button::new("Inside").show(ui);
        });
        Button::new("After").show(ui);
    });
    fixture.settle();
    let (bar, after) = (fixture.by_role(Role::GROUP).into_iter().next().expect("the bar is shown"), fixture.by_label("After").expect("shown"));
    assert!(after.rect.top() >= bar.rect.bottom(), "the control after the bar sits below it");
    assert!(after.rect.top() - bar.rect.bottom() < bar.rect.height(), "nothing but the column's gap separates them");
}

/// A child that fills its width in the bar's horizontally scrolling body takes its content width: the body's room is
/// unbounded, not the largest extent egui can make.
#[trial]
fn a_filling_child_of_the_scrolling_body_measures_its_content() {
    let crumbs: Vec<Entry<u8>> = ["photos", "2026", "trip"].iter().zip(0u8..).map(|(name, at)| Entry::new(at, (*name).to_owned())).collect();
    let mut fixture = Fixture::new(move |ui| {
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            BreadcrumbBar::new("Path", &crumbs).show(ui);
        });
    });
    fixture.room(Vec2::new(900.0, 80.0)).settle();
    let path = fixture.by_label("Path").expect("the breadcrumb has a node");
    let bar = fixture.by_role(Role::GROUP).into_iter().next().expect("the bar is shown");
    let last = fixture.by_label("trip").expect("the last crumb is shown");
    assert!(
        path.rect.width().is_finite() && path.rect.width() < bar.rect.width(),
        "the breadcrumb is bounded by the bar: {:?} in {:?}",
        path.rect,
        bar.rect
    );
    assert!(path.rect.right() >= last.rect.right(), "the breadcrumb is at least as wide as its crumbs: {:?} holds {:?}", path.rect, last.rect);
}
