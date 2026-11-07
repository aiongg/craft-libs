//! Reads a kittest node's AccessKit data into the parity tree's lines.

use super::{Action, Role, Toggled};
use crate::testing::parity::{KitNode as Node, KitNodeT as _, Line};
use std::collections::HashMap;
use std::fmt::Write as _;

/// The actions a line lists, when the node supports them.
const ACTIONS: [Action; 18] = [
    Action::Click,
    Action::Focus,
    Action::Blur,
    Action::Collapse,
    Action::Expand,
    Action::Decrement,
    Action::Increment,
    Action::SetValue,
    Action::ShowContextMenu,
    Action::ScrollIntoView,
    Action::ScrollDown,
    Action::ScrollUp,
    Action::ScrollLeft,
    Action::ScrollRight,
    Action::ReplaceSelectedText,
    Action::SetTextSelection,
    Action::HideTooltip,
    Action::ShowTooltip,
];

pub(crate) fn child_path(parent: &str, index: usize) -> String {
    if parent == "r" { format!("r.{index}") } else { format!("{parent}.{index}") }
}

pub(crate) fn paths_of(node: &Node<'_>, path: &str, paths: &mut HashMap<String, String>) {
    paths.insert(format!("{:?}", node.accesskit_node().id()), path.to_owned());
    for (i, child) in node.children().enumerate() {
        paths_of(&child, &child_path(path, i), paths);
    }
}

pub(crate) fn line_of(node: &Node<'_>, path: &str, depth: usize, paths: &HashMap<String, String>) -> Line {
    let ak = node.accesskit_node();
    let data = ak.data();
    let name = ak.label().unwrap_or_default();
    let mut text = format!("{:?} {name:?}", ak.role());
    let mut put = |label: &str, value: String| {
        let _ = write!(text, " {label}={value}");
    };
    if let Some(value) = ak.value() {
        put("value", format!("{value:?}"));
    }
    if let Some(description) = ak.description() {
        put("description", format!("{description:?}"));
    }
    if let Some(placeholder) = ak.placeholder() {
        put("placeholder", format!("{placeholder:?}"));
    }
    if let Some(shortcut) = data.keyboard_shortcut() {
        put("shortcut", format!("{shortcut:?}"));
    }
    if let Some(n) = ak.numeric_value() {
        let range = |n: Option<f64>| n.map_or_else(|| "-".to_owned(), |n| format!("{n}"));
        put("number", format!("{n} in {}..{} step {}", range(ak.min_numeric_value()), range(ak.max_numeric_value()), range(ak.numeric_value_step())));
    }
    let related: [(&str, Vec<&str>); 2] = [
        ("labelled_by", ak.labelled_by().map(|n| paths.get(&format!("{:?}", n.id())).map_or("?", String::as_str)).collect()),
        ("controls", ak.controls().map(|n| paths.get(&format!("{:?}", n.id())).map_or("?", String::as_str)).collect()),
    ];
    for (label, targets) in related {
        if !targets.is_empty() {
            put(label, format!("[{}]", targets.join(",")));
        }
    }
    let mut flags: Vec<String> = Vec::new();
    if ak.is_hidden() {
        flags.push("hidden".into());
    }
    if ak.is_disabled() {
        flags.push("disabled".into());
    }
    if ak.is_read_only()
        && matches!(ak.role(), Role::TextInput | Role::MultilineTextInput | Role::SearchInput | Role::PasswordInput | Role::NumberInput)
    {
        flags.push("read-only".into());
    }
    if ak.is_required() {
        flags.push("required".into());
    }
    if let Some(toggled) = ak.toggled() {
        flags.push(match toggled {
            Toggled::False => "unchecked".into(),
            Toggled::True => "checked".into(),
            Toggled::Mixed => "mixed".into(),
        });
    }
    if let Some(expanded) = data.is_expanded() {
        flags.push(if expanded { "expanded" } else { "collapsed" }.into());
    }
    if let Some(selected) = ak.is_selected() {
        flags.push(if selected { "selected" } else { "unselected" }.into());
    }
    if let Some(popup) = ak.has_popup() {
        flags.push(format!("popup:{popup:?}"));
    }
    if !flags.is_empty() {
        let _ = write!(text, " [{}]", flags.join(" "));
    }
    let actions: Vec<Action> = ACTIONS.into_iter().filter(|a| data.supports_action(*a)).collect();
    if !actions.is_empty() {
        let names: Vec<String> = actions.iter().map(|a| format!("{a:?}")).collect();
        let _ = write!(text, " actions=[{}]", names.join(","));
    }
    Line {
        depth,
        path: path.to_owned(),
        text,
        focused: ak.is_focused(),
        role: ak.role(),
        name,
        hidden: ak.is_hidden(),
        disabled: ak.is_disabled(),
        actions,
    }
}
