//! The condition builder: rows of field, operator and value, with nested groups (N6.02).

use super::style::SheetStyle;
use craft_ui::controls::{Button, Checkbox, ControlSize, DatePicker, IconButton, NumberField, Select, TextField};
use craft_ui::direction;
use craft_ui::edit::{EditEvent, GestureDraft};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _, UiTokens, margin};
use egui::{Align, AsIdSalt, Frame, Id, IdSalt, Layout, Response, RichText, Sense, Ui, vec2};

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
        unit: "conditions.unit" = "Unit";
        bool_rule: "conditions.boolRule" = "Is set";
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
    /// Two dates for "is between": each picker stops at the other, so the two stay in order.
    DateRange(craft_ui::controls::PickedDate, craft_ui::controls::PickedDate),
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
    fn choices(&self, _field: &str) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Why a rule can't apply, shown under it ("Enter a number").
    fn problem(&self, _rule: &Condition) -> Option<String> {
        None
    }
    /// Why a group can't apply, shown under it with the warning icon (LightCraft
    /// `rules_editor.rs:299`, `:333`). `path` is the group's place: the indices from the top
    /// group, empty for the top group itself.
    fn group_problem(&self, _path: &[usize], _group: &ConditionGroup) -> Option<String> {
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
        let Self { id_salt, group, fields, labels } = self;
        let labels = labels.unwrap_or_else(|| ConditionLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let cx = Cx { id, fields, labels: &labels, numbers: &numbers, tokens: &tokens };
        let mut changed = false;
        let mut response = ui.with_layout(direction::column(ui), |ui| edit_group(&cx, ui, group, &mut Vec::new(), &mut changed)).response;
        if changed {
            response.mark_changed();
        }
        response
    }
}

/// How deep groups nest: a group inside a group at this depth can't be added.
const MAX_DEPTH: usize = 3;

/// What every row of a pass reads.
struct Cx<'a> {
    id: Id,
    fields: &'a dyn ConditionFields,
    labels: &'a ConditionLabels,
    numbers: &'a LocaleNumbers,
    tokens: &'a UiTokens,
}

impl Cx<'_> {
    /// A new rule: the first field, its first operator and their default value.
    fn new_rule(&self) -> Condition {
        let Some((field, ..)) = self.fields.fields().into_iter().next() else { return Condition::default() };
        let operator = self.fields.operators(&field).into_iter().next().map(|(id, _)| id).unwrap_or_default();
        let value = self.fields.default_value(&field, &operator);
        Condition { field, operator, value }
    }

    fn new_group(&self) -> ConditionGroup {
        ConditionGroup { matching: Match::Any, items: vec![ConditionItem::Rule(self.new_rule())] }
    }

    /// Whether `value` is one the field's kind edits.
    fn fits(&self, field: &str, value: &ConditionValue) -> bool {
        use ConditionKind as K;
        use ConditionValue as V;
        matches!(
            (self.fields.kind(field), value),
            (_, V::None)
                | (K::Text, V::Text(_))
                | (K::Number, V::Number(_) | V::Range(..))
                | (K::Date, V::Date(_) | V::DateRange(..) | V::Last { .. })
                | (K::Bool, V::Bool(_))
                | (K::Choice, V::Choice(_))
        )
    }

    fn id(&self, path: &[usize], part: &str) -> Id {
        self.id.with((path, part))
    }
}

/// The value of a `Commit` or `Set`: a control that writes into a local value (a menu, a picker)
/// is read only when it settles, so a preview or a cancelled pick changes nothing.
fn settled<T>(event: &Option<EditEvent<T>>) -> Option<&T> {
    match event {
        Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => Some(value),
        _ => None,
    }
}

/// A row whose items start at the top, in reading order.
fn top_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) };
    ui.allocate_ui_with_layout(vec2(ui.available_width(), ui.spacing().interact_size.y), layout, add).inner
}

