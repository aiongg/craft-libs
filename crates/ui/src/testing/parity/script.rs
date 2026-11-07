//! The generic scripted interaction recorded after a story's tree at rest.
//!
//! Output sections, in order:
//! - `tab`: Tab presses until focus returns to a node already visited, one `focus` line each.
//! - `drive <path> <Role> "<name>"`: for each actionable node in tree order, its steps (`click`,
//!   `Space`, `Enter`, arrows, `Escape`). A step prints `<step>: no change` or the changes:
//!   `focus`, `arg`, and tree lines as `+`/`-`.

use std::fmt::Write as _;

use crate::runtime::egui::access::{Action, Role};
use crate::runtime::egui::fixture::AccessActionKind;
use egui::{Key, Modifiers};

use super::tree::{self as dump, Line, Tree};
use super::{ParityState, Scene};

/// Frames run after each input.
const FRAMES: usize = 8;

/// Tab presses before the walk gives up on a focus order that does not cycle.
const MAX_TABS: usize = 80;

/// Nodes driven in one story.
const MAX_TARGETS: usize = 64;

/// Roles whose value arrow keys, Home and End change.
const VALUE_ROLES: [Role; 5] = [Role::Slider, Role::SpinButton, Role::ScrollBar, Role::Splitter, Role::ProgressIndicator];

/// Roles that pick among siblings with the arrow keys.
const SELECTION_ROLES: [Role; 17] = [
    Role::Tab,
    Role::TabList,
    Role::RadioButton,
    Role::RadioGroup,
    Role::ListBox,
    Role::ListBoxOption,
    Role::ListItem,
    Role::Menu,
    Role::MenuBar,
    Role::MenuItem,
    Role::MenuItemCheckBox,
    Role::MenuItemRadio,
    Role::ComboBox,
    Role::Tree,
    Role::TreeItem,
    Role::Row,
    Role::Cell,
];

/// Runs frames until animations at the story's first frame have advanced the same amount on
/// every run.
pub fn settle<S: ParityState>(scene: &mut Scene<S>) {
    scene.fixture.idle(FRAMES);
}

struct Recorder<'a, S> {
    scene: &'a mut Scene<S>,
    out: &'a mut String,
    tree: Tree,
    args: S,
}

impl<S: ParityState> Recorder<'_, S> {
    /// Runs the frames of a step and prints what changed since the last step.
    fn report(&mut self, step: &str) {
        settle(self.scene);
        let tree = self.scene.tree();
        let mut changes = Vec::new();
        let focus = |t: &Tree| t.focused().map(Line::title);
        if focus(&self.tree) != focus(&tree) {
            changes.push(format!(
                "focus: {} -> {}",
                focus(&self.tree).unwrap_or_else(|| "none".into()),
                focus(&tree).unwrap_or_else(|| "none".into())
            ));
        }
        changes.extend(dump::args_diff(&self.args, &self.scene.state()));
        changes.extend(self.tree.diff(&tree));
        if changes.is_empty() {
            let _ = writeln!(self.out, "  {step}: no change");
        } else {
            let _ = writeln!(self.out, "  {step}:");
            for change in changes {
                let _ = writeln!(self.out, "    {change}");
            }
        }
        self.tree = tree;
        self.args = self.scene.state();
    }

    /// The path of the node `target` now has, when it is still in the tree: the same path with
    /// the same role and name, else the first node with that role and name.
    fn find(&self, target: &Line) -> Option<String> {
        let same = |l: &Line| l.role == target.role && l.name == target.name;
        self.tree.line(&target.path).filter(|l| same(l)).or_else(|| self.tree.lines.iter().find(|l| same(l))).map(|l| l.path.clone())
    }

    fn click(&mut self, target: &Line) {
        let Some(path) = self.find(target) else { return self.gone("click") };
        let Some(at) = self.tree.line(&path).and_then(|line| self.scene.fixture.target(line.key)).filter(|at| at.rect.is_finite()) else {
            return self.unreachable("click");
        };
        self.scene.fixture.click(&at);
        self.report("click");
    }

    /// Puts focus on the node, then presses `key`.
    fn press(&mut self, target: &Line, key: Key) {
        let Some(path) = self.find(target) else { return self.gone(&format!("{key:?}")) };
        let focused = self.tree.line(&path).is_some_and(|l| l.focused);
        if !focused {
            if let Some(at) = self.tree.line(&path).and_then(|line| self.scene.fixture.target(line.key)) {
                self.scene.fixture.access(AccessActionKind::Focus, &at);
            }
            settle(self.scene);
            self.tree = self.scene.tree();
        }
        self.scene.fixture.key(key, Modifiers::NONE);
        self.report(&format!("{key:?}"));
    }

    fn unreachable(&mut self, step: &str) {
        let _ = writeln!(self.out, "  {step}: node has no finite bounds");
    }

    fn gone(&mut self, step: &str) {
        let _ = writeln!(self.out, "  {step}: node gone");
    }
}

