//! `Autocomplete` rules and its harness.

use super::*;

// `completion_rest("Seattle", "sea")` is "ttle"; a typed text longer than or different from it gives `None`.
#[test]
fn the_rest_of_a_completion_follows_the_typed_text() {
    assert_eq!(completion_rest("Seattle", "sea"), Some("ttle"));
    assert_eq!(completion_rest("Seattle", "SEA"), Some("ttle"));
    assert_eq!(completion_rest("Seattle", ""), Some("Seattle"));
    assert_eq!(completion_rest("Seattle", "Seattle"), None, "nothing is left to complete");
    assert_eq!(completion_rest("Sea", "seattle"), None, "typed is longer");
    assert_eq!(completion_rest("Seattle", "sed"), None, "not a prefix");
    assert_eq!(completion_rest("Straße", "STRA"), Some("ße"), "counted in characters");
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod list {
    use egui::accesskit::Role;
    use egui::{Key, Ui};
    use egui_kittest::kittest::Queryable as _;
    use egui_kittest::{Harness, HarnessBuilder};

    use super::super::{Autocomplete, Suggestion};
    use crate::theme::{Base, install};

    struct Host {
        text: String,
        suggestions: Vec<Suggestion>,
        accepted: Vec<usize>,
        dismissed: u32,
        /// Esc presses left for the editor after the list took its keys.
        escapes: u32,
        tabs: u32,
        downs: u32,
    }

    fn harness() -> Harness<'static, Host> {
        let suggestions = vec![
            Suggestion::new("SUM").mark("ƒ").description("Adds numbers."),
            Suggestion::new("SUMIF").mark("ƒ").description("Adds the cells that meet a condition."),
            Suggestion::new("SUMPRODUCT").mark("ƒ"),
        ];
        HarnessBuilder::default().with_size([360.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                let editor = ui.make_persistent_id("editor");
                let list = Autocomplete::new("formula", editor, &host.suggestions);
                let keys = list.take_keys(ui);
                host.escapes += u32::from(ui.input(|i| i.key_pressed(Key::Escape)));
                host.tabs += u32::from(ui.input(|i| i.key_pressed(Key::Tab)));
                host.downs += u32::from(ui.input(|i| i.key_pressed(Key::ArrowDown)));
                let field = egui::TextEdit::singleline(&mut host.text).id(editor).show(ui);
                let _ = ui.button("Other");
                ui.ctx().accesskit_node_builder(field.response.id, |node| node.set_label("Formula"));
                let out = list.show(ui, field.response.rect, keys);
                host.accepted.extend(out.accepted);
                host.dismissed += u32::from(out.dismissed);
            },
            Host { text: "=SU".to_string(), suggestions, accepted: Vec::new(), dismissed: 0, escapes: 0, tabs: 0, downs: 0 },
        )
    }

    fn focus_editor(harness: &mut Harness<'_, Host>) {
        harness.get_by_role(Role::TextInput).focus();
        harness.run();
        harness.run();
    }

    // With the list showing, ↓ past the last row goes to the first; Tab reports `accepted` with the active row and the editor never sees the Tab.
    #[test]
    fn arrows_wrap_and_tab_accepts() {
        let mut harness = harness();
        focus_editor(&mut harness);
        assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 3, "the list shows");
        for _ in 0..4 {
            harness.key_press(Key::ArrowDown);
            harness.run();
        }
        harness.key_press(Key::Tab);
        harness.run();
        assert_eq!(harness.state().accepted, [1], "three rows: four downs from the first land on the second");
        assert_eq!(harness.state().tabs, 0, "the editor never saw the Tab");
        harness.key_press(Key::ArrowUp);
        harness.run();
        harness.key_press(Key::ArrowUp);
        harness.run();
        harness.key_press(Key::Tab);
        harness.run();
        assert_eq!(harness.state().accepted, [1, 2], "↑ wraps from the first row to the last");
    }

    // Esc reports `dismissed`; the editor keeps its text and its focus; a second Esc reaches the editor.
    #[test]
    fn esc_closes_the_list_and_keeps_the_text() {
        let mut harness = harness();
        focus_editor(&mut harness);
        harness.key_press(Key::Escape);
        harness.run();
        harness.run();
        assert_eq!(harness.state().dismissed, 1);
        assert_eq!(harness.state().escapes, 0, "the list took the Esc");
        assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 0, "the list closed");
        assert_eq!(harness.state().text, "=SU");
        harness.key_press(Key::Escape);
        harness.run();
        assert_eq!(harness.state().escapes, 1, "a second Esc reaches the editor");
        focus_editor(&mut harness);
        harness.state_mut().suggestions.pop();
        harness.run();
        harness.run();
        assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 2, "other suggestions show the list again");
    }

    // With the focus on another widget the list is hidden and takes no key: ↓, Tab and Esc stay with the focused widget.
    #[test]
    fn the_list_needs_the_editors_focus() {
        let mut harness = harness();
        harness.get_by_label("Other").focus();
        harness.run();
        harness.run();
        assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 0, "no list while the editor lacks the focus");
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::Escape);
        harness.run();
        assert_eq!((harness.state().downs, harness.state().escapes), (1, 1), "the keys were not taken");
        assert_eq!(harness.state().dismissed, 0);
        focus_editor(&mut harness);
        assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 3, "the list shows with the editor focused");
    }
}
