use std::borrow::Cow;

use egui::accesskit::Role;
use egui::{Event, Id, Key, Modifiers, Sense, vec2};

use super::{ListOptions, Listbox, RadioSelection, TYPE_AHEAD_PAUSE};
use crate::primitives::Interactive;

fn items(n: usize) -> Vec<(Id, bool)> {
    (0..n).map(|i| (Id::unique(("item", i)), true)).collect()
}

#[test]
fn a_selection_past_the_last_item_selects_none_and_the_first_item_stays_the_tab_stop() {
    egui::__run_test_ui(|ui| {
        let radios = RadioSelection::begin(ui, items(3), Some(3));
        assert_eq!((radios.selected(), radios.changed()), (None, false));
        assert_eq!([radios.sense(0), radios.sense(1)], [Sense::click(), Sense::CLICK]);
        let mut empty = RadioSelection::begin(ui, Vec::new(), Some(0));
        empty.sensed(ui, &[]);
        assert_eq!(empty.selected(), None);
    });
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "one sensed element per item")]
fn sensing_more_elements_than_items_is_caught_in_debug_builds() {
    egui::__run_test_ui(|ui| {
        let mut radios = RadioSelection::begin(ui, items(1), Some(0));
        let sensed = vec![
            Interactive::new(0, Role::RadioButton, "A").interact(ui, vec2(10.0, 10.0)),
            Interactive::new(1, Role::RadioButton, "B").interact(ui, vec2(10.0, 10.0)),
        ];
        radios.sensed(ui, &sensed);
    });
}

/// Five options, the fourth disabled.
struct Fruit;

const FRUIT: [&str; 5] = ["Apple", "Apricot", "Banana", "Avocado", "Blueberry"];

impl ListOptions for Fruit {
    fn len(&self) -> usize {
        FRUIT.len()
    }

    fn option_at(&self, at: usize) -> usize {
        at
    }

    fn enabled(&self, option: usize) -> bool {
        option != 3
    }

    fn text(&self, option: usize) -> Cow<'_, str> {
        FRUIT[option].into()
    }
}

/// Feeds `event` at input time `time` to a listbox over [`Fruit`] that keeps its state in `ctx`,
/// as keys and type-ahead; returns the active option's text.
fn feed(ctx: &egui::Context, event: Event, time: f64) -> Option<&'static str> {
    let mut active = None;
    let input = egui::RawInput { time: Some(time), events: vec![event], ..Default::default() };
    ctx.run_ui(input, |ui| {
        let mut listbox = Listbox::load(ui, Id::unique("fruit"));
        listbox.keys(ui, &Fruit, 2, true);
        listbox.type_ahead(ui, &Fruit);
        active = listbox.active().map(|o| FRUIT[o]);
        listbox.store(ui);
    })
    .textures_delta
    .clear();
    active
}

fn type_at(ctx: &egui::Context, text: &str, time: f64) -> Option<&'static str> {
    feed(ctx, Event::Text(text.to_owned()), time)
}

#[test]
fn type_ahead_matches_a_prefix_cycles_on_a_repeated_letter_skips_disabled_options_and_starts_again_after_a_pause() {
    let ctx = egui::Context::default();
    assert_eq!(type_at(&ctx, "a", 0.0), Some("Apple"));
    assert_eq!(type_at(&ctx, "p", 0.2), Some("Apple"), "\"ap\" still matches the active option");
    assert_eq!(type_at(&ctx, "r", 0.4), Some("Apricot"));
    let later = 0.4 + TYPE_AHEAD_PAUSE + 0.1;
    assert_eq!(type_at(&ctx, "b", later), Some("Banana"), "after a pause the prefix starts again");
    assert_eq!(type_at(&ctx, "b", later + 0.2), Some("Blueberry"), "a repeated letter moves to the next option with it");
    assert_eq!(type_at(&ctx, "b", later + 0.4), Some("Banana"), "and wraps");
    let again = later + 0.4 + TYPE_AHEAD_PAUSE + 0.1;
    assert_eq!(type_at(&ctx, "av", again), Some("Banana"), "\"Avocado\" is disabled: nothing matches, the active option stays");
}

#[test]
fn keys_match_their_modifiers_exactly_and_a_move_starts_the_type_ahead_prefix_again() {
    let ctx = egui::Context::default();
    let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
    let (ctrl, mac_cmd) = (Modifiers::CTRL | Modifiers::COMMAND, Modifiers::MAC_CMD | Modifiers::COMMAND);
    assert_eq!(feed(&ctx, key(Key::ArrowDown, Modifiers::NONE), 0.0), Some("Apple"));
    assert_eq!(feed(&ctx, key(Key::ArrowDown, Modifiers::SHIFT), 0.1), Some("Apple"), "Shift+Down is left to a list that selects ranges");
    assert_eq!(feed(&ctx, key(Key::End, ctrl), 0.2), Some("Blueberry"));
    assert_eq!(feed(&ctx, key(Key::ArrowUp, ctrl), 0.3), Some("Blueberry"), "Ctrl+Up is not Cmd+Up");
    assert_eq!(feed(&ctx, key(Key::ArrowUp, mac_cmd), 0.4), Some("Apple"));
    assert_eq!(feed(&ctx, key(Key::ArrowDown, mac_cmd), 0.5), Some("Blueberry"));
    assert_eq!(feed(&ctx, key(Key::Home, ctrl), 0.6), Some("Apple"));
    assert_eq!(type_at(&ctx, "a", 0.7), Some("Apricot"));
    assert_eq!(feed(&ctx, key(Key::ArrowDown, Modifiers::NONE), 0.8), Some("Banana"));
    assert_eq!(type_at(&ctx, "b", 0.9), Some("Blueberry"), "\"b\" alone, not \"ab\"");
}
