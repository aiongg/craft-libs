//! `RecoveryDialog` and its age text.

use std::time::{Duration, SystemTime};

use egui::Key;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::dialog::DialogHost;
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<RecoveryAction>>,
}

fn document(blocked: Option<&str>) -> RecoveredDocument {
    RecoveredDocument {
        key: "k1".into(),
        name: "poster.psd".into(),
        saved: SystemTime::now() - Duration::from_secs(300),
        blocked: blocked.map(str::to_owned),
    }
}

fn harness(document: RecoveredDocument) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(RecoveryDialog::new(document));
    HarnessBuilder::default().with_size(egui::vec2(640.0, 480.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            app.host.show(ui.ctx());
            if let Some(answer) = app.host.take::<RecoveryDialog>() {
                app.answers.push(answer);
            }
        },
        App { host, answers: Vec::new() },
    )
}

// Recover and Discard answer; Esc cancels and keeps the copy; a blocked document disables Recover
// with the reason as its tooltip.
#[test]
fn recover_discard_and_close_answer() {
    let mut harness = harness(document(None));
    harness.run();
    harness.get_by_label_contains("“poster.psd” has unsaved changes from an earlier run, autosaved 5 minutes ago.");
    harness.get_by_label("Recover").click();
    harness.run();
    assert_eq!(harness.state().answers.len(), 1);
    assert!(matches!(harness.state().answers[0], DialogResult::Ok(RecoveryAction::Recover)));

    let mut harness = self::harness(document(None));
    harness.run();
    harness.get_by_label("Discard").click();
    harness.run();
    assert!(matches!(harness.state().answers[0], DialogResult::Ok(RecoveryAction::Discard)));

    let mut harness = self::harness(document(None));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(matches!(harness.state().answers[0], DialogResult::Cancel));

    let mut harness = self::harness(document(Some("Stop recording first")));
    harness.run();
    assert!(harness.get_by_label("Recover").accesskit_node().is_disabled());
    harness.get_by_label("Discard").click();
    harness.run();
    assert!(matches!(harness.state().answers[0], DialogResult::Ok(RecoveryAction::Discard)));
}

// 30 s is "less than a minute ago", 61 s "1 minute ago", 7200 s "2 hours ago".
#[test]
fn relative_age_steps() {
    let ctx = egui::Context::default();
    let labels = RecoveryLabels::default();
    let age = |seconds| relative_age(&ctx, seconds, &labels);
    assert_eq!(age(30), "less than a minute ago");
    assert_eq!(age(61), "1 minute ago");
    assert_eq!(age(7200), "2 hours ago");
    assert_eq!(age(3_600), "1 hour ago");
    assert_eq!(age(86_400 * 2), "2 days ago");
}
