//! The condition builder: rows of field, operator and value, with nested groups (N6.02).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

craft_ui::words! {
    /// The builder's words, English by default (LightCraft's).
    #[non_exhaustive]
    pub struct ConditionLabels {
        match_word: "conditions.match" = "Match";
        of_rules: "conditions.ofRules" = "of the following rules:";
        all: "conditions.all" = "all";
        any: "conditions.any" = "any";
        none: "conditions.none" = "none";
        field: "conditions.field" = "Field";
        operator: "conditions.operator" = "Operator";
        value: "conditions.value" = "Value";
        and: "conditions.and" = "and";
        add_rule: "conditions.addRule" = "+ Rule";
        add_group: "conditions.addGroup" = "+ Group";
        add_rule_tip: "conditions.addRuleTip" = "Add a rule (⌥-click: a group)";
        remove_rule: "conditions.removeRule" = "Remove this rule";
        remove_group: "conditions.removeGroup" = "Remove this group";
    }
}

/// How a group's rules combine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Match {
    #[default]
    All,
    Any,
    None,
}

/// A value editor kind; the field's kind picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConditionKind {
    Text,
    Number,
    /// A date, edited with the kernel's [`DatePicker`](craft_ui::controls::DatePicker) (its value
    /// a [`PickedDate`](craft_ui::controls::PickedDate)).
    Date,
    Bool,
    /// One of the field's choices (an album, a label).
    Choice,
}

/// A rule's value.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ConditionValue {
    #[default]
    None,
    Text(String),
    Number(f64),
    /// Two numbers for "is between".
    Range(f64, f64),
    /// A date (a day, or a month or year for a partial date), the date picker's value.
    Date(craft_ui::controls::PickedDate),
    /// "in the last N units".
    Last {
        n: u32,
        unit: String,
    },
    Bool(bool),
    Choice(String),
}

/// One rule.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    pub value: ConditionValue,
}

/// A group of rules and nested groups.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConditionGroup {
    pub matching: Match,
    pub items: Vec<ConditionItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConditionItem {
    Rule(Condition),
    Group(ConditionGroup),
}

/// The fields a builder offers and their operators; the app's (LightCraft's photo fields,
/// WordCraft's recipient columns).
pub trait ConditionFields {
    /// `(id, label)` in menu order, grouped by `group` headings when given.
    fn fields(&self) -> Vec<(String, String, Option<String>)>;
    fn kind(&self, field: &str) -> ConditionKind;
    /// `(id, label)` of the operators for `field` ("is", "contains", "is between").
    fn operators(&self, field: &str) -> Vec<(String, String)>;
    /// A sensible value for a new `field` / `operator` (LightCraft `rules_editor.rs:17`).
    fn default_value(&self, field: &str, operator: &str) -> ConditionValue;
    /// The choices of a `Choice` field.
    fn choices(&self, field: &str) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Why a rule can't apply, shown under it ("Enter a number").
    fn problem(&self, rule: &Condition) -> Option<String> {
        None
    }
}

/// LightCraft's smart-collection rules editor: "Match [all ▾] of the following rules:", then a
/// row per rule: field menu, operator menu, the value editor for the field's kind ("and" between
/// two values of a range), − (remove) and + (add a rule after it; Alt-click: a group); nested
/// groups indent with their own Match row and −, up to depth 3; "+ Rule" and "+ Group" at the
/// bottom. Changing a field resets operator and value to the defaults. Changes `group` directly
/// and reports `changed`. Every menu, field and button is a Tab stop in reading order.
pub struct ConditionBuilder<'a> {
    id_salt: IdSalt,
    group: &'a mut ConditionGroup,
    fields: &'a dyn ConditionFields,
    labels: Option<ConditionLabels>,
}

impl<'a> ConditionBuilder<'a> {
    pub fn new(id_salt: impl AsIdSalt, group: &'a mut ConditionGroup, fields: &'a dyn ConditionFields) -> Self {
        Self { id_salt: IdSalt::new(id_salt), group, fields, labels: None }
    }

    pub fn labels(mut self, labels: ConditionLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // lightcraft: crates/ui-egui/src/dialogs/rules_editor.rs:305 `edit`, `:50` `match_combo`,
    // `:135` `value_editor`, `:241` `field_menu`, `:296` `problem_note`; wordcraft:
    // crates/ui-egui/src/dialogs/mailings.rs (filter and sort rows).
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
