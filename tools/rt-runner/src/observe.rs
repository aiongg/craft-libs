//! What a run shows: the document, the focus, the list's scroll and the access tree in the
//! experiment's vocabulary, so a run compares with the v1 reference files step by step.

use accesskit::{Role, Toggled};
use craft_ui_platform_headless::HeadlessPlatform;
use craft_ui_rt::{AccessEntry, Renderer, Runtime, WindowId};
use craft_ui_test_kit::rt::Rig;
use serde::{Deserialize, Serialize};

use craft_rt_layers::DocCell;
use craft_rt_layers::DocSnapshot;

/// One access node as the vocabulary names it. Bounds are logical points in window coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessNode {
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub toggled: Option<bool>,
    pub selected: Option<bool>,
    pub focused: bool,
    /// `[min_x, min_y, max_x, max_y]`
    pub bounds: [f32; 4],
    /// A text input's selection as character offsets into its value.
    #[serde(default)]
    pub selection: Option<[usize; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub doc: DocSnapshot,
    pub focus: Option<String>,
    /// The list's scroll offset in whole points.
    pub scroll: i32,
    pub access: Vec<AccessNode>,
}

/// The experiment's shared role vocabulary, from a role's `Debug` name.
pub fn role_name(role: &str) -> String {
    match role {
        "List" | "ListBox" | "Tree" => "list",
        "ListItem" | "ListBoxOption" | "TreeItem" | "Row" => "list_item",
        "Button" => "button",
        "CheckBox" | "Switch" | "RadioButton" => "toggle",
        "Label" | "TextRun" => "label",
        "TextInput" | "MultilineTextInput" | "SearchInput" => "text_input",
        "GenericContainer" | "Unknown" => "generic",
        "Window" => "window",
        other => return other.to_lowercase(),
    }
    .to_owned()
}

const GENERIC_ROLES: &[&str] = &["", "generic", "container", "group", "unknown", "pane", "window"];

/// The text selection as character offsets into the field's value. The snapshot keeps each node as
/// its element wrote it, so the field's text runs are its sub-nodes.
fn selection(entry: &AccessEntry<'_>, entries: &[AccessEntry<'_>]) -> Option<[usize; 2]> {
    let selection = entry.node.text_selection()?;
    let runs = || entries.iter().filter(|e| e.owner == entry.owner && e.sub.is_some()).map(|e| (e.id, e.node.character_lengths().len()));
    let offset = |at: &accesskit::TextPosition| text_offset(runs(), &at.node, at.character_index);
    Some([offset(&selection.anchor)?, offset(&selection.focus)?])
}

/// A text position as a character offset: every run before the named one adds its length.
fn text_offset<I: PartialEq>(runs: impl IntoIterator<Item = (I, usize)>, node: &I, character: usize) -> Option<usize> {
    let mut before = 0;
    for (id, count) in runs {
        if id == *node {
            return Some(before + character);
        }
        before += count;
    }
    None
}

/// Reads `window`'s access tree, the focus, the list's scroll and the document.
pub fn observe(runtime: &Runtime, window: WindowId, doc: DocCell) -> Observation {
    let focus_id = runtime.access_focus(window);
    let mut scroll = 0.0;
    let entries = runtime.access_snapshot(window);
    let mut nodes = Vec::new();
    for e in &entries {
        if e.node.role() == Role::List {
            scroll = e.node.scroll_y().unwrap_or(0.0);
        }
        let role = role_name(&format!("{:?}", e.node.role()));
        let name = e.node.label().unwrap_or_default().to_owned();
        if GENERIC_ROLES.contains(&role.as_str()) && name.is_empty() {
            continue;
        }
        nodes.push(AccessNode {
            role,
            name,
            value: e.node.value().map(str::to_owned),
            description: e.node.description().map(str::to_owned),
            toggled: e.node.toggled().and_then(|t| match t {
                Toggled::True => Some(true),
                Toggled::False => Some(false),
                Toggled::Mixed => None,
            }),
            selected: e.node.is_selected(),
            focused: e.id == focus_id,
            bounds: [e.bounds.min.x, e.bounds.min.y, e.bounds.max.x, e.bounds.max.y],
            selection: selection(e, &entries),
        });
    }
    let focus = nodes.iter().find(|n| n.focused).map(|n| n.name.clone()).filter(|n| !n.is_empty());
    Observation { doc: runtime.store().peek(doc).snapshot(), focus, scroll: scroll.round() as i32, access: nodes }
}

