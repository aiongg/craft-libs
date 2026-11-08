//! The browser's action rows are rows of its collection in both views: the lead reaches them from the presets, Enter
//! and a click report them, and no drag lands after them.

use craft_behavior::EditEvent;
use craft_ui::contract::{Bind, ListDrag};
use craft_ui::patterns::presets::{PresetBrowser, PresetEvent, PresetReorder, PresetRow, PresetState, PresetView};
use craft_ui::runtime::drag;
use craft_ui::runtime::env::Label;
use craft_ui::testing::access::focused_key;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const AMBER: u32 = 1;
const CORAL: u32 = 2;
const CREATE: &str = "Create preset";
const DETACH: &str = "Detach";
const VIEWS: [PresetView; 2] = [PresetView::Grid, PresetView::List];

/// What the browser reported over every pass, and the drag it reported last.
#[derive(Default)]
struct Seen {
    events: Vec<PresetEvent<u32>>,
    drag: Option<ListDrag<u32>>,
}

impl Seen {
    fn actions(&self) -> Vec<usize> {
        self.events.iter().filter_map(|event| if let PresetEvent::Action(at) = event { Some(*at) } else { None }).collect()
    }

    fn reordered(&self) -> bool {
        self.events.iter().any(|event| matches!(event, PresetEvent::Reorder(EditEvent::Commit { .. } | EditEvent::Set { .. })))
    }
}

/// A browser of two presets in `view`, with the two action rows and presets that reorder.
fn browser(view: PresetView) -> (Fixture, Rc<RefCell<Seen>>) {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let record = seen.clone();
    let state = Rc::new(RefCell::new(PresetState::<u32>::new()));
    let mut fixture = Fixture::new(move |ui| {
        let rows = vec![PresetRow::item(AMBER, "Amber", 0), PresetRow::item(CORAL, "Coral", 0)];
        let mut held = state.borrow_mut();
        let out = PresetBrowser::new("Swatches", Bind::bound(&mut *held), &rows)
            .view_lock(view)
            .reorder(PresetReorder::items())
            .actions([Label::from(CREATE), Label::from(DETACH)])
            .show(ui);
        let mut seen = record.borrow_mut();
        seen.events.extend(out.events);
        seen.drag = out.drag;
    });
    fixture.settle();
    (fixture, seen)
}

fn row(fixture: &Fixture, label: &str, view: PresetView) -> Target {
    fixture.by_label(label).unwrap_or_else(|| panic!("the {label} row in the {view:?} view"))
}

fn press(fixture: &mut Fixture, key: Key) {
    fixture.key(key, Modifiers::NONE);
    fixture.idle(2);
}

/// Down from the last preset moves the lead to Create, then to Detach, with no stop on the divider between them and
/// the presets; Up comes back the same way. Moving the lead onto an action reports nothing.
#[trial]
fn down_from_the_last_preset_reaches_the_actions_and_up_returns() {
    for view in VIEWS {
        let (mut fixture, seen) = browser(view);
        let coral = row(&fixture, "Coral", view);
        fixture.click(&coral);
        fixture.idle(2);
        let lead = |fixture: &Fixture| focused_key(fixture.tree());
        assert_eq!(lead(&fixture), coral.node, "the click puts the lead on Coral in the {view:?} view");
        let (create, detach) = (row(&fixture, CREATE, view).node, row(&fixture, DETACH, view).node);
        for (key, wanted, step) in [
            (Key::ArrowDown, create, "Down from the last preset"),
            (Key::ArrowDown, detach, "Down from Create"),
            (Key::ArrowUp, create, "Up from Detach"),
            (Key::ArrowUp, coral.node, "Up from Create"),
        ] {
            press(&mut fixture, key);
            assert_eq!(lead(&fixture), wanted, "{step} in the {view:?} view");
        }
        assert!(seen.borrow().actions().is_empty(), "moving the lead chooses no action in the {view:?} view");
    }
}

/// Enter on Create reports the first action, and a click on Detach the second, once each.
#[trial]
fn enter_on_create_and_a_click_on_detach_report_their_actions() {
    for view in VIEWS {
        let (mut fixture, seen) = browser(view);
        let coral = row(&fixture, "Coral", view);
        fixture.click(&coral);
        fixture.idle(2);
        press(&mut fixture, Key::ArrowDown);
        press(&mut fixture, Key::Enter);
        assert_eq!(seen.borrow().actions(), [0], "Enter on Create reports it in the {view:?} view");
        seen.borrow_mut().events.clear();
        let detach = row(&fixture, DETACH, view);
        fixture.click(&detach);
        fixture.idle(2);
        assert_eq!(seen.borrow().actions(), [1], "a click on Detach reports it in the {view:?} view");
        let picked = seen.borrow().events.iter().any(|event| matches!(event, PresetEvent::Select { .. } | PresetEvent::Activate { .. }));
        assert!(!picked, "an action row is no preset in the {view:?} view");
    }
}

/// A preset dragged over the actions carries the app's keys and shows no drop mark, and its release there reorders
/// nothing; dropped after the other preset, it reorders.
#[trial]
fn a_dragged_preset_cannot_drop_after_the_actions() {
    for view in VIEWS {
        let (mut fixture, seen) = browser(view);
        let (amber, coral, detach) = (row(&fixture, "Amber", view), row(&fixture, "Coral", view), row(&fixture, DETACH, view));
        let below = detach.center() + Vec2::new(0.0, detach.rect.height() / 4.0) - amber.center();
        fixture.drag_at_pausing(amber.center(), below, |fixture| {
            let drag = seen.borrow().drag.clone();
            assert_eq!(drag.map(|drag| drag.grabbed), Some(AMBER), "the browser reports the drag with the app's key in the {view:?} view");
            assert!(drag::mark(fixture.ctx()).is_none(), "no drop mark over the actions in the {view:?} view");
        });
        fixture.idle(2);
        assert!(!seen.borrow().reordered(), "a release over the actions reorders nothing in the {view:?} view");

        let after = coral.center() + coral.rect.size() * 0.3 - amber.center();
        fixture.drag(&amber, after);
        fixture.idle(2);
        assert!(seen.borrow().reordered(), "a drop after Coral reorders in the {view:?} view");
    }
}
