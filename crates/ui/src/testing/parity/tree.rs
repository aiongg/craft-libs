//! The text form of a story's AccessKit tree, its args, and the difference between two trees. The format is the
//! one the phase-0 capture of the legacy library was written in.
//!
//! A tree line is `<indent><path> <Role> "<name>" [value=..] [flags] [actions=[..]]`. A path is
//! the child indices from the root (`0.2.1`), so reruns print the same ids. Bounds are left out.

use std::collections::HashMap;

use super::ParityState;
use crate::runtime::egui::access::parity::{child_path, line_of, paths_of};
use crate::runtime::egui::access::{Action, Role};
use egui_kittest::kittest::NodeT as _;
use egui_kittest::{Harness, Node};

/// One node of a [`Tree`].
#[non_exhaustive]
pub struct Line {
    pub depth: usize,
    pub path: String,
    /// Everything after the path.
    pub text: String,
    pub focused: bool,
    pub role: Role,
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
#[non_exhaustive]
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

fn collect(node: &Node<'_>, path: &str, depth: usize, paths: &HashMap<String, String>, lines: &mut Vec<Line>) {
    lines.push(line_of(node, path, depth, paths));
    for (i, child) in node.children().enumerate() {
        collect(&child, &child_path(path, i), depth + 1, paths, lines);
    }
}

/// The story's args, one per line.
pub fn args<S: ParityState>(state: &S) -> String {
    state.args().iter().map(|(name, value)| format!("  {name} = {value}\n")).collect()
}

/// The args that differ between two states of the same story, one per line.
pub fn args_diff<S: ParityState>(before: &S, after: &S) -> Vec<String> {
    before
        .args()
        .into_iter()
        .zip(after.args())
        .filter(|((_, b), (_, a))| b != a)
        .map(|((_, b), (name, a))| format!("arg {name}: {b} -> {a}"))
        .collect()
}