/// [`observe`] on a rig's first window.
pub fn observe_rig<R: Renderer>(rig: &Rig<R>, doc: DocCell) -> Observation {
    observe(&rig.driver.runtime, HeadlessPlatform::MAIN, doc)
}

fn round_bounds(b: [f32; 4]) -> [i32; 4] {
    b.map(|v| v.round() as i32)
}

fn describe(n: &AccessNode) -> String {
    format!(
        "{} {:?} value={:?} desc={:?} toggled={:?} selected={:?} focused={} bounds={:?} text selection={:?}",
        n.role,
        n.name,
        n.value,
        n.description,
        n.toggled,
        n.selected,
        n.focused,
        round_bounds(n.bounds),
        n.selection
    )
}

fn same_node(a: &AccessNode, b: &AccessNode) -> bool {
    a.role == b.role
        && a.name == b.name
        && a.value == b.value
        && a.description == b.description
        && a.toggled == b.toggled
        && a.selected == b.selected
        && a.focused == b.focused
        && round_bounds(a.bounds) == round_bounds(b.bounds)
        // The v1 reference (egui) publishes no text selection: a node of `a` without one has none to compare.
        && (a.selection.is_none() || a.selection == b.selection)
}

/// The access nodes as compared: a text field is one node (its role, name, value, selection, focus
/// and bounds); the text runs and the unnamed inner input inside it fold into it.
pub fn compared(nodes: &[AccessNode]) -> Vec<AccessNode> {
    let inside = |inner: &AccessNode, outer: &AccessNode| {
        const SLACK: f32 = 0.5;
        inner.bounds[0] >= outer.bounds[0] - SLACK
            && inner.bounds[1] >= outer.bounds[1] - SLACK
            && inner.bounds[2] <= outer.bounds[2] + SLACK
            && inner.bounds[3] <= outer.bounds[3] + SLACK
    };
    let mut out: Vec<AccessNode> = Vec::with_capacity(nodes.len());
    let mut i = 0;
    while i < nodes.len() {
        let mut field = nodes[i].clone();
        i += 1;
        if field.role != "text_input" || field.name.is_empty() {
            out.push(field);
            continue;
        }
        while let Some(part) = nodes.get(i) {
            let folds = part.role == "label" || (part.role == "text_input" && part.name.is_empty());
            if !folds || !inside(part, &field) {
                break;
            }
            if field.value.is_none() {
                field.value = part.value.clone();
            }
            if field.selection.is_none() {
                field.selection = part.selection;
            }
            field.focused |= part.focused;
            i += 1;
        }
        out.push(field);
    }
    out
}

/// The first difference between two observations, or `None`. Bounds compare rounded to 1 px.
pub fn diff(a: &Observation, b: &Observation) -> Option<String> {
    if a.doc != b.doc {
        return Some(format!("fixture state:\n  a: {:?}\n  b: {:?}", a.doc, b.doc));
    }
    if a.focus != b.focus {
        return Some(format!("focus: a={:?} b={:?}", a.focus, b.focus));
    }
    if a.scroll != b.scroll {
        return Some(format!("scroll: a={} b={}", a.scroll, b.scroll));
    }
    let (a_access, b_access) = (compared(&a.access), compared(&b.access));
    for (i, (x, y)) in a_access.iter().zip(&b_access).enumerate() {
        if !same_node(x, y) {
            return Some(format!("access node {i}:\n  a: {}\n  b: {}", describe(x), describe(y)));
        }
    }
    if a_access.len() != b_access.len() {
        let (longer, tag) = if a_access.len() > b_access.len() { (&a_access, "a") } else { (&b_access, "b") };
        let extra = &longer[a_access.len().min(b_access.len())];
        return Some(format!("access node count: a={} b={}; first extra in {tag}: {}", a_access.len(), b_access.len(), describe(extra)));
    }
    None
}

/// Largest bounds difference over nodes matched by (role, name), the k-th with the k-th, in points;
/// `None` when a node is missing.
pub fn max_bounds_delta(a: &Observation, b: &Observation) -> Option<f32> {
    let (a, b) = (compared(&a.access), compared(&b.access));
    let mut worst = 0.0f32;
    for (i, n) in a.iter().enumerate() {
        let same = |m: &&AccessNode| m.role == n.role && m.name == n.name;
        let k = a[..i].iter().filter(same).count();
        let m = b.iter().filter(same).nth(k)?;
        for j in 0..4 {
            worst = worst.max((n.bounds[j] - m.bounds[j]).abs());
        }
    }
    (a.len() == b.len()).then_some(worst)
}
