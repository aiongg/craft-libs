//! The text form of a story's AccessKit tree, its args, and the difference between two trees.
//!
//! A tree line is `<indent><path> <Role> "<name>" [value=..] [flags] [actions=[..]]`. A path is
//! the child indices from the root (`0.2.1`), so reruns print the same ids. Bounds are left out.

use std::collections::HashMap;
use std::fmt::Write as _;

use craft_ui::story::Args;
use egui::accesskit::{Action, Role, Toggled};
use egui_kittest::kittest::NodeT as _;
use egui_kittest::{Harness, Node};

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

/// One node of a [`Tree`].
pub struct Line {
    pub depth: usize,
    pub path: String,
    /// Everything after the path.
    pub text: String,
    pub focused: bool,
    pub role: egui::accesskit::Role,
    pub name: String,
    pub hidden: bool,
    pub disabled: bool,
    pub actions: Vec<Action>,
}

impl Line {
    /// The node as one line of the dump.
    pub fn show(&self) -> String {
        format!("{}{} {}", "  ".repeat(self.depth), self.path, self.text)
    }

    /// A short name for a step's heading.
    pub fn title(&self) -> String {
        format!("{} {:?} {:?}", self.path, self.role, self.name)
    }
}

/// The nodes of the tree in document order.
pub struct Tree {
    pub lines: Vec<Line>,
}

impl Tree {
    pub fn text(&self) -> String {
        self.lines.iter().map(|l| l.show() + "\n").collect()
    }

    pub fn focused(&self) -> Option<&Line> {
        self.lines.iter().find(|l| l.focused)
    }

    pub fn line(&self, path: &str) -> Option<&Line> {
        self.lines.iter().find(|l| l.path == path)
    }

    /// The lines that are in `after` and not in `self`, as `+`, and the other way round, as `-`,
    /// in document order. Nodes are compared by depth and text, not path, so a node inserted
    /// above its siblings does not rewrite them.
    pub fn diff(&self, after: &Tree) -> Vec<String> {
        let key = |l: &Line| (l.depth, l.text.clone());
        let before: Vec<_> = self.lines.iter().map(key).collect();
        let after_keys: Vec<_> = after.lines.iter().map(key).collect();
        let prefix = before.iter().zip(&after_keys).take_while(|(a, b)| a == b).count();
        let suffix = before[prefix..].iter().rev().zip(after_keys[prefix..].iter().rev()).take_while(|(a, b)| a == b).count();
        let old = &before[prefix..before.len() - suffix];
        let new = &after_keys[prefix..after_keys.len() - suffix];
        let mut out = Vec::new();
        // Longest common subsequence of the differing middle; a middle too large to tabulate is
        // reported as replaced whole.
        if old.len().saturating_mul(new.len()) > 16_000_000 {
            out.extend(self.lines[prefix..before.len() - suffix].iter().map(|l| format!("- {}", l.show().trim_start())));
            out.extend(after.lines[prefix..after_keys.len() - suffix].iter().map(|l| format!("+ {}", l.show().trim_start())));
            return out;
        }
        let width = new.len() + 1;
        let mut table = vec![0u32; (old.len() + 1) * width];
        for i in (0..old.len()).rev() {
            for j in (0..new.len()).rev() {
                table[i * width + j] =
                    if old[i] == new[j] { table[(i + 1) * width + j + 1] + 1 } else { table[(i + 1) * width + j].max(table[i * width + j + 1]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < old.len() || j < new.len() {
            if i < old.len() && j < new.len() && old[i] == new[j] {
                i += 1;
                j += 1;
            } else if i < old.len() && (j == new.len() || table[(i + 1) * width + j] >= table[i * width + j + 1]) {
                out.push(format!("- {}", self.lines[prefix + i].show().trim_start()));
                i += 1;
            } else {
                out.push(format!("+ {}", after.lines[prefix + j].show().trim_start()));
                j += 1;
            }
        }
        out
    }
}

/// The harness's tree.
pub fn tree<S>(harness: &Harness<'_, S>) -> Tree {
    let mut paths = HashMap::new();
    paths_of(&harness.root(), "r", &mut paths);
    let mut lines = Vec::new();
    collect(&harness.root(), "r", 0, &paths, &mut lines);
    Tree { lines }
}

/// The node at `path` of a tree made by [`tree`].
pub fn node_at<'a, S>(harness: &'a Harness<'_, S>, path: &str) -> Option<Node<'a>> {
    let mut node = harness.root();
    for index in path.strip_prefix('r')?.split('.').filter(|s| !s.is_empty()) {
        node = node.children().nth(index.parse().ok()?)?;
    }
    Some(node)
}

fn child_path(parent: &str, index: usize) -> String {
    if parent == "r" { format!("r.{index}") } else { format!("{parent}.{index}") }
}

fn paths_of(node: &Node<'_>, path: &str, paths: &mut HashMap<String, String>) {
    paths.insert(format!("{:?}", node.accesskit_node().id()), path.to_owned());
    for (i, child) in node.children().enumerate() {
        paths_of(&child, &child_path(path, i), paths);
    }
}

fn collect(node: &Node<'_>, path: &str, depth: usize, paths: &HashMap<String, String>, lines: &mut Vec<Line>) {
    lines.push(line_of(node, path, depth, paths));
    for (i, child) in node.children().enumerate() {
        collect(&child, &child_path(path, i), depth + 1, paths, lines);
    }
}

fn line_of(node: &Node<'_>, path: &str, depth: usize, paths: &HashMap<String, String>) -> Line {
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

/// The story's args, one per line.
pub fn args(args: &Args) -> String {
    args.iter().map(|a| format!("  {} = {:?}\n", a.name, a.value)).collect()
}

/// The args that differ between two states of the same story, one per line.
pub fn args_diff(before: &Args, after: &Args) -> Vec<String> {
    before
        .iter()
        .zip(after.iter())
        .filter(|(b, a)| b.value != a.value)
        .map(|(b, a)| format!("arg {}: {:?} -> {:?}", a.name, b.value, a.value))
        .collect()
}
