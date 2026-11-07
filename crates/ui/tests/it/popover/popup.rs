//! Popups: opening, staying open, and the dismissal rules.

use crate::overlay::kit::{Leaf, Opened, Opener, Seen};
use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, CollectionEvent, Overlay, Props as _, Show as _, ShowCollection as _, ShowContainer as _, Trigger};
use craft_ui::list::{List, ListRow};
use craft_ui::popover::PopupList;
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{click, moved, press, release};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

const OPENER: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(180.0, 120.0) };
const BELOW: Rect = Rect { min: Pos2::new(300.0, 300.0), max: Pos2::new(360.0, 330.0) };

/// An opener of the preset `make` and a leaf below it at a fixed place; returns whether the popup body ran.
fn scene(frames: &mut Frames, events: Vec<egui::Event>, trigger: Trigger, make: fn() -> Overlay, open: bool, below: &RefCell<Seen>) -> bool {
    let opened = Opened::default();
    let content = RefCell::new(Seen::default());
    frames.frame(events, |ui: &mut Ui| {
        Opener::new(&opened, &content, trigger, make).open(open).id_salt("opener").show_at(ui, OPENER);
        Leaf::new(below, Vec2::new(60.0, 30.0)).id_salt("below").show_at(ui, BELOW);
    });
    opened.shown.get()
}

#[trial]
fn manual_popup_opens_only_through_open() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events, open| scene(frames, events, Trigger::Manual, Overlay::dropdown, open, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![], false);
    }
    let at = OPENER.center();
    frame(&mut frames, vec![moved(at)], false);
    frame(&mut frames, vec![press(at)], false);
    frame(&mut frames, vec![release(at)], false);
    let alt_down = key_press(Key::ArrowDown, Modifiers::ALT);
    let after_keys = [frame(&mut frames, vec![alt_down], false), frame(&mut frames, vec![], false)];
    assert_eq!(after_keys, [false, false], "presses and Intent::Open never open a manual popup");
    frame(&mut frames, vec![], true);
    assert!(frame(&mut frames, vec![], false), "the popup is open after `open`");
    assert!(frame(&mut frames, vec![], false), "and stays open");
}

#[trial]
fn a_popup_stays_open_over_idle_frames() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    frames.frame(vec![], |_| {});
    let opened = Opened::default();
    let content = RefCell::new(Seen::default());
    let mut show = |ui: &mut Ui| {
        opened.shown.set(false);
        Opener::new(&opened, &content, Trigger::Click, Overlay::dropdown).id_salt("opener").show_at(ui, OPENER);
        Leaf::new(&below, Vec2::new(60.0, 30.0)).id_salt("below").show_at(ui, BELOW);
    };
    click(&mut frames, OPENER.center(), &mut show);
    frames.idle(2, &mut show);
    assert!(opened.shown.get(), "a click on the opener opens its popup");
    for _ in 0..120 {
        frames.frame(vec![], &mut show);
        assert!(opened.shown.get(), "the popup stays open over two seconds of idle frames");
    }
}

/// Opens the popup, then presses outside it on the leaf below; returns whether the popup still shows after the press
/// and what the leaf below saw.
fn press_outside(make: fn() -> Overlay) -> (bool, u32) {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events, open| scene(frames, events, Trigger::Manual, make, open, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![], false);
    }
    frame(&mut frames, vec![], true);
    for _ in 0..30 {
        frame(&mut frames, vec![], false);
    }
    assert!(frame(&mut frames, vec![], false), "the popup is open before the press");
    let at = BELOW.center();
    frame(&mut frames, vec![moved(at)], false);
    frame(&mut frames, vec![press(at)], false);
    frame(&mut frames, vec![release(at)], false);
    let still = [frame(&mut frames, vec![], false), frame(&mut frames, vec![], false)];
    let presses = below.borrow().presses;
    (still[1], presses)
}

#[trial]
fn an_outside_press_closes_a_dropdown_and_reaches_the_widget_below() {
    let (open, presses) = press_outside(Overlay::dropdown);
    assert!(!open, "an outside press closes the popup");
    assert!(presses > 0, "the press reaches the widget below");
}

