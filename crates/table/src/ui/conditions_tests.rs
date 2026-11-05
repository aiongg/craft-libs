//! The condition builder.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::controls::PickedDate;
use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::{Modifiers, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::conditions::{Condition, ConditionBuilder, ConditionFields, ConditionGroup, ConditionItem, ConditionKind, ConditionValue, Match};

struct Photos;

impl ConditionFields for Photos {
    fn fields(&self) -> Vec<(String, String, Option<String>)> {
        vec![("rating".into(), "Rating".into(), None), ("name".into(), "File name".into(), None), ("taken".into(), "Taken".into(), None)]
    }

    fn kind(&self, field: &str) -> ConditionKind {
        match field {
            "rating" => ConditionKind::Number,
            "taken" => ConditionKind::Date,
            _ => ConditionKind::Text,
        }
    }

    fn operators(&self, field: &str) -> Vec<(String, String)> {
        let ids: &[&str] = if field == "rating" { &["gte", "lte"] } else { &["contains", "is"] };
        ids.iter().map(|id| (id.to_string(), id.to_string())).collect()
    }

    fn default_value(&self, field: &str, _operator: &str) -> ConditionValue {
        match field {
            "rating" => ConditionValue::Number(3.0),
            "taken" => ConditionValue::DateRange(PickedDate::day(2026, 1, 1), PickedDate::day(2026, 12, 31)),
            _ => ConditionValue::Text(String::new()),
        }
    }

    fn group_problem(&self, path: &[usize], _group: &ConditionGroup) -> Option<String> {
        path.is_empty().then(|| "Rules overlap".to_string())
    }
}

fn rule(field: &str, operator: &str, value: ConditionValue) -> ConditionItem {
    ConditionItem::Rule(Condition { field: field.into(), operator: operator.into(), value })
}

fn show(ui: &mut Ui, group: &mut ConditionGroup) {
    ConditionBuilder::new("rules", group, &Photos).show(ui);
}

fn build(group: ConditionGroup) -> Harness<'static, ConditionGroup> {
    let mut harness = Harness::<ConditionGroup>::builder()
        .with_size([700.0, 500.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, group);
    harness.run_steps(10);
    harness
}

fn one_rule() -> ConditionGroup {
    ConditionGroup { matching: Match::All, items: vec![rule("rating", "lte", ConditionValue::Number(5.0))] }
}

// a new field takes its first operator and default value.
#[test]
fn changing_the_field_resets_operator_and_value() {
    let mut harness = build(one_rule());
    harness.get_by_label("Field").click();
    harness.run_steps(10);
    harness.get_by_label("File name").click();
    harness.run_steps(10);
    let ConditionItem::Rule(rule) = &harness.state().items[0] else { panic!("a rule") };
    assert_eq!(rule, &Condition { field: "name".into(), operator: "contains".into(), value: ConditionValue::Text(String::new()) });
}

// + adds a rule after the row, Alt+click a group; groups stop at depth 3.
#[test]
fn plus_adds_a_rule_and_alt_plus_a_group() {
    let mut harness = build(one_rule());
    harness.get_by_label("Add a rule (⌥-click: a group)").click();
    harness.run_steps(10);
    assert_eq!(harness.state().items.len(), 2);
    assert!(matches!(harness.state().items[1], ConditionItem::Rule(_)));
    harness.get_all_by_label("Add a rule (⌥-click: a group)").next().expect("a + button").click_modifiers(Modifiers::ALT);
    harness.run_steps(10);
    assert!(matches!(harness.state().items[1], ConditionItem::Group(_)), "{:?}", harness.state().items);

    let mut deep = ConditionGroup { matching: Match::Any, items: vec![rule("rating", "gte", ConditionValue::Number(1.0))] };
    for _ in 0..3 {
        deep = ConditionGroup { matching: Match::All, items: vec![ConditionItem::Group(deep)] };
    }
    let mut harness = build(deep);
    let innermost = harness.get_all_by_label("Add a rule (⌥-click: a group)").next().expect("a + button");
    innermost.click_modifiers(Modifiers::ALT);
    harness.run_steps(10);
    let mut group = &harness.state().items[0];
    for _ in 0..2 {
        let ConditionItem::Group(inner) = group else { panic!("a group") };
        group = &inner.items[0];
    }
    let ConditionItem::Group(inner) = group else { panic!("a group") };
    assert!(matches!(inner.items[..], [ConditionItem::Rule(_), ConditionItem::Rule(_)]), "a depth-3 group gets a rule: {inner:?}");
}

// − removes the row or the group.
#[test]
fn minus_removes_rules_and_groups() {
    let inner = ConditionGroup { matching: Match::Any, items: vec![rule("name", "is", ConditionValue::Text("a".into()))] };
    let group = ConditionGroup { matching: Match::All, items: vec![rule("rating", "gte", ConditionValue::Number(1.0)), ConditionItem::Group(inner)] };
    let mut harness = build(group);
    harness.get_all_by_label("Remove this group").next().expect("a group's −").click();
    harness.run_steps(10);
    assert_eq!(harness.state().items.len(), 1);
    harness.get_by_label("Remove this rule").click();
    harness.run_steps(10);
    assert!(harness.state().items.is_empty());
}

// Esc during a typing run puts the value from before the run back.
#[test]
fn esc_in_a_number_field_restores_the_value() {
    let mut harness = build(one_rule());
    harness.get_by_label("Value").click();
    harness.run_steps(5);
    harness.type_text("9");
    harness.run_steps(3);
    assert_eq!(harness.state().items[0], rule("rating", "lte", ConditionValue::Number(9.0)), "the typed number previews");
    harness.press(egui::KeyboardShortcut::new(Modifiers::NONE, egui::Key::Escape));
    harness.run_steps(5);
    assert_eq!(harness.state().items[0], rule("rating", "lte", ConditionValue::Number(5.0)));
}

// "is between" on a date edits two dates, each picker stopping at the other.
#[test]
fn a_between_date_rule_shows_two_date_pickers() {
    let range = ConditionValue::DateRange(PickedDate::day(2026, 3, 1), PickedDate::day(2026, 4, 1));
    let harness = build(ConditionGroup { matching: Match::All, items: vec![rule("taken", "between", range.clone())] });
    assert_eq!(harness.get_all_by_label("Pick a date").count(), 2);
    assert_eq!(harness.state().items[0], rule("taken", "between", range), "the value is kept");
}

// A group's problem shows under it, after the warning icon.
#[test]
fn a_group_problem_shows_under_the_group() {
    let harness = build(one_rule());
    harness.get_by_label("Rules overlap");
}
