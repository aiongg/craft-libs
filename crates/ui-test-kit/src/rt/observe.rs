//! What a screen-reader user and the app can tell about a run: per window, the access nodes
//! (role, name, value, state, bounds) and the focused one, plus the app's state as text. Two runs
//! of one scenario are the same when their snapshots are, step by step.

use craft_ui_platform_headless::HeadlessPlatform;
use craft_ui_rt::{Runtime, WindowId};

/// Roles that carry no meaning of their own; unnamed ones are left out of snapshots.
const GENERIC: &[accesskit::Role] =
    &[accesskit::Role::GenericContainer, accesskit::Role::Group, accesskit::Role::Unknown, accesskit::Role::Pane, accesskit::Role::Window];

/// One access node, without its id.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessNode {
    /// The role's name (`ListItem`, `Button`).
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub toggled: Option<bool>,
    pub selected: Option<bool>,
    pub focused: bool,
    /// `[min_x, min_y, max_x, max_y]` in logical points, window coordinates.
    pub bounds: [f32; 4],
}

impl AccessNode {
    /// The middle of the bounds.
    pub fn center(&self) -> craft_ui_rt::Pos2 {
        craft_ui_rt::pos2((self.bounds[0] + self.bounds[2]) / 2.0, (self.bounds[1] + self.bounds[3]) / 2.0)
    }

    fn rounded(&self) -> [i32; 4] {
        self.bounds.map(|v| v.round() as i32)
    }

    fn same(&self, other: &AccessNode) -> bool {
        self.role == other.role
            && self.name == other.name
            && self.value == other.value
            && self.description == other.description
            && self.toggled == other.toggled
            && self.selected == other.selected
            && self.focused == other.focused
            && self.rounded() == other.rounded()
    }

    fn describe(&self) -> String {
        format!(
            "{} {:?} value={:?} desc={:?} toggled={:?} selected={:?} focused={} bounds={:?}",
            self.role,
            self.name,
            self.value,
            self.description,
            self.toggled,
            self.selected,
            self.focused,
            self.rounded()
        )
    }
}

/// One window's observable state.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowSnapshot {
    pub window: WindowId,
    /// The name of the access node with focus.
    pub focus: Option<String>,
    /// In tree order.
    pub access: Vec<AccessNode>,
}

impl WindowSnapshot {
    /// The `n`-th node with this role and name.
    pub fn find(&self, role: &str, name: &str) -> Option<&AccessNode> {
        self.access.iter().find(|n| n.role == role && n.name == name)
    }

    pub fn with_role<'a>(&'a self, role: &'a str) -> impl Iterator<Item = &'a AccessNode> {
        self.access.iter().filter(move |n| n.role == role)
    }
}

/// Every window's access tree and the app state, after one step.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub windows: Vec<WindowSnapshot>,
    /// The app's state as text (see `Rig::with_state`); empty when the rig has none.
    pub state: String,
}

impl Snapshot {
    pub(crate) fn of(rt: &Runtime, platform: &HeadlessPlatform, state: Option<&(dyn Fn(&Runtime) -> String + Send + Sync)>) -> Snapshot {
        let windows = rt.windows().map(|window| window_snapshot(rt, platform, window)).collect();
        Snapshot { windows, state: state.map_or_else(String::new, |f| f(rt)) }
    }

    pub fn window(&self, window: WindowId) -> Option<&WindowSnapshot> {
        self.windows.iter().find(|w| w.window == window)
    }

    /// The first difference between two snapshots, or `None`. Bounds compare rounded to 1 px.
    pub fn diff(&self, other: &Snapshot) -> Option<String> {
        if self.state != other.state {
            return Some(format!("app state:\n  a: {}\n  b: {}", self.state, other.state));
        }
        if self.windows.len() != other.windows.len() {
            return Some(format!("window count: a={} b={}", self.windows.len(), other.windows.len()));
        }
        for (a, b) in self.windows.iter().zip(&other.windows) {
            if a.window != b.window {
                return Some(format!("window ids: a={:?} b={:?}", a.window, b.window));
            }
            if a.focus != b.focus {
                return Some(format!("{:?} focus: a={:?} b={:?}", a.window, a.focus, b.focus));
            }
            for (i, (x, y)) in a.access.iter().zip(&b.access).enumerate() {
                if !x.same(y) {
                    return Some(format!("{:?} access node {i}:\n  a: {}\n  b: {}", a.window, x.describe(), y.describe()));
                }
            }
            if a.access.len() != b.access.len() {
                let (longer, tag) = if a.access.len() > b.access.len() { (a, "a") } else { (b, "b") };
                let extra = &longer.access[a.access.len().min(b.access.len())];
                return Some(format!(
                    "{:?} access node count: a={} b={}; first extra in {tag}: {}",
                    a.window,
                    a.access.len(),
                    b.access.len(),
                    extra.describe()
                ));
            }
        }
        None
    }
}

fn window_snapshot(rt: &Runtime, platform: &HeadlessPlatform, window: WindowId) -> WindowSnapshot {
    let focus_id = platform.updates_for(window).last().map(|u| u.focus);
    let mut focus = None;
    let mut access = Vec::new();
    for entry in rt.access_snapshot(window) {
        let node = entry.node;
        let focused = Some(entry.id) == focus_id;
        let name = node.label().unwrap_or_default().to_owned();
        if focused {
            focus = Some(name.clone());
        }
        if GENERIC.contains(&node.role()) && name.is_empty() {
            continue;
        }
        access.push(AccessNode {
            role: format!("{:?}", node.role()),
            name,
            value: node.value().map(str::to_owned),
            description: node.description().map(str::to_owned),
            toggled: node.toggled().map(|t| t != accesskit::Toggled::False),
            selected: node.is_selected(),
            focused,
            bounds: [entry.bounds.min.x, entry.bounds.min.y, entry.bounds.max.x, entry.bounds.max.y],
        });
    }
    WindowSnapshot { window, focus, access }
}

/// Where two runs of a scenario first differ.
#[derive(Debug)]
pub struct StepDiff {
    pub step: usize,
    pub label: String,
    pub detail: String,
}

/// Compares two runs step by step; the first differing step wins. A run that has fewer steps than
/// the other differs at the first missing step.
pub fn first_difference(labels: &[String], a: &[Snapshot], b: &[Snapshot]) -> Option<StepDiff> {
    let label = |i: usize| labels.get(i).cloned().unwrap_or_default();
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if let Some(detail) = x.diff(y) {
            return Some(StepDiff { step: i, label: label(i), detail });
        }
    }
    (a.len() != b.len()).then(|| {
        let i = a.len().min(b.len());
        StepDiff { step: i, label: label(i), detail: format!("step count: a={} b={}", a.len(), b.len()) }
    })
}
