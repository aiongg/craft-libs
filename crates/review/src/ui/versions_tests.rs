//! Version history.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::versions::{VersionAction, VersionHistoryPanel};
use crate::versions::{Version, VersionId};

struct App {
    versions: Vec<Version>,
    actions: Vec<VersionAction>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = VersionHistoryPanel::new("versions", &app.versions, &[]).show(ui);
    app.actions.extend(out.action);
}

fn harness() -> Harness<'static, App> {
    let version = |id: u64, title: &str, autosave: bool| Version {
        id: VersionId(id),
        title: title.into(),
        description: String::new(),
        created: i64::try_from(id).expect("small") * 1_000,
        author: "Ana".into(),
        autosave,
    };
    let versions = vec![version(1, "Draft", false), version(2, "", true), version(3, "", true), version(4, "Final", false)];
    let mut harness = Harness::<App>::builder()
        .with_size([420.0, 640.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, App { versions, actions: Vec::new() });
    harness.run();
    harness
}

fn enter() -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::NONE, Key::Enter)
}

// Save version, typing and Enter report Save.
#[test]
fn save_form_reports_title_and_description() {
    let mut harness = harness();
    harness.get_by_role_and_label(Role::Button, "Save version").click();
    harness.run();
    harness.get_by_label("Title").click();
    harness.run();
    harness.type_text("Before review");
    harness.press(enter());
    assert_eq!(harness.state().actions, [VersionAction::Save { title: "Before review".into(), description: String::new() }]);
}

// the row menu reports each action; Delete confirms first.
#[test]
fn row_menu_restores_renames_duplicates_and_deletes() {
    let mut harness = harness();
    let choose = |harness: &mut Harness<'static, App>, item: &str| {
        harness.get_all_by_role_and_label(Role::Button, "\u{2026}").next().expect("the newest row's menu").click();
        harness.run();
        harness.get_by_label(item).click();
        harness.run();
    };
    choose(&mut harness, "Restore this version");
    choose(&mut harness, "Duplicate");
    assert_eq!(harness.state().actions, [VersionAction::Restore(VersionId(4)), VersionAction::Duplicate(VersionId(4))]);

    choose(&mut harness, "Delete");
    assert_eq!(harness.state().actions.len(), 2, "Delete waits for its confirmation");
    harness.get_by_label("Delete this version? Final");
    harness.get_by_role_and_label(Role::Button, "Delete").click();
    harness.run();
    assert_eq!(harness.state().actions.last(), Some(&VersionAction::Delete(VersionId(4))));

    choose(&mut harness, "Rename");
    harness.type_text("Release");
    harness.press(enter());
    assert_eq!(harness.state().actions.last(), Some(&VersionAction::Rename { id: VersionId(4), title: "Release".into() }));
}
