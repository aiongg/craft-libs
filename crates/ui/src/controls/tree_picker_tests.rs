//! `TreePicker` search and paths.

use super::*;

fn entry(key: u32, name: &str, parent: Option<u32>, folder: bool) -> TreeEntry<u32> {
    TreeEntry { key, name: name.to_string(), parent, folder, blocked: None, icon: None }
}

/// Library / 2025 / Trip, Library / Family, Archive / Old Trip, Trip Notes.
fn albums() -> Vec<TreeEntry<u32>> {
    vec![
        entry(1, "Library", None, true),
        entry(2, "2025", Some(1), true),
        entry(3, "Trip", Some(2), false),
        entry(4, "Family", Some(1), false),
        entry(5, "Archive", None, true),
        entry(6, "Old Trip", Some(5), false),
        entry(7, "Trip Notes", None, false),
    ]
}

// `tree_search`: every word must be in the path; names starting with the query come first; folders never match.
#[test]
fn search_matches_every_word_and_puts_name_starts_first() {
    let entries = albums();
    assert_eq!(tree_search(&entries, "trip"), [2, 6, 5], "Trip and Trip Notes start with it, Old Trip only holds it");
    assert_eq!(tree_search(&entries, "TRIP lib"), [2], "every word, in any case, anywhere in the path");
    assert_eq!(tree_search(&entries, "library"), [2, 3], "an item matches by its folders; the folder itself never does");
    assert_eq!(tree_search(&entries, "2025 family"), Vec::<usize>::new());
    assert!(tree_search(&entries, "  ").is_empty());
}

// `tree_path` gives "Folder / Sub / Name" and empty for an unknown key.
#[test]
fn the_path_joins_the_folders() {
    let entries = albums();
    assert_eq!(tree_path(&entries, &3, " / "), "Library / 2025 / Trip");
    assert_eq!(tree_path(&entries, &7, " › "), "Trip Notes");
    assert_eq!(tree_path(&entries, &99, " / "), "");
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod picker {
    use egui::accesskit::Role;
    use egui::{Key, Ui};
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use egui_kittest::{Harness, HarnessBuilder};

    use super::super::{TreeEntry, TreePicker};
    use super::*;
    use crate::edit::EditEvent;
    use crate::theme::{Base, install};

    struct Host {
        entries: Vec<TreeEntry<u32>>,
        value: Option<u32>,
        events: Vec<EditEvent<Option<u32>>>,
    }

    fn harness() -> Harness<'static, Host> {
        HarnessBuilder::default().with_size([360.0, 480.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                let out = TreePicker::new("album", "Album", &mut host.value, &host.entries).show(ui);
                host.events.extend(out.event);
            },
            Host { entries: albums(), value: Some(3), events: Vec::new() },
        )
    }

    fn items(harness: &Harness<'_, Host>) -> Vec<String> {
        harness.query_all_by_role(Role::TreeItem).filter_map(|n| n.accesskit_node().label()).collect()
    }

    // In a harness: opening the popup opens the folders holding the value and selects it; Enter on another item picks it and closes the popup.
    #[test]
    fn opening_shows_the_chosen_item_with_its_folders_open() {
        let mut harness = harness();
        harness.run();
        assert_eq!(harness.get_by_role(Role::ComboBox).accesskit_node().value().as_deref(), Some("Library / 2025 / Trip"));
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.run();
        assert_eq!(items(&harness), ["Library", "2025", "Trip", "Family", "Archive", "Trip Notes"], "Archive stays closed");
        assert!(harness.get_by_label("Trip").accesskit_node().is_selected().unwrap_or(false), "the chosen item is selected");
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        harness.run();
        assert_eq!(harness.state().value, Some(4));
        assert!(matches!(harness.state().events.as_slice(), [EditEvent::Set { value: Some(4), .. }]), "a pick is a Set");
        assert!(items(&harness).is_empty(), "the popup closed");
        assert_eq!(harness.get_by_role(Role::ComboBox).accesskit_node().value().as_deref(), Some("Library / Family"));
    }

    // A folder opens and closes with → and ←, and Enter on it does not pick; a blocked item does not pick either.
    #[test]
    fn folders_open_with_the_arrows_and_blocked_items_do_not_pick() {
        let mut harness = harness();
        harness.state_mut().value = None;
        harness.state_mut().entries[5].blocked = Some("Read-only".to_string());
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.run();
        assert_eq!(items(&harness), ["Library", "Archive", "Trip Notes"].map(String::from), "everything closed with no value");
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert_eq!(items(&harness), ["Library", "Archive", "Old Trip", "Trip Notes"].map(String::from));
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().value, None, "a blocked item is not picked");
        harness.key_press(Key::ArrowUp);
        harness.key_press(Key::ArrowLeft);
        harness.run();
        assert_eq!(items(&harness), ["Library", "Archive", "Trip Notes"].map(String::from));
        assert!(harness.state().events.is_empty());
    }

    // Typing lists the matching items with their paths; Enter picks the first match.
    #[test]
    fn typing_searches_and_enter_picks_the_first_match() {
        let mut harness = harness();
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.run();
        harness.event(egui::Event::Text("trip".to_string()));
        harness.run();
        harness.run();
        let rows: Vec<String> = harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect();
        assert_eq!(
            rows,
            ["Trip, Library / 2025 / Trip", "Trip Notes, Trip Notes", "Old Trip, Archive / Old Trip"],
            "each with its path as the detail"
        );
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::Enter);
        harness.run();
        harness.run();
        assert_eq!(harness.state().value, Some(7));
    }

    // A pointer click on a row of the open popup picks it: the popup's layer must not swallow its own presses.
    #[test]
    fn clicking_a_row_with_the_pointer_picks_it() {
        let mut harness = harness();
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.run();
        let at = harness.get_by_label("Family").rect().center();
        harness.hover_at(at);
        harness.run();
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            harness.run();
        }
        harness.run();
        assert_eq!(harness.state().value, Some(4));
        assert!(items(&harness).is_empty(), "the popup closed");
    }
}