/// The group at `path`: its Match row, its rules and groups, and the buttons that add more.
fn edit_group(cx: &Cx<'_>, ui: &mut Ui, group: &mut ConditionGroup, path: &mut Vec<usize>, changed: &mut bool) {
    let labels = cx.labels;
    let depth = path.len();
    direction::horizontal(ui, |ui| {
        ui.label(&labels.match_word);
        let mut matching = group.matching;
        let picked = Select::new(cx.id(path, "match"), &labels.match_word, &mut matching)
            .option(Match::All, &labels.all)
            .option(Match::Any, &labels.any)
            .option(Match::None, &labels.none)
            .show(ui);
        if settled(&picked.event).is_some() && matching != group.matching {
            group.matching = matching;
            *changed = true;
        }
        ui.label(&labels.of_rules);
    });
    let mut remove = None;
    let mut insert = None;
    for (at, item) in group.items.iter_mut().enumerate() {
        path.push(at);
        match item {
            ConditionItem::Group(nested) => {
                let frame = Frame::new()
                    .inner_margin(margin(cx.tokens.space.pad.control))
                    .stroke(Line::Hairline.egui(ui.ctx(), cx.tokens.color.line.divider.to_egui()))
                    .corner_radius(cx.tokens.radius.control);
                frame.show(ui, |ui| {
                    top_row(ui, |ui| {
                        let button = IconButton::new(Icon::Minus, &labels.remove_group).size(ControlSize::Sm).id_salt(cx.id(path, "remove"));
                        if ui.add(button).clicked() {
                            remove = Some(at);
                        }
                        ui.with_layout(direction::column(ui), |ui| edit_group(cx, ui, nested, path, changed));
                    });
                });
            }
            ConditionItem::Rule(rule) => edit_rule(cx, ui, rule, path, depth, (at, &mut remove, &mut insert), changed),
        }
        path.pop();
    }
    if let Some(at) = remove {
        group.items.remove(at);
        *changed = true;
    }
    if let Some((at, item)) = insert {
        group.items.insert(at.min(group.items.len()), item);
        *changed = true;
    }
    if let Some(problem) = cx.fields.group_problem(path, group) {
        problem_note(cx, ui, &problem);
    }
    if group.items.is_empty() || depth == 0 {
        direction::horizontal(ui, |ui| {
            if ui.add(Button::new(&labels.add_rule).size(ControlSize::Sm).id_salt(cx.id(path, "add-rule"))).clicked() {
                group.items.push(ConditionItem::Rule(cx.new_rule()));
                *changed = true;
            }
            if depth < MAX_DEPTH && ui.add(Button::new(&labels.add_group).size(ControlSize::Sm).id_salt(cx.id(path, "add-group"))).clicked() {
                group.items.push(ConditionItem::Group(cx.new_group()));
                *changed = true;
            }
        });
    }
}

type Pending<'a> = (usize, &'a mut Option<usize>, &'a mut Option<(usize, ConditionItem)>);

/// One rule: field, operator, value editor, the problem under it, and − and + at the end.
fn edit_rule(cx: &Cx<'_>, ui: &mut Ui, rule: &mut Condition, path: &[usize], depth: usize, (at, remove, insert): Pending<'_>, changed: &mut bool) {
    let labels = cx.labels;
    direction::horizontal(ui, |ui| {
        pick_field(cx, ui, rule, path, changed);
        pick_operator(cx, ui, rule, path, changed);
        edit_value(cx, ui, rule, path, changed);
        let side = IconButton::style(cx.tokens, ControlSize::Sm).side;
        let buttons = 2.0 * side + ui.spacing().item_spacing.x;
        ui.add_space((ui.available_width() - buttons).max(0.0));
        if ui.add(IconButton::new(Icon::Minus, &labels.remove_rule).size(ControlSize::Sm).id_salt(cx.id(path, "remove"))).clicked() {
            *remove = Some(at);
        }
        let add = ui.add(IconButton::new(Icon::Plus, &labels.add_rule_tip).size(ControlSize::Sm).id_salt(cx.id(path, "add")));
        if add.clicked() {
            let item = if ui.input(|input| input.modifiers.alt) && depth < MAX_DEPTH {
                ConditionItem::Group(cx.new_group())
            } else {
                ConditionItem::Rule(cx.new_rule())
            };
            *insert = Some((at + 1, item));
        }
    });
    if let Some(problem) = cx.fields.problem(rule) {
        problem_note(cx, ui, &problem);
    }
}

