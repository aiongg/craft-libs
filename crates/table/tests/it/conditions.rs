//! `ConditionBuilder`: field changes, adding and removing rules, date ranges and problems.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{Condition, ConditionBuilder, ConditionFields, ConditionGroup, ConditionItem, ConditionKind, ConditionValue, Match};
use craft_ui::contract::{Bind, Role};
use craft_ui::pickers::PickedDate;
use craft_ui::testing::access::name;
use craft_ui::testing::{Fixture, trial};
use egui::{Modifiers, Vec2};

/// The room the builder is shown in: wide enough for a rule on one row.
const ROOM: Vec2 = Vec2::new(900.0, 600.0);

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

const ADD: &str = "Add a rule (⌥-click: a group)";

fn rule(field: &str, operator: &str, value: ConditionValue) -> ConditionItem {
    ConditionItem::Rule(Condition::new(field, operator, value))
}

fn one_rule() -> ConditionGroup {
    ConditionGroup::new(Match::All, vec![rule("rating", "lte", ConditionValue::Number(5.0))])
}

fn build(group: ConditionGroup) -> (Fixture, Rc<RefCell<ConditionGroup>>) {
    let state = Rc::new(RefCell::new(group));
    let shown = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        ConditionBuilder::new("Rules", Bind::bound(&mut shown.borrow_mut()), &Photos).show(ui);
    });
    fixture.room(ROOM).settle();
    (fixture, state)
}

/// A new field takes its first operator and default value.
#[trial]
fn changing_the_field_resets_operator_and_value() {
    let (mut fixture, state) = build(one_rule());
    let field = fixture.control("Field").expect("the field menu");
    fixture.click(&field);
    fixture.idle(2);
    let choice = fixture.by_label("File name").expect("the field's entry");
    fixture.click(&choice);
    fixture.idle(2);
    let ConditionItem::Rule(rule) = &state.borrow().items[0] else { panic!("a rule") };
    assert_eq!(rule, &Condition::new("name", "contains", ConditionValue::Text(String::new())));
}

/// + adds a rule after the row, Alt+click a group.
#[trial]
fn plus_adds_a_rule_and_alt_plus_a_group() {
    let (mut fixture, state) = build(one_rule());
    let plus = fixture.control(ADD).expect("a + button");
    {
        let tree = fixture.tree();
        eprintln!("DIAG plus={:?}", plus.rect);
        for key in tree.keys() {
            if let Some(r) = tree.bounds_of(key) { if r.contains(plus.rect.center()) { eprintln!("DIAG over role={:?} name={:?} rect={:?}", tree.role_of(key), tree.name_of(key), r); } }
        }
    }
    fixture.click(&plus);
    fixture.idle(2);
    eprintln!("DIAG after items={:?} texts={:?}", state.borrow().items.len(), fixture.texts().len());
    assert_eq!(state.borrow().items.len(), 2);
    assert!(matches!(state.borrow().items[1], ConditionItem::Rule(_)));
    let plus = fixture.control(ADD).expect("a + button");
    fixture.hold(Modifiers::ALT);
    fixture.click(&plus);
    fixture.hold(Modifiers::NONE);
    fixture.idle(2);
    assert!(state.borrow().items.iter().any(|item| matches!(item, ConditionItem::Group(_))), "{:?}", state.borrow().items);
}

/// − removes the row or the group.
#[trial]
fn minus_removes_rules_and_groups() {
    let inner = ConditionGroup::new(Match::Any, vec![rule("name", "is", ConditionValue::Text("a".into()))]);
    let group = ConditionGroup::new(Match::All, vec![rule("rating", "gte", ConditionValue::Number(1.0)), ConditionItem::Group(inner)]);
    let (mut fixture, state) = build(group);
    let minus = fixture.control("Remove this group").expect("a group's −");
    fixture.click(&minus);
    fixture.idle(2);
    assert_eq!(state.borrow().items.len(), 1);
    let minus = fixture.control("Remove this rule").expect("a rule's −");
    fixture.click(&minus);
    fixture.idle(2);
    assert!(state.borrow().items.is_empty());
}

/// "is between" on a date edits two dates, each picker stopping at the other.
#[trial]
fn a_between_date_rule_shows_two_date_pickers() {
    let range = ConditionValue::DateRange(PickedDate::day(2026, 3, 1), PickedDate::day(2026, 4, 1));
    let (fixture, state) = build(ConditionGroup::new(Match::All, vec![rule("taken", "between", range.clone())]));
    let calendars = fixture.by_role(Role::BUTTON).into_iter().filter(|target| name(fixture.tree(), target.node) == Some("Pick a date")).count();
    assert_eq!(calendars, 2, "two date fields, each with its calendar button");
    assert_eq!(state.borrow().items[0], rule("taken", "between", range), "the value is kept");
}

/// A group's problem shows under it.
#[trial]
fn a_group_problem_shows_under_the_group() {
    let (fixture, _) = build(one_rule());
    let tree = fixture.tree();
    assert!(tree.keys().into_iter().any(|key| tree.name_of(key).is_some_and(|name| name.contains("Rules overlap"))));
}
