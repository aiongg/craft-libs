//! `AltTextField` rules and its harness.

use super::*;

// `starts_decorative`: false for none or any undecorated, true when all are.
#[test]
fn decorative_starts_checked_only_when_every_object_is() {
    assert!(!starts_decorative([]));
    assert!(!starts_decorative([true, false]));
    assert!(!starts_decorative([false]));
    assert!(starts_decorative([true]));
    assert!(starts_decorative([true, true, true]));
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod field {
    use egui::Ui;
    use egui::accesskit::Role;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use egui_kittest::{Harness, HarnessBuilder};

    use super::super::{AltText, AltTextField};
    use crate::edit::EditEvent;
    use crate::theme::{Base, install};

    struct Host {
        alt: AltText,
        events: Vec<EditEvent<AltText>>,
    }

    fn harness() -> Harness<'static, Host> {
        HarnessBuilder::default().with_size([420.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                let out = AltTextField::new("alt", &mut host.alt).show(ui);
                host.events.extend(out.event);
            },
            Host { alt: AltText { text: "A red kite".to_string(), decorative: false }, events: Vec::new() },
        )
    }

    // Checking Mark as decorative reports a `Set` with `decorative` and the text kept; the box is disabled.
    #[test]
    fn marking_decorative_disables_the_box_and_keeps_the_text() {
        let mut harness = harness();
        harness.run();
        assert!(!harness.get_by_role(Role::MultilineTextInput).accesskit_node().is_disabled());
        harness.get_by_role(Role::CheckBox).click();
        harness.run();
        harness.run();
        let state = harness.state();
        assert_eq!(state.alt, AltText { text: "A red kite".to_string(), decorative: true });
        assert!(
            matches!(state.events.as_slice(), [EditEvent::Set { value, .. }] if *value == AltText { text: "A red kite".to_string(), decorative: true }),
            "one Set with the whole value: {:?}",
            state.events
        );
        assert!(harness.get_by_role(Role::MultilineTextInput).accesskit_node().is_disabled(), "the box is disabled while decorative");
        harness.get_by_role(Role::CheckBox).click();
        harness.run();
        harness.run();
        assert_eq!(harness.state().alt.text, "A red kite", "unchecking brings the text back");
        assert!(!harness.get_by_role(Role::MultilineTextInput).accesskit_node().is_disabled());
    }
}