/// Records the Tab walk and the per-node steps onto `out`.
pub fn run<S: ParityState>(scene: &mut Scene<S>, out: &mut String) {
    let tree = scene.tree();
    let args = scene.state();
    let mut rec = Recorder { scene, out, tree, args };
    rec.tab_walk();
    rec.drive();
}

impl<S: ParityState> Recorder<'_, S> {
    fn tab_walk(&mut self) {
        let _ = writeln!(self.out, "tab");
        let _ = writeln!(self.out, "  start: {}", self.tree.focused().map_or_else(|| "none".into(), Line::title));
        let mut seen: Vec<String> = Vec::new();
        for n in 1..=MAX_TABS {
            self.scene.fixture.key(Key::Tab, Modifiers::NONE);
            settle(self.scene);
            let tree = self.scene.tree();
            let now = tree.focused().map(|l| (l.path.clone(), l.title()));
            let _ = writeln!(self.out, "  {n}: {}", now.as_ref().map_or("none", |(_, t)| t));
            let changes = self.tree.diff(&tree);
            for change in &changes {
                let _ = writeln!(self.out, "    {change}");
            }
            self.tree = tree;
            self.args = self.scene.state();
            match now {
                Some((path, _)) if seen.contains(&path) => {
                    let _ = writeln!(self.out, "  cycle after {} nodes", seen.len());
                    return;
                }
                Some((path, _)) => seen.push(path),
                None if n > 1 => return,
                None => {}
            }
        }
        let _ = writeln!(self.out, "  no cycle in {MAX_TABS} presses");
    }

    fn drive(&mut self) {
        let targets: Vec<Line> = self
            .tree
            .lines
            .iter()
            .filter(|l| !l.hidden && !l.disabled && l.actions.iter().any(|a| *a != Action::ScrollIntoView))
            .filter(|l| {
                l.actions.iter().any(|a| {
                    matches!(
                        a,
                        Action::Click | Action::Focus | Action::Increment | Action::Decrement | Action::SetValue | Action::Expand | Action::Collapse
                    )
                })
            })
            .map(|l| Line {
                depth: l.depth,
                path: l.path.clone(),
                key: l.key,
                text: l.text.clone(),
                focused: false,
                role: l.role,
                name: l.name.clone(),
                hidden: false,
                disabled: false,
                actions: l.actions.clone(),
            })
            .collect();
        let _ = writeln!(self.out, "drive: {} actionable nodes", targets.len());
        if targets.len() > MAX_TARGETS {
            let _ = writeln!(self.out, "  the first {MAX_TARGETS} are driven");
        }
        for target in targets.iter().take(MAX_TARGETS) {
            let _ = writeln!(self.out, "drive {}", target.title());
            if self.find(target).is_none() {
                self.gone("all steps");
                continue;
            }
            let clickable = target.actions.contains(&Action::Click);
            let focusable = target.actions.contains(&Action::Focus);
            if clickable {
                self.click(target);
            }
            if focusable {
                self.press(target, Key::Space);
                self.press(target, Key::Enter);
            }
            if VALUE_ROLES.contains(&target.role) {
                for key in [Key::ArrowLeft, Key::ArrowRight, Key::ArrowUp, Key::ArrowDown, Key::Home, Key::End] {
                    self.press(target, key);
                }
            } else if SELECTION_ROLES.contains(&target.role) {
                for key in [Key::ArrowDown, Key::ArrowUp, Key::ArrowLeft, Key::ArrowRight] {
                    self.press(target, key);
                }
            }
            self.press(target, Key::Escape);
        }
    }
}
