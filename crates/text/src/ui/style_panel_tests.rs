//! The styles panel in a harness.

use craft_ui::localized::Labels as _;
use craft_ui::menu::facts_id;
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
                app.actions.extend(r.actions);
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
    // The second click of a double-click reports its Apply and then Options, in that order.
    let n = harness.state().actions.len();
    assert_eq!(harness.state().actions[n - 2..], [TextStyleAction::Apply { id: 2, clear_overrides: false }, TextStyleAction::Options(2)]);
}

// Delete and Backspace both delete the picked style while the list has the keyboard.
#[test]
fn delete_and_backspace_delete_the_picked_style() {
    let mut harness = harness(TextStyleKind::Paragraph, &["Basic Paragraph", "Heading", "Body"], 1, false, true);
    harness.run();
    harness.get_by_label("Body").click();
    harness.run();
    for key in [Key::Delete, Key::Backspace] {
        harness.state_mut().actions.clear();
        harness.key_press(key);
        harness.run();
        assert_eq!(harness.state().actions, [TextStyleAction::Delete(2)], "{key:?}");
    }
}

// The footer keeps the end group's buttons clear of the start group's: at a width that can't hold
// all six, a press on the part of Redefine that sticks out past Options reaches neither it nor Clear Override.
#[test]
fn a_narrow_footer_leaves_the_end_buttons_uncovered() {
    let labels = TextStylesLabels::english();
    let build = |width: f32| {
        let app = App {
            kind: TextStyleKind::Paragraph,
            rows: rows(&["Basic Paragraph", "Heading"]),
            state: TextStylesState::default(),
            current: Some(1),
            overridden: true,
            text_selected: true,
            actions: Vec::new(),
            statuses: Vec::new(),
        };
        let mut harness = HarnessBuilder::default()
            .with_step_dt(0.05)
            .with_size(egui::vec2(width, 300.0))
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui_state(
                |ui, app: &mut App| {
                    let r = TextStylesPanel::new("styles", app.kind, &app.rows, &mut app.state)
                        .current(app.current, app.overridden)
                        .target(true)
                        .show(ui);
                    app.actions.extend(r.actions);
                },
                app,
            );
        harness.run();
        harness
    };
    // The room of four buttons can't hold the six.
    let one = build(150.0).get_by_label(&labels.options).rect().width();
    let mut harness = build(4.0 * one);
    let options = harness.get_by_label(&labels.options).rect();
    let redefine = harness.get_by_label(&labels.redefine).rect();
    assert!(redefine.right() > options.right(), "the width must be too narrow for the six buttons: {redefine:?} {options:?}");

    // Redefine's right edge sticks out past Options; the start group is clipped before Options.
    let at = egui::pos2(redefine.right() - 1.0, redefine.center().y);
    for frame in craft_ui::testing::events::click(at, egui::PointerButton::Primary, 1, Modifiers::NONE) {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
    harness.run();
    let actions = &harness.state().actions;
    assert!(!actions.iter().any(|a| matches!(a, TextStyleAction::Redefine | TextStyleAction::ClearOverride)), "{actions:?}");
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
fn highlighted_rows_act_together() {
    use craft_ui::behavior::Selection;

    // `drop_at` also removes the pointer, which a drop target needs on the release pass.
    fn release(harness: &Harness<'_, Highlighting>, at: egui::Pos2) {
        harness.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    }

    struct Highlighting {
        rows: Vec<TextStyleRow>,
        state: TextStylesState,
        highlighted: Selection<u64>,
        actions: Vec<TextStyleAction>,
    }
    let app = Highlighting {
        rows: rows(&["Basic Paragraph", "A", "B", "C", "D"]),
        state: TextStylesState::default(),
        highlighted: Selection::new(),
        actions: Vec::new(),
    };
    let mut harness = HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(320.0, 400.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut Highlighting| {
                let r = TextStylesPanel::new("styles", TextStyleKind::Paragraph, &app.rows, &mut app.state)
                    .target(true)
                    .highlighted(&mut app.highlighted)
                    .show(ui);
                app.actions.extend(r.actions);
            },
            app,
        );
    harness.run();
    let labels = TextStylesLabels::english();

    harness.get_by_label("A").click();
    harness.run();
    assert_eq!(harness.state().actions, [TextStyleAction::Apply { id: 1, clear_overrides: false }]);
    harness.state_mut().actions.clear();

    harness.get_by_label("C").click_modifiers(Modifiers::SHIFT);
    harness.run();
    harness.get_by_label("D").click_modifiers(Modifiers::COMMAND);
    harness.run();
    let mut items = harness.state().highlighted.items.clone();
    items.sort_unstable();
    assert_eq!(items, [1, 2, 3, 4]);
    assert!(harness.state().actions.is_empty(), "{:?}", harness.state().actions);

    // The panel menu's facts sit under the kernel's id and count the highlight, so its Duplicate
    // and Delete act on every highlighted style.
    let ctx = harness.ctx.clone();
    let facts = ctx.data(|d| d.get_temp::<MenuFacts>(facts_id(egui::IdSalt::new("styles"))));
    assert_eq!(facts.map(|facts| facts.highlighted), Some(4));
    let mut state = TextStylesState::default();
    assert_eq!(TextStylesPanel::run_command(&ctx, "styles", &mut state, "textStyles.duplicate"), Some(TextStyleAction::DuplicateHighlighted));
    assert_eq!(TextStylesPanel::run_command(&ctx, "styles", &mut state, "textStyles.delete"), Some(TextStyleAction::DeleteHighlighted));

    harness.key_press(Key::Delete);
    harness.run();
    assert_eq!(harness.state().actions, [TextStyleAction::DeleteHighlighted]);
    harness.state_mut().actions.clear();

    // A highlighted row dropped on New duplicates every highlighted style.
    let new = harness.get_by_label(&labels.new_style).rect().center();
    let from = harness.get_by_label("B").rect().center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(0.0, 12.0));
    harness.run();
    harness.hover_at(new);
    harness.run();
    release(&harness, new);
    harness.run();
    assert_eq!(harness.state().actions, [TextStyleAction::DuplicateHighlighted]);
    harness.state_mut().actions.clear();

    // An unhighlighted row dropped there duplicates itself only.
    harness.get_by_label("Basic Paragraph").click();
    harness.run();
    harness.state_mut().actions.clear();
    let from = harness.get_by_label("D").rect().center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(0.0, -12.0));
    harness.run();
    harness.hover_at(new);
    harness.run();
    release(&harness, new);
    harness.run();
    assert_eq!(harness.state().actions, [TextStyleAction::Duplicate(4)]);
}
