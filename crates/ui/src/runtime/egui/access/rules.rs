//! `RoleRule`: the fields each AccessKit role requires (12.3). A violation is logged in debug builds and is a
//! finding the conformance run fails on.

use crate::runtime::access::{Numeric, SetPosition};
use accesskit::Role;

/// What a node carries, as far as the rules read it.
pub(crate) struct RoleFacts<'a> {
    pub role: Role,
    pub named: bool,
    pub numeric: Option<&'a Numeric>,
    pub position: Option<SetPosition>,
    pub level: Option<usize>,
    pub expanded: Option<bool>,
    pub controls: bool,
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

/// The rules `facts` breaks, each as a short phrase.
pub(crate) fn role_violations(facts: &RoleFacts<'_>) -> Vec<&'static str> {
    let mut found = Vec::new();
    if NAMED.contains(&facts.role) && !facts.named {
        found.push("a name");
    }
    match facts.role {
        Role::Slider => {
            let finite = facts.numeric.is_some_and(|numeric| numeric.range.start().is_finite() && numeric.range.end().is_finite());
            if !finite {
                found.push("a finite range");
            }
        }
        Role::Tab | Role::ListBoxOption | Role::TreeItem => {
            if facts.position.is_none() {
                found.push("a position in its set");
            }
        }
        Role::ComboBox => {
            if facts.expanded.is_none() {
                found.push("an expanded state");
            } else if facts.expanded == Some(true) && !facts.controls {
                found.push("`controls` while open");
            }
        }
        Role::Heading if facts.level.is_none() => found.push("a level"),
        _ => {}
    }
    found
}