#[trial]
fn an_outside_press_closes_a_menu_and_does_not_reach_the_widget_below() {
    let (open, presses) = press_outside(Overlay::menu);
    assert!(!open, "an outside press closes the menu");
    assert_eq!(presses, 0, "the menu absorbs the press");
}

#[trial]
fn a_press_inside_the_popup_keeps_it_open() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events, open| scene(frames, events, Trigger::Manual, Overlay::dropdown, open, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![], false);
    }
    frame(&mut frames, vec![], true);
    for _ in 0..30 {
        frame(&mut frames, vec![], false);
    }
    // The dropdown hangs below its anchor, as wide as the anchor.
    let inside = rect(OPENER.left(), OPENER.bottom(), OPENER.width(), 30.0).center();
    frame(&mut frames, vec![moved(inside)], false);
    frame(&mut frames, vec![press(inside)], false);
    frame(&mut frames, vec![release(inside)], false);
    assert!(frame(&mut frames, vec![], false), "a press inside the popup does not close it");
    assert!(frame(&mut frames, vec![], false));
}

#[trial]
fn a_click_on_the_opener_closes_its_open_menu() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let opened = Opened::default();
    let content = RefCell::new(Seen::default());
    let mut show = |ui: &mut Ui| {
        opened.shown.set(false);
        Opener::new(&opened, &content, Trigger::Click, Overlay::menu).id_salt("opener").show_at(ui, OPENER);
        Leaf::new(&below, Vec2::new(60.0, 30.0)).id_salt("below").show_at(ui, BELOW);
    };
    frames.idle(3, &mut show);
    click(&mut frames, OPENER.center(), &mut show);
    frames.idle(30, &mut show);
    assert!(opened.shown.get(), "a click opens the menu");
    click(&mut frames, OPENER.center(), &mut show);
    frames.idle(30, &mut show);
    assert!(!opened.shown.get(), "the press on the opener reaches it, and its toggle closes the menu");
}

#[trial]
fn a_press_trigger_opens_on_the_press() {
    let mut frames = Frames::new();
    let below = RefCell::new(Seen::default());
    let frame = |frames: &mut Frames, events| scene(frames, events, Trigger::Press, Overlay::popover, false, &below);
    for _ in 0..3 {
        frame(&mut frames, vec![]);
    }
    let at = OPENER.center();
    frame(&mut frames, vec![moved(at)]);
    frame(&mut frames, vec![press(at)]);
    let shown = [frame(&mut frames, vec![]), frame(&mut frames, vec![])];
    assert!(shown[1], "the popup opens while the button is still down");
}

#[trial]
fn a_hover_popup_closes_after_a_grace_period_with_the_pointer_on_neither() {
    let mut frames = Frames::new();
    let (opened, content) = (Opened::default(), RefCell::new(Seen::default()));
    let mut show = |ui: &mut Ui| {
        opened.shown.set(false);
        Opener::new(&opened, &content, Trigger::Hover, Overlay::popover).id_salt("opener").show_at(ui, OPENER);
    };
    frames.idle(3, &mut show);
    frames.frame(vec![moved(OPENER.center())], &mut show);
    frames.advance(1.0);
    frames.idle(3, &mut show);
    assert!(opened.shown.get(), "hovering the opener opens the popup after the dwell");
    frames.frame(vec![moved(Pos2::new(700.0, 500.0))], &mut show);
    frames.idle(2, &mut show);
    assert!(opened.shown.get(), "the pointer on neither the opener nor the popup does not close it at once");
    frames.advance(1.0);
    frames.idle(3, &mut show);
    assert!(!opened.shown.get(), "it closes once the grace period passed");
}

