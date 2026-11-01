//! The signature pad.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::testing::symbols;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::signature::SignaturePadDialog;
use crate::signature::{SavedSignature, SignatureDraft, SignatureTab};

fn harness(draft: SignatureDraft) -> Harness<'static, DialogHost> {
    let mut host = DialogHost::new();
    host.open(SignaturePadDialog::new(draft));
    let mut harness = Harness::<DialogHost>::builder()
        .with_size([640.0, 480.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(|ui, host: &mut DialogHost| host.show(ui.ctx()), host);
    harness.run();
    harness
}

// Apply returns the drawn strokes on the Draw tab.
#[test]
fn apply_returns_the_current_tab() {
    let strokes = vec![vec![[0.1, 0.5], [0.4, 0.2], [0.9, 0.6]]];
    let draft = SignatureDraft { tab: SignatureTab::Draw, strokes: strokes.clone(), ..SignatureDraft::new(false, "Ana") };
    let mut harness = harness(draft);
    harness.get_by_role_and_label(Role::Button, "Apply").click();
    harness.run();
    let answer = harness.state_mut().take::<SignaturePadDialog>();
    assert!(matches!(answer, Some(DialogResult::Ok((SavedSignature::Drawn(drawn), false))) if drawn == strokes));
}

// Apply is disabled for an empty draft.
#[test]
fn apply_waits_for_content() {
    let mut harness = harness(SignatureDraft::new(false, "  "));
    assert!(harness.get_by_role_and_label(Role::Button, "Apply").accesskit_node().is_disabled());
    harness.get_by_role_and_label(Role::Button, "Clear").click();
    harness.run();
    assert!(harness.state_mut().take::<SignaturePadDialog>().is_none());

    let harness = self::harness(SignatureDraft::new(false, "Ana"));
    assert!(!harness.get_by_role_and_label(Role::Button, "Apply").accesskit_node().is_disabled());
}
