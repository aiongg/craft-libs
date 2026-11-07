//! `RoleRule`: the fields each AccessKit role requires (spec 12.3).

use accesskit::Role;
use std::ops::RangeInclusive;

/// What a node carries, as far as the rules read it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RoleRule<'a> {
    /// The node's role.
    pub role: Role,
    /// The node has a name, a labelling node or is hidden on purpose.
    pub named: bool,
    /// The numeric range, when the node has one.
    pub range: Option<&'a RangeInclusive<f64>>,
    /// The node knows its place in a set.
    pub positioned: bool,
    /// The node has a heading level or tree depth.
    pub leveled: bool,
    /// The expanded state, `None` while the node has none.
    pub expanded: Option<bool>,
    /// The node controls another node.
    pub controls: bool,
}

impl<'a> RoleRule<'a> {
    /// A node of `role` that carries nothing yet.
    pub const fn new(role: Role) -> Self {
        RoleRule { role, named: false, range: None, positioned: false, leveled: false, expanded: None, controls: false }
    }

    /// The node has a name, a labelling node or is hidden on purpose.
    #[must_use]
    pub const fn named(mut self, named: bool) -> Self {
        self.named = named;
        self
    }

    /// The node's numeric range.
    #[must_use]
    pub const fn range(mut self, range: Option<&'a RangeInclusive<f64>>) -> Self {
        self.range = range;
        self
    }

    /// The node knows its place in a set.
    #[must_use]
    pub const fn positioned(mut self, positioned: bool) -> Self {
        self.positioned = positioned;
        self
    }

    /// The node has a heading level or tree depth.
    #[must_use]
    pub const fn leveled(mut self, leveled: bool) -> Self {
        self.leveled = leveled;
        self
    }

    /// The node's expanded state.
    #[must_use]
    pub const fn expanded(mut self, expanded: Option<bool>) -> Self {
        self.expanded = expanded;
        self
    }

    /// The node controls another node.
    #[must_use]
    pub const fn controls(mut self, controls: bool) -> Self {
        self.controls = controls;
        self
    }
}

/// Roles that are checked or unchecked: a node of one always carries a toggled state.
pub fn is_checkable(role: Role) -> bool {
    matches!(role, Role::CheckBox | Role::Switch | Role::RadioButton | Role::MenuItemCheckBox | Role::MenuItemRadio)
}

/// Roles that are selected or not: a node of one always carries a selected state.
pub fn is_selectable(role: Role) -> bool {
    matches!(role, Role::Tab | Role::ListItem | Role::ListBoxOption | Role::TreeItem | Role::Row | Role::Cell)
}

/// Roles a screen reader announces by name.
const NAMED: &[Role] = &[
    Role::Button,
    Role::CheckBox,
    Role::RadioButton,
    Role::Switch,
    Role::Link,
    Role::Tab,
    Role::MenuItem,
    Role::MenuItemCheckBox,
    Role::MenuItemRadio,
    Role::ListBoxOption,
    Role::TreeItem,
    Role::Slider,
    Role::SpinButton,
    Role::ComboBox,
    Role::TextInput,
    Role::MultilineTextInput,
];

/// The rules `rule` breaks, each as a short phrase.
pub fn role_violations(rule: &RoleRule<'_>) -> Vec<&'static str> {
    let mut found = Vec::new();
    if NAMED.contains(&rule.role) && !rule.named {
        found.push("a name");
    }
    match rule.role {
        Role::Slider => {
            if !rule.range.is_some_and(|range| range.start().is_finite() && range.end().is_finite()) {
                found.push("a finite range");
            }
        }
        Role::Tab | Role::ListBoxOption | Role::TreeItem => {
            if !rule.positioned {
                found.push("a position in its set");
            }
        }
        Role::ComboBox => {
            if rule.expanded.is_none() {
                found.push("an expanded state");
            } else if rule.expanded == Some(true) && !rule.controls {
                found.push("`controls` while open");
            }
        }
        Role::Heading if !rule.leveled => found.push("a level"),
        _ => {}
    }
    found
}