/// A problem under a rule or a group: the warning icon, then the text, in the caution colour.
fn problem_note(cx: &Cx<'_>, ui: &mut Ui, problem: &str) {
    let color = cx.tokens.color.status.warn.fg.to_egui();
    let font = cx.tokens.font(SheetStyle::of(cx.tokens).problem_text);
    direction::horizontal(ui, |ui| {
        let side = IconSize::Sm.points(cx.tokens);
        let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        icons::paint(ui, rect, Icon::Warning, IconSize::Sm, color);
        ui.label(RichText::new(problem).font(font).color(color));
    });
}

/// Applies a number or text control's event to the rule's value through the rule's draft: a
/// preview shows while the gesture runs, and Esc restores the value from before it began.
fn write(cx: &Cx<'_>, ui: &Ui, path: &[usize], value: &mut ConditionValue, event: Option<EditEvent<ConditionValue>>, changed: &mut bool) {
    let Some(event) = event else { return };
    let key = cx.id(path, "draft");
    let mut draft = ui.data(|d| d.get_temp::<GestureDraft<ConditionValue>>(key)).unwrap_or_default();
    let before = value.clone();
    draft.apply(value, event);
    ui.data_mut(|d| d.insert_temp(key, draft));
    if *value != before {
        *changed = true;
    }
}

/// The field menu; a new field takes its first operator and the default value.
fn pick_field(cx: &Cx<'_>, ui: &mut Ui, rule: &mut Condition, path: &[usize], changed: &mut bool) {
    let mut field = rule.field.clone();
    let mut select = Select::new(cx.id(path, "field"), &cx.labels.field, &mut field);
    let mut heading: Option<String> = None;
    for (id, label, group) in cx.fields.fields() {
        if group != heading {
            select = match &group {
                Some(group) => select.group(group),
                None => select.separator(),
            };
            heading = group;
        }
        select = select.option(id, label);
    }
    let picked = select.show(ui);
    if settled(&picked.event).is_some() && field != rule.field {
        rule.operator = cx.fields.operators(&field).into_iter().next().map(|(id, _)| id).unwrap_or_default();
        rule.value = cx.fields.default_value(&field, &rule.operator);
        rule.field = field;
        *changed = true;
    }
}

/// The operator menu; an operator that wants another kind of value takes its default.
fn pick_operator(cx: &Cx<'_>, ui: &mut Ui, rule: &mut Condition, path: &[usize], changed: &mut bool) {
    let mut operator = rule.operator.clone();
    let mut select = Select::new(cx.id(path, "operator"), &cx.labels.operator, &mut operator);
    for (id, label) in cx.fields.operators(&rule.field) {
        select = select.option(id, label);
    }
    let picked = select.show(ui);
    if settled(&picked.event).is_some() && operator != rule.operator {
        let value = cx.fields.default_value(&rule.field, &operator);
        if std::mem::discriminant(&value) != std::mem::discriminant(&rule.value) {
            rule.value = value;
        }
        rule.operator = operator;
        *changed = true;
    }
}

