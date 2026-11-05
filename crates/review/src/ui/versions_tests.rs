//! Version history.

use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::ReviewStyle;
use super::versions::{Columns, VersionAction, VersionHistoryPanel, VersionPreviewBanner};
use crate::versions::{Branch, BranchId, BranchState, Version, VersionId};
use craft_ui::list::ListRow;
use craft_ui::theme::TokensOf as _;

struct App {
    versions: Vec<Version>,
    branches: Vec<Branch>,
    actions: Vec<VersionAction>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = VersionHistoryPanel::new("versions", &app.versions, &app.branches).show(ui);
    app.actions.extend(out.actions);
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
    let branches = vec![Branch { id: BranchId(1), name: "Dark mode".into(), state: BranchState::Merged, created: 500, author: "Bo".into() }];
    let versions = vec![version(1, "Draft", false), version(2, "", true), version(3, "", true), version(4, "Final", false)];
    let mut harness = Harness::<App>::builder()
        .with_size([420.0, 640.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, App { versions, branches, actions: Vec::new() });
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
        harness.get_by_role_and_label(Role::Button, "More (Final)").click();
        harness.run();
        harness.get_by_label(item).click();
        harness.run();
    };
    choose(&mut harness, "Restore this version");
    choose(&mut harness, "Duplicate");
    assert_eq!(harness.state().actions, [VersionAction::Restore(VersionId(4)), VersionAction::Duplicate(VersionId(4))]);

    choose(&mut harness, "Delete");
    assert_eq!(harness.state().actions.len(), 2, "Delete waits for its confirmation");
    harness.get_by_label("Delete \u{201c}Final\u{201d}? This can't be undone.");
    harness.get_by_role_and_label(Role::Button, "Delete").click();
    harness.run();
    assert_eq!(harness.state().actions.last(), Some(&VersionAction::Delete(VersionId(4))));

    choose(&mut harness, "Rename");
    harness.type_text("Release");
    harness.press(enter());
    assert_eq!(harness.state().actions.last(), Some(&VersionAction::Rename { id: VersionId(4), title: "Release".into() }));
}

// Rename in the row's menu opens the field in the pass of the click (not on the next one).
#[test]
fn rename_from_the_row_menu_opens_the_field_in_the_same_pass() {
    let mut harness = harness();
    harness.get_by_role_and_label(Role::Button, "More (Final)").click();
    harness.run();
    harness.get_by_label("Rename").click();
    harness.step();
    harness.get_by_role(Role::TextInput);
}

// Cancel drops the delete strip without an action.
#[test]
fn cancelling_the_delete_strip_reports_nothing() {
    let mut harness = harness();
    harness.get_by_role_and_label(Role::Button, "More (Final)").click();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Cancel").click();
    harness.run();
    assert!(harness.query_by_label_contains("This can't be undone.").is_none());
    assert!(harness.state().actions.is_empty());
}

// an autosave run is one row until it is opened.
#[test]
fn an_autosave_group_opens_into_its_versions() {
    let mut harness = harness();
    let rows = |harness: &Harness<'static, App>| harness.get_all_by_label_contains("More (").count();
    assert_eq!(rows(&harness), 3, "Draft, Final and the branch");
    harness.get_by_label("2 autosave versions").click();
    harness.run();
    assert_eq!(rows(&harness), 5, "the two autosaves join them");
    harness.get_by_label("2 autosave versions").click();
    harness.run();
    assert_eq!(rows(&harness), 3);
}

// a row's dot is centred at `review.version_dot_x` and its label starts at `review.version_text_x`.
#[test]
fn the_slot_puts_the_dot_and_the_text_in_their_columns() {
    let mut harness = Harness::builder().with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(|ui| {
        let tokens = craft_ui::theme::Tokens::of(ui);
        let (style, row) = (ReviewStyle::of(&tokens), ListRow::<u8>::style(&tokens));
        let columns = Columns::new(&style, &row);
        let slot_start = row.pad_x + columns.indent(false);
        assert!((slot_start + columns.slot / 2.0 - style.version_dot_x).abs() < 0.01, "dot");
        assert!((slot_start + columns.slot + row.gap - style.version_text_x).abs() < 0.01, "text");
    });
    harness.run();
}

// Create branch opens a named form; Enter reports the trimmed name.
#[test]
fn the_branch_form_creates_a_branch() {
    let mut harness = harness();
    harness.get_by_role_and_label(Role::Button, "Create branch").click();
    harness.run();
    harness.get_by_label("Branch name").click();
    harness.run();
    harness.type_text(" Dark mode 2 ");
    harness.press(enter());
    assert_eq!(harness.state().actions, [VersionAction::CreateBranch { name: "Dark mode 2".into() }]);
}

// a branch shows its state word and its menu offers Open, Review and merge (not Archive once merged).
#[test]
fn a_branch_row_names_its_state_and_has_a_menu() {
    let mut harness = harness();
    harness.get_by_label_contains("Merged \u{b7}");
    harness.get_by_role_and_label(Role::Button, "More (Dark mode)").click();
    harness.run();
    assert!(harness.query_by_label("Archive").is_none(), "a merged branch can't be archived");
    harness.get_by_label("Review and merge").click();
    harness.run();
    assert_eq!(harness.state().actions, [VersionAction::MergeBranch(BranchId(1))]);
}

struct Banner {
    version: Version,
    focus_field: bool,
    text: String,
    actions: Vec<VersionAction>,
}

fn banner(ui: &mut Ui, app: &mut Banner) {
    let canvas = ui.max_rect();
    craft_ui::focus::key_owner::register_canvas(ui.ctx(), ui.scope_id().with("canvas"));
    let field = ui.add(egui::TextEdit::singleline(&mut app.text));
    field.widget_info(|| egui::WidgetInfo::labeled(Role::TextInput, true, "Name"));
    if app.focus_field {
        field.request_focus();
    }
    let out = VersionPreviewBanner::new("banner", &app.version).show(ui, canvas);
    app.actions.extend(out.action);
}

fn banner_harness(focus_field: bool) -> Harness<'static, Banner> {
    let version =
        Version { id: VersionId(9), title: "Final".into(), description: String::new(), created: 1_000, author: "Ana".into(), autosave: false };
    let mut harness = Harness::<Banner>::builder()
        .with_size([420.0, 200.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(banner, Banner { version, focus_field, text: String::new(), actions: Vec::new() });
    harness.run();
    harness
}

// Esc goes back to the current version while the app owns the keys.
#[test]
fn esc_leaves_the_preview() {
    let mut harness = banner_harness(false);
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
    assert_eq!(harness.state().actions, [VersionAction::Current]);
}

// Esc in a text field is the field's.
#[test]
fn esc_in_a_text_field_stays_with_the_field() {
    let mut harness = banner_harness(true);
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
    assert!(harness.state().actions.is_empty(), "{:?}", harness.state().actions);
}
