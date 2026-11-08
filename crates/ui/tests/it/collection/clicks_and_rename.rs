//! Which clicks activate a row.

use super::kit::{Dismissed, List, ROW_H, Row, Words};
use craft_ui::contract::{CollectionEvent, SelectMode, ShowCollection as _};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(300.0, 200.0);

/// Two quick primary clicks on the first row, and the `Activate` events they gave.
fn activations_of_two_quick_clicks(list: impl Fn() -> List<'static>) -> usize {
    let words = Words::flat(&["one", "two"]);
    let mut events: Vec<CollectionEvent<usize, Dismissed>> = Vec::new();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            events.extend(list().id_salt("list").show_items(ui, &words, &mut |_, _| Row::new()).events);
        });
    };
    frames.idle(3, &mut show);
    let at = ORIGIN + Vec2::new(50.0, ROW_H / 2.0);
    frames.frame(vec![pointer::moved(at)], &mut show);
    for _ in 0..2 {
        frames.frame(vec![pointer::press(at)], &mut show);
        frames.frame(vec![pointer::release(at)], &mut show);
    }
    frames.idle(2, &mut show);
    drop(show);
    events.iter().filter(|event| matches!(event, CollectionEvent::Activate(0))).count()
}

/// A list that activates on click activates on every primary click; the default activates on the double click only.
#[trial]
fn a_click_list_activates_on_each_quick_click_and_a_double_click_list_once() {
    let on_click = activations_of_two_quick_clicks(|| List::new(SelectMode::Single).activate_on_click());
    assert_eq!(on_click, 2, "each of two quick clicks activates a Click list");
    let on_double = activations_of_two_quick_clicks(|| List::new(SelectMode::Single));
    assert_eq!(on_double, 1, "two quick clicks are one double click in a DoubleClick list");
}
