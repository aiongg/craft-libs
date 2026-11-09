//! `ConditionBuilder`: field changes, adding and removing rules, date ranges and problems.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{Condition, ConditionBuilder, ConditionFields, ConditionGroup, ConditionItem, ConditionKind, ConditionValue, Match};
use craft_ui::contract::{Bind, Role};
use craft_ui::pickers::PickedDate;
use craft_ui::testing::access::name;
use craft_ui::testing::{Fixture, trial};
use egui::{Modifiers, Vec2};

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
    let field = fixture.by_label("Field").expect("the field menu");
    fixture.click(&field);
    fixture.idle(2);
    let choice = fixture.by_label("File name").expect("the field's entry");
    fixture.click(&choice);
    fixture.idle(2);
    let ConditionItem::Rule(rule) = &state.borrow().items[0] else { panic!("a rule") };
    assert_eq!(rule, &Condition::new("name", "contains", ConditionValue::Text(String::new())));
}

