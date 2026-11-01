//! The styles panel in a harness.

use craft_ui::localized::Labels as _;
use craft_ui::theme::{Base, install};
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use craft_ui::composites::StatusLine;

use super::style_panel::*;

struct App {
    kind: TextStyleKind,
    rows: Vec<TextStyleRow>,
    state: TextStylesState,
    current: Option<u64>,
    overridden: bool,
    text_selected: bool,
    actions: Vec<TextStyleAction>,
    statuses: Vec<StatusLine>,
}

fn rows(names: &[&str]) -> Vec<TextStyleRow> {
    names.iter().enumerate().map(|(id, name)| TextStyleRow { id: id as u64, name: (*name).to_owned() }).collect()
}

fn harness(kind: TextStyleKind, names: &[&str], current: u64, overridden: bool, text_selected: bool) -> Harness<'static, App> {
    let app = App {
        kind,
        rows: rows(names),
        state: TextStylesState::default(),
        current: Some(current),
        overridden,
        text_selected,
        actions: Vec::new(),
        statuses: Vec::new(),
    };
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(320.0, 400.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let r = TextStylesPanel::new("styles", app.kind, &app.rows, &mut app.state)
                    .current(app.current, app.overridden)
                    .target(true)
                    .text_selected(app.text_selected)
                    .show(ui);
                app.actions.extend(r.action);
                app.statuses.extend(r.status);
            },
            app,
        )
}

// A click with a target applies, Alt-click applies clearing overrides, double-click asks for
// Style Options; "+" follows an overridden current style.
#[test]
fn rows_apply_clear_overrides_and_open_options() {
    let mut harness = harness(TextStyleKind::Paragraph, &["Basic Paragraph", "Heading", "Body"], 1, true, true);
    harness.run();
    harness.get_by_label("Heading, overridden");

    harness.get_by_label("Body").click();
    harness.run();
    assert_eq!(harness.state().actions, [TextStyleAction::Apply { id: 2, clear_overrides: false }]);
    assert_eq!(harness.state().state.selected, Some(2));

    harness.state_mut().actions.clear();
    harness.get_by_label("Heading, overridden").click_modifiers(Modifiers::ALT);
    harness.run();
    assert_eq!(harness.state().actions, [TextStyleAction::Apply { id: 1, clear_overrides: true }]);

    harness.state_mut().actions.clear();
    harness.get_by_label("Body").click();
    harness.run();
    harness.get_by_label("Body").click();
    harness.run();
    assert_eq!(harness.state().actions.last(), Some(&TextStyleAction::Options(2)));
}

// A character style click at a bare caret applies nothing and sets the "Select text…" status;
// Delete and Redefine do nothing for character style 0.
#[test]
fn character_style_rules_at_a_caret_and_for_none() {
    let labels = TextStylesLabels::english();
    let mut harness = harness(TextStyleKind::Character, &["None", "Emphasis"], 0, false, false);
    harness.run();
    harness.get_by_label("Emphasis").click();
    harness.run();
    assert!(harness.state().actions.is_empty());
    assert_eq!(harness.state().statuses.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), [labels.select_text.as_str()]);

    harness.state_mut().state.selected = Some(0);
    harness.state_mut().text_selected = true;
    harness.get_by_label("None").click();
    harness.run();
    assert_eq!(harness.state().actions, [TextStyleAction::Apply { id: 0, clear_overrides: false }]);
    harness.state_mut().actions.clear();

    harness.key_press(Key::Delete);
    harness.run();
    harness.get_by_label(&labels.redefine).click();
    harness.run();
    assert!(harness.state().actions.is_empty(), "{:?}", harness.state().actions);
    assert!(!harness.get_by_label(&labels.redefine).accesskit_node().is_disabled());
}

// The panel menu has the footer's commands under the footer's conditions, and a command comes back
// as the footer button's action.
#[test]
fn panel_menu_follows_the_footers_conditions() {
    let mut harness = harness(TextStyleKind::Character, &["None", "Emphasis"], 0, false, true);
    harness.run();
    let ctx = harness.ctx.clone();
    let enabled = |state: &TextStylesState| {
        TextStylesPanel::panel_menu(&ctx, "styles", state)
            .iter()
            .filter_map(|e| e.item())
            .map(|i| (i.label().to_owned(), i.is_enabled()))
            .collect::<Vec<_>>()
    };
    let labels = TextStylesLabels::english();
    let rows = enabled(&TextStylesState::default());
    let state_of = |label: &str| rows.iter().find(|(l, _)| l == label).map(|(_, on)| *on);
    // Character style None is current: not overridden, not redefinable, not deletable or editable.
    assert_eq!(state_of(&labels.clear_override), Some(false));
    assert_eq!(state_of(&labels.redefine), Some(false));
    assert_eq!(state_of(&labels.delete), Some(false));
    assert_eq!(state_of(&labels.options), Some(false));
    assert_eq!(state_of(&labels.new_style), Some(true));

    let mut selected = TextStylesState { selected: Some(1) };
    let rows = enabled(&selected);
    assert!(rows.iter().find(|(l, _)| *l == labels.delete).is_some_and(|(_, on)| *on));
    assert_eq!(TextStylesPanel::run_command(&ctx, "styles", &mut selected, "textStyles.options"), Some(TextStyleAction::Options(1)));
    assert_eq!(TextStylesPanel::run_command(&ctx, "styles", &mut selected, "textStyles.delete"), Some(TextStyleAction::Delete(1)));
    assert_eq!(selected.selected, None);
    assert_eq!(TextStylesPanel::run_command(&ctx, "styles", &mut selected, "textStyles.clearOverride"), None);
}

// With a highlight: Shift-click highlights a range and Cmd/Ctrl-click toggles one without
// applying; Delete with three highlighted returns DeleteHighlighted; a highlighted row dropped on
// New returns DuplicateHighlighted, an unhighlighted one Duplicate(id).
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn highlighted_rows_act_together() {
    todo!()
}
