//! The accessibility inspector: the AccessKit tree of a frame as an outline, each node with its role, name and the
//! actions it declares, and a flag on interactive nodes that have no name.

use craft_behavior::{DisplayOrder, IconId, ItemSource, Selection};
use craft_theme::variant::Tone;
use craft_ui::contract::{Bind, CollectionEvent, Pattern, Props as _, Role, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::Stack;
use craft_ui::list::{List, ListRow};
use craft_ui::primitives::IconSpec;
use craft_ui::runtime::memory::Remember as _;
use craft_ui::style::TypeRole;
use craft_ui::testing::access::{AccessTree, Declared, NodeKey, is_interactive, name};
use egui::{Response, Ui};
use std::borrow::Cow;
use std::collections::HashSet;

/// The actions listed on a node, in this order.
const ACTIONS: &[(Declared, &str)] = &[
    (Declared::Click, "click"),
    (Declared::Focus, "focus"),
    (Declared::SetValue, "set value"),
    (Declared::Increment, "increment"),
    (Declared::Decrement, "decrement"),
    (Declared::Expand, "expand"),
    (Declared::Collapse, "collapse"),
];

/// What the inspector keeps between passes: the nodes folded shut and the selected node. A tree changes from frame to
/// frame, so the folded nodes are kept, not the open ones.
#[derive(Clone, Default)]
struct Session {
    collapsed: HashSet<NodeKey>,
    selection: Selection<NodeKey>,
}

/// One node of the outline.
struct Node {
    key: NodeKey,
    depth: usize,
    has_children: bool,
    label: String,
    actions: String,
    unnamed: bool,
    text: String,
}

/// The nodes of `tree` that assistive technology reports, depth first from the roots.
fn nodes(tree: &AccessTree) -> Vec<Node> {
    let keys = tree.keys();
    let reported: HashSet<NodeKey> = keys.iter().copied().collect();
    let children: HashSet<NodeKey> = keys.iter().flat_map(|key| tree.children_of(*key)).collect();
    let roots: Vec<NodeKey> = keys.iter().copied().filter(|key| !children.contains(key)).collect();
    let mut found = Vec::new();
    let mut pending: Vec<(NodeKey, usize)> = roots.into_iter().rev().map(|key| (key, 0)).collect();
    while let Some((key, depth)) = pending.pop() {
        let children: Vec<NodeKey> = tree.children_of(key).into_iter().filter(|child| reported.contains(child)).collect();
        let unnamed = is_interactive(tree, key) && name(tree, key).is_none();
        let role = match tree.role_of(key) {
            Some(Role::Node(role)) => format!("{role:?}"),
            _ => String::new(),
        };
        let given = name(tree, key).map(str::to_owned);
        let label = match &given {
            Some(given) => format!("{role} \"{given}\""),
            None => role,
        };
        let actions: Vec<&str> = ACTIONS.iter().filter(|(action, _)| tree.declares_key(key, *action)).map(|(_, word)| *word).collect();
        found.push(Node {
            key,
            depth,
            has_children: !children.is_empty(),
            text: given.unwrap_or_default(),
            label,
            actions: actions.join(", "),
            unnamed,
        });
        pending.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
    }
    found
}

/// The outline's visible rows: the nodes not under a folded one.
struct Outline<'a> {
    nodes: &'a [Node],
    shown: Vec<usize>,
    collapsed: &'a HashSet<NodeKey>,
}

impl<'a> Outline<'a> {
    fn of(nodes: &'a [Node], collapsed: &'a HashSet<NodeKey>) -> Self {
        let mut shown = Vec::new();
        let mut folded_at: Option<usize> = None;
        for (at, node) in nodes.iter().enumerate() {
            if folded_at.is_some_and(|depth| node.depth > depth) {
                continue;
            }
            folded_at = collapsed.contains(&node.key).then_some(node.depth);
            shown.push(at);
        }
        Outline { nodes, shown, collapsed }
    }

    fn node(&self, key: &NodeKey) -> Option<&Node> {
        self.nodes.iter().find(|node| node.key == *key)
    }
}

impl DisplayOrder<NodeKey> for Outline<'_> {
    fn len(&self) -> usize {
        self.shown.len()
    }

    fn key(&self, at: usize) -> NodeKey {
        self.nodes[self.shown[at]].key
    }

    fn depth(&self, at: usize) -> usize {
        self.nodes[self.shown[at]].depth
    }

    fn has_children(&self, at: usize) -> bool {
        self.nodes[self.shown[at]].has_children
    }

    fn open(&self, at: usize) -> bool {
        !self.collapsed.contains(&self.nodes[self.shown[at]].key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.nodes[self.shown[at]].text.as_str()))
    }
}

impl ItemSource<NodeKey> for Outline<'_> {}

/// The AccessKit tree of one frame as a foldable outline.
#[derive(Pattern)]
pub struct AccessInspector<'a> {
    tree: &'a AccessTree,
}

impl<'a> AccessInspector<'a> {
    /// An inspector over `tree`, the tree of a finished frame.
    pub fn new(tree: &'a AccessTree) -> Self {
        AccessInspector { tree }
    }

    /// Shows the outline across the room offered.
    pub fn show(self, ui: &mut Ui) -> Response {
        let all = nodes(self.tree);
        let interactive = all.iter().filter(|node| is_interactive(self.tree, node.key)).count();
        let unnamed = all.iter().filter(|node| node.unnamed).count();
        let overview = format!("{} nodes, {interactive} interactive, {unnamed} without a name", all.len());
        Stack::new()
            .fill()
            .header(|ui, _| {
                Text::new(overview.clone()).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
            })
            .show(ui, |ui, cx| {
                let mut session = cx.memory::<Session>().clone();
                let outline = Outline::of(&all, &session.collapsed);
                let events = List::<NodeKey>::new("Accessibility tree")
                    .select(SelectMode::Single)
                    .tree(true)
                    .empty("The frame made no accessibility tree")
                    .selection(Bind::bound(&mut session.selection))
                    .show_items(ui, &outline, &mut |key, _| {
                        let Some(node) = outline.node(key) else { return ListRow::spacer() };
                        let mut row = ListRow::new(node.label.clone()).depth(node.depth);
                        if node.has_children {
                            row = row.group(!outline.collapsed.contains(key));
                        }
                        if node.unnamed {
                            row = row.icon(IconSpec::svg(IconId::new("warning"))).tone(Tone::Warn).trailing("no name");
                        } else if !node.actions.is_empty() {
                            row = row.trailing(node.actions.clone());
                        }
                        row
                    })
                    .events;
                for event in events {
                    if let CollectionEvent::Open(change) = event {
                        let groups = all.iter().filter(|node| node.has_children).map(|node| node.key);
                        let keys: Vec<NodeKey> = if change.all { groups.collect() } else { vec![change.key] };
                        for key in keys {
                            if change.open {
                                session.collapsed.remove(&key);
                            } else {
                                session.collapsed.insert(key);
                            }
                        }
                    }
                }
                *cx.memory::<Session>() = session;
            })
            .output
    }
}