/// Keyboard focus on a hover opener counts as resting on it: the popup opens after the dwell, and closes after the
/// grace once the focus has moved on.
#[trial]
fn a_hover_popup_opens_when_its_opener_gets_keyboard_focus() {
    use craft_ui::contract::tooltip::{DELAY, GRACE};
    let mut frames = Frames::new();
    let (hover, other) = (Opened::default(), Opened::default());
    let (content, unused) = (RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    let mut show = |ui: &mut Ui| {
        hover.shown.set(false);
        Opener::new(&hover, &content, Trigger::Hover, Overlay::popover).id_salt("opener").show_at(ui, OPENER);
        Opener::new(&other, &unused, Trigger::Manual, Overlay::popover).id_salt("next").show_at(ui, BELOW);
    };
    frames.idle(3, &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.advance(DELAY);
    frames.idle(3, &mut show);
    assert!(hover.shown.get(), "keyboard focus on the opener opens its hover popup after the dwell");
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    frames.advance(GRACE);
    frames.idle(3, &mut show);
    assert!(!hover.shown.get(), "the popup closes after the grace once focus left the opener");
}

const TIPS: [&str; 4] = ["Round", "Square", "Chisel", "Fan"];

/// The tip names as a collection source, keyed by position.
struct Tips;

impl DisplayOrder<usize> for Tips {
    fn len(&self) -> usize {
        TIPS.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < TIPS.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        TIPS.get(at).map(|name| std::borrow::Cow::Borrowed(*name))
    }
}

impl ItemSource<usize> for Tips {}

/// A button with a `PopupList` of the tips under it, opened while the button has the key focus.
struct Picker {
    fixture: Fixture,
    closed: Rc<Cell<Option<CloseReason>>>,
    activated: Rc<Cell<Option<usize>>>,
    opener: Rc<Cell<Option<Id>>>,
}

fn open_picker() -> Picker {
    let (open, closed, activated, opener_id) =
        (Rc::new(Cell::new(false)), Rc::new(Cell::new(None)), Rc::new(Cell::new(None)), Rc::new(Cell::new(None)));
    let (shown_open, shown_closed, shown_activated, shown_opener) = (open.clone(), closed.clone(), activated.clone(), opener_id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush tip").show(ui);
        shown_opener.set(Some(opener.id));
        let mut is_open = shown_open.get();
        let shown = PopupList::new("Brush tips", Bind::bound(&mut is_open)).opened_by(&opener).show(ui, |ui, _| {
            let events = List::<usize>::options("Brush tips").show_items(ui, &Tips, &mut |at, _| ListRow::new(TIPS[*at]).option()).events;
            if let Some(at) = events.into_iter().find_map(|event| match event {
                CollectionEvent::Activate(at) => Some(at),
                _ => None,
            }) {
                shown_activated.set(Some(at));
            }
        });
        shown_open.set(is_open);
        if shown.output.closed.is_some() {
            shown_closed.set(shown.output.closed);
        }
    });
    fixture.settle();
    let button = fixture.by_label("Brush tip").expect("the opener is a node");
    fixture.click(&button);
    open.set(true);
    fixture.idle(4);
    Picker { fixture, closed, activated, opener: opener_id }
}

/// Down moves the lead and End reaches the last row; Enter activates the lead row, closes the popup with `Activated`
/// and leaves the key focus on the opener, which the host never moved.
#[trial]
fn a_popup_list_takes_list_keys_from_its_opener() {
    let mut picker = open_picker();
    picker.fixture.key(Key::ArrowDown, Modifiers::NONE);
    picker.fixture.idle(2);
    picker.fixture.key(Key::Enter, Modifiers::NONE);
    picker.fixture.idle(4);
    assert_eq!(picker.activated.get(), Some(1), "Down moved the lead from the first row to the second, and Enter activated it");
    assert_eq!(picker.closed.get(), Some(CloseReason::Activated), "activating a row closes the popup");
    assert_eq!(picker.fixture.focused(), picker.opener.get(), "the key focus stayed on the opener");

    let mut picker = open_picker();
    picker.fixture.key(Key::End, Modifiers::NONE);
    picker.fixture.idle(2);
    picker.fixture.key(Key::Enter, Modifiers::NONE);
    picker.fixture.idle(4);
    assert_eq!(picker.activated.get(), Some(TIPS.len() - 1), "End moved the lead to the last row");
}

/// A letter typed on an opener that is not a text field moves the lead to the next row that starts with it.
#[trial]
fn a_letter_on_a_popup_list_opener_moves_the_lead_by_type_ahead() {
    let mut picker = open_picker();
    picker.fixture.type_text("c");
    picker.fixture.idle(2);
    picker.fixture.key(Key::Enter, Modifiers::NONE);
    picker.fixture.idle(4);
    assert_eq!(picker.activated.get(), Some(2), "`c` moved the lead to Chisel");
}