/// The value editor the field's kind and the operator's value call for. Number and text controls
/// write through the rule's draft ([`write`]); menus, pickers and the check box write when they
/// settle.
fn edit_value(cx: &Cx<'_>, ui: &mut Ui, rule: &mut Condition, path: &[usize], changed: &mut bool) {
    let labels = cx.labels;
    if !cx.fits(&rule.field, &rule.value) {
        rule.value = cx.fields.default_value(&rule.field, &rule.operator);
        *changed = true;
    }
    let number = |ui: &mut Ui, part: &str, value: f64| NumberField::new(cx.id(path, part), &labels.value, value, cx.numbers).show(ui).event;
    let current = rule.value.clone();
    let settled_value = match &current {
        ConditionValue::None => return,
        ConditionValue::Text(text) => {
            let event = TextField::new(cx.id(path, "value"), &labels.value, text).show(ui).event;
            write(cx, ui, path, &mut rule.value, event.map(|event| event.map(ConditionValue::Text)), changed);
            None
        }
        ConditionValue::Number(value) => {
            let event = number(ui, "value", *value);
            write(cx, ui, path, &mut rule.value, event.map(|event| event.map(ConditionValue::Number)), changed);
            None
        }
        ConditionValue::Range(low, high) => {
            let low_event = number(ui, "low", *low);
            ui.label(&labels.and);
            let high_event = number(ui, "high", *high);
            write(cx, ui, path, &mut rule.value, low_event.map(|event| event.map(|low| ConditionValue::Range(low, *high))), changed);
            write(cx, ui, path, &mut rule.value, high_event.map(|event| event.map(|high| ConditionValue::Range(*low, high))), changed);
            None
        }
        ConditionValue::Date(date) => {
            let mut picked = Some(*date);
            let out = DatePicker::new(cx.id(path, "value"), &labels.value, &mut picked).show(ui);
            settled(&out.event).copied().flatten().map(ConditionValue::Date)
        }
        ConditionValue::DateRange(low, high) => {
            // Each picker stops at the other date (LightCraft `rules_editor.rs:187-193`).
            let (mut low_picked, mut high_picked) = (Some(*low), Some(*high));
            let low_out = DatePicker::new(cx.id(path, "low"), &labels.value, &mut low_picked).max(Some(*high)).show(ui);
            ui.label(&labels.and);
            let high_out = DatePicker::new(cx.id(path, "high"), &labels.value, &mut high_picked).min(Some(*low)).show(ui);
            let (new_low, new_high) = (settled(&low_out.event).copied().flatten(), settled(&high_out.event).copied().flatten());
            (new_low.is_some() || new_high.is_some()).then(|| ConditionValue::DateRange(new_low.unwrap_or(*low), new_high.unwrap_or(*high)))
        }
        ConditionValue::Last { n, unit } => {
            let count = number(ui, "count", f64::from(*n));
            let units = cx.fields.choices(&rule.field);
            let unit_value = if units.is_empty() {
                let event = TextField::new(cx.id(path, "unit"), &labels.unit, unit).show(ui).event;
                write(cx, ui, path, &mut rule.value, event.map(|event| event.map(|unit| ConditionValue::Last { n: *n, unit })), changed);
                None
            } else {
                let mut picked = unit.clone();
                let mut select = Select::new(cx.id(path, "unit"), &labels.unit, &mut picked);
                for (id, label) in units {
                    select = select.option(id, label);
                }
                let out = select.show(ui);
                settled(&out.event).is_some().then_some(picked)
            };
            let to_count = |count: f64| count.max(0.0).round() as u32;
            let unit_now = match &rule.value {
                ConditionValue::Last { unit, .. } => unit.clone(),
                _ => unit.clone(),
            };
            write(
                cx,
                ui,
                path,
                &mut rule.value,
                count.map(|event| event.map(|count| ConditionValue::Last { n: to_count(count), unit: unit_now })),
                changed,
            );
            unit_value.filter(|picked| picked != unit).map(|unit| ConditionValue::Last { n: *n, unit })
        }
        ConditionValue::Bool(on) => {
            let label =
                cx.fields.fields().into_iter().find(|(id, ..)| *id == rule.field).map_or_else(|| labels.bool_rule.clone(), |(_, label, _)| label);
            let mut checked = *on;
            let out = Checkbox::bool(label, &mut checked).id_salt(cx.id(path, "value")).show(ui);
            settled(&out.event).map(|on| ConditionValue::Bool(*on))
        }
        ConditionValue::Choice(choice) => {
            let mut picked = choice.clone();
            let mut select = Select::new(cx.id(path, "value"), &labels.value, &mut picked);
            for (id, label) in cx.fields.choices(&rule.field) {
                select = select.option(id, label);
            }
            let out = select.show(ui);
            settled(&out.event).is_some().then_some(ConditionValue::Choice(picked))
        }
    };
    if let Some(value) = settled_value.filter(|value| *value != rule.value) {
        rule.value = value;
        *changed = true;
    }
}
