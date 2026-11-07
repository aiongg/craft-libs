//! Reads a node's AccessKit data into the parity tree's lines.

use super::{AccessTree, Action, NodeKey, Role, Toggled};
use crate::testing::parity::Line;
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

/// The path of every node below `key`, by AccessKit node number.
pub(crate) fn paths_of(tree: &AccessTree, key: NodeKey, path: &str, paths: &mut HashMap<u64, String>) {
    paths.insert(key.raw(), path.to_owned());
    for (i, child) in tree.children_of(key).into_iter().enumerate() {
        paths_of(tree, child, &child_path(path, i), paths);
    }
}

pub(crate) fn line_of(tree: &AccessTree, key: NodeKey, path: &str, depth: usize, paths: &HashMap<u64, String>) -> Option<Line> {
    let data = tree.data_of(key)?;
    let name = data.label().unwrap_or_default().to_owned();
    let mut text = format!("{:?} {name:?}", data.role());
    let mut put = |label: &str, value: String| {
        let _ = write!(text, " {label}={value}");
    };
    if let Some(value) = data.value() {
        put("value", format!("{value:?}"));
    }
    if let Some(description) = data.description() {
        put("description", format!("{description:?}"));
    }
    if let Some(placeholder) = data.placeholder() {
        put("placeholder", format!("{placeholder:?}"));
    }
    if let Some(shortcut) = data.keyboard_shortcut() {
        put("shortcut", format!("{shortcut:?}"));
    }
    if let Some(n) = data.numeric_value() {
        let range = |n: Option<f64>| n.map_or_else(|| "-".to_owned(), |n| format!("{n}"));
        put(
            "number",
            format!("{n} in {}..{} step {}", range(data.min_numeric_value()), range(data.max_numeric_value()), range(data.numeric_value_step())),
        );
    }
    let targets =
        |ids: &[accesskit::NodeId]| -> Vec<String> { ids.iter().map(|id| paths.get(&id.0).map_or("?", String::as_str).to_owned()).collect() };
    for (label, targets) in [("labelled_by", targets(data.labelled_by())), ("controls", targets(data.controls()))] {
        if !targets.is_empty() {
            put(label, format!("[{}]", targets.join(",")));
        }
    }
    let mut flags: Vec<String> = Vec::new();
    if data.is_hidden() {
        flags.push("hidden".into());
    }
    if data.is_disabled() {
        flags.push("disabled".into());
    }
    if data.is_read_only()
        && matches!(data.role(), Role::TextInput | Role::MultilineTextInput | Role::SearchInput | Role::PasswordInput | Role::NumberInput)
    {
        flags.push("read-only".into());
    }
    if data.is_required() {
        flags.push("required".into());
    }
    if let Some(toggled) = data.toggled() {
        flags.push(match toggled {
            Toggled::False => "unchecked".into(),
            Toggled::True => "checked".into(),
            Toggled::Mixed => "mixed".into(),
        });
    }
    if let Some(expanded) = data.is_expanded() {
        flags.push(if expanded { "expanded" } else { "collapsed" }.into());
    }
    if let Some(selected) = data.is_selected() {
        flags.push(if selected { "selected" } else { "unselected" }.into());
    }
    if let Some(popup) = data.has_popup() {
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
    Some(Line {
        depth,
        path: path.to_owned(),
        key,
        text,
        focused: tree.focused_key() == key,
        role: data.role(),
        name,
        hidden: data.is_hidden(),
        disabled: data.is_disabled(),
        actions,
    })
}
