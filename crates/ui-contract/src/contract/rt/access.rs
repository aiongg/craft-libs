//! What a component says to assistive technology, on craft-ui-rt (docs/contract-rt.md sections 5, 7
//! and 10).
//!
//! A component's `access` hook fills an [`Access`] in its run. The run turns it into one AccessKit
//! node and its sub-nodes and keeps them with the node's other outputs; ui-rt sends a node only
//! when it differs from what it sent, and builds none while the window's access tree is off. A
//! node out of view (outside a clip above it) is not in the tree; ui-rt applies that rule when it
//! walks. Actions arrive as queued input ([`AccessAction`]) on the node or sub-node they name.

use accesskit::{Action, ActionData, Node, Role as NodeRole};
use craft_behavior::PartRef;
use craft_theme::Flags;

use super::node::part_id;
pub use crate::runtime::access::{Access, GridPlace, Name, Numeric, Politeness, PopupKind, Role, SetPosition, SubNode};

/// What the run knows about a node beside its `Access`: the label a `Name::FromLabel` names it by,
/// the state flags (the only source of AccessKit states), and the actions its sense and popups give.
pub(crate) struct NodeFacts<'a> {
    pub(crate) label: &'a str,
    pub(crate) flags: Flags,
    pub(crate) enabled: bool,
    pub(crate) clickable: bool,
    pub(crate) focusable: bool,
    /// A context-menu opener ran: the node offers `ShowContextMenu`.
    pub(crate) context_menu: bool,
}

/// The node and sub-nodes a run keeps: `None` for no node (`Role::None`, or `Role::Merged` with
/// nothing of its own).
pub(crate) fn finish(mut access: Access, default: NodeRole, facts: &NodeFacts<'_>) -> Option<Built> {
    let role = match access.role {
        Some(Role::Node(role)) => role,
        Some(Role::None | Role::Merged) => return None,
        None => default,
    };
    let mut subs = Vec::with_capacity(access.nodes.len());
    for sub in std::mem::take(&mut access.nodes) {
        let role = match sub.role {
            Role::Node(role) => role,
            _ => NodeRole::GenericContainer,
        };
        let mut node = Node::new(role);
        name(&mut node, &sub.name, facts.label);
        if let Some(text) = &sub.description {
            node.set_description(text.as_str());
        }
        numeric(&mut node, sub.numeric.as_ref(), sub.steps);
        if let Some(text) = &sub.value {
            node.set_value(text.as_str());
        }
        if let Some(on) = sub.expanded {
            node.set_expanded(on);
        }
        if let Some(kind) = sub.has_popup {
            node.set_has_popup(has_popup(kind));
        }
        if let Some(p) = &sub.position {
            position(&mut node, p);
        }
        grid(&mut node, &sub.grid);
        if let Some(level) = sub.level {
            node.set_level(level);
        }
        if let Some(text) = &sub.shortcut {
            node.set_keyboard_shortcut(text.as_str());
        }
        if let Some(key) = sub.access_key {
            node.set_access_key(key.to_string());
        }
        if let Some(url) = &sub.url {
            node.set_url(url.as_str());
        }
        if sub.current {
            node.set_aria_current(accesskit::AriaCurrent::True);
        }
        states(&mut node, sub.flags);
        actions(&mut node, &sub.custom_actions);
        if let Some(b) = sub.bounds {
            node.set_bounds(accesskit::Rect::new(f64::from(b.min.x), f64::from(b.min.y), f64::from(b.max.x), f64::from(b.max.y)));
        }
        if !facts.enabled {
            node.set_disabled();
        }
        subs.push(BuiltSub { key: part_id(sub.part), part: sub.part, parent: sub.under.map(part_id), node });
    }
    let node = write(&access, role, facts);
    Some(Built { node, subs })
}

/// The AccessKit node of the component's own facts.
fn write(a: &Access, role: NodeRole, facts: &NodeFacts<'_>) -> Node {
    let mut node = Node::new(role);
    name(&mut node, &a.name, facts.label);
    let r = &a.relations;
    for id in &r.labelled_by {
        node.push_labelled_by(accesskit::NodeId(id.value()));
    }
    for id in &r.described_by {
        node.push_described_by(accesskit::NodeId(id.value()));
    }
    for id in &r.controls {
        node.push_controlled(accesskit::NodeId(id.value()));
    }
    for id in &r.flow_to {
        node.push_flow_to(accesskit::NodeId(id.value()));
    }
    if let Some(text) = &a.description {
        node.set_description(text.as_str());
    }
    if let Some(text) = &a.placeholder {
        node.set_placeholder(text.as_str());
    }
    match (&a.value, &a.unit) {
        (Some(value), Some(unit)) => node.set_value(format!("{value} {unit}")),
        (Some(value), None) => node.set_value(value.as_str()),
        _ => {}
    }
    numeric(&mut node, a.numeric.as_ref(), true);
    if let Some(on) = a.expanded {
        node.set_expanded(on);
    }
    if let Some(kind) = a.has_popup {
        node.set_has_popup(has_popup(kind));
    }
    if let Some(level) = a.level {
        node.set_level(level);
    }
    if let Some(p) = &a.position {
        position(&mut node, p);
    }
    grid(&mut node, &a.grid);
    if a.required {
        node.set_required();
    }
    if let Some(url) = &a.url {
        node.set_url(url.as_str());
    }
    if let Some(text) = a.shortcut.as_ref().filter(|_| !a.no_shortcut) {
        node.set_keyboard_shortcut(text.as_str());
    }
    if let Some(key) = a.access_key {
        node.set_access_key(key.to_string());
    }
    if let Some(politeness) = a.live {
        node.set_live(match politeness {
            Politeness::Polite => accesskit::Live::Polite,
            Politeness::Assertive => accesskit::Live::Assertive,
        });
    }
    if a.busy {
        node.set_busy();
    }
    states(&mut node, facts.flags);
    if !facts.enabled {
        node.set_disabled();
        return node;
    }
    if facts.clickable {
        node.add_action(Action::Click);
    }
    if facts.focusable {
        node.add_action(Action::Focus);
    }
    if facts.context_menu {
        node.add_action(Action::ShowContextMenu);
    }
    let (vertical, horizontal) = a.pans;
    if vertical {
        node.add_action(Action::ScrollUp);
        node.add_action(Action::ScrollDown);
    }
    if horizontal {
        node.add_action(Action::ScrollLeft);
        node.add_action(Action::ScrollRight);
    }
    actions(&mut node, &a.custom_actions);
    node
}

fn name(node: &mut Node, name: &Name, label: &str) {
    match name {
        Name::FromLabel if !label.is_empty() => node.set_label(label),
        Name::FromLabel => {}
        Name::Text(text) => node.set_label(text.as_str()),
        Name::Joined(texts) => node.set_label(texts.join(", ")),
        Name::LabelledBy(id) => node.push_labelled_by(accesskit::NodeId(id.value())),
        Name::Decorative => node.set_hidden(),
    }
}

/// A numeric value and, with a step and `steps`, the Increment and Decrement it can take.
fn numeric(node: &mut Node, n: Option<&Numeric>, steps: bool) {
    let Some(n) = n else { return };
    if let Some(value) = n.value {
        node.set_numeric_value(value);
    }
    node.set_min_numeric_value(*n.range.start());
    node.set_max_numeric_value(*n.range.end());
    if let Some(text) = &n.text {
        node.set_value(text.as_str());
    }
    node.add_action(Action::SetValue);
    let Some(step) = n.step.filter(|_| steps) else { return };
    node.set_numeric_value_step(step);
    match n.value {
        Some(value) => {
            if n.wraps || value < *n.range.end() {
                node.add_action(Action::Increment);
            }
            if n.wraps || value > *n.range.start() {
                node.add_action(Action::Decrement);
            }
        }
        None => {
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
        }
    }
}

fn has_popup(kind: PopupKind) -> accesskit::HasPopup {
    match kind {
        PopupKind::Menu => accesskit::HasPopup::Menu,
        PopupKind::Listbox => accesskit::HasPopup::Listbox,
        PopupKind::Tree => accesskit::HasPopup::Tree,
        PopupKind::Grid => accesskit::HasPopup::Grid,
        PopupKind::Dialog => accesskit::HasPopup::Dialog,
    }
}

fn position(node: &mut Node, p: &SetPosition) {
    node.set_position_in_set(p.index);
    if let Some(size) = p.size {
        node.set_size_of_set(size);
    }
    if let Some(level) = p.level {
        node.set_level(level);
    }
}

fn grid(node: &mut Node, g: &GridPlace) {
    if let Some(v) = g.row_index {
        node.set_row_index(v);
    }
    if let Some(v) = g.row_count {
        node.set_row_count(v);
    }
    if let Some(v) = g.column_index {
        node.set_column_index(v);
    }
    if let Some(v) = g.column_count {
        node.set_column_count(v);
    }
}

/// The AccessKit states of the state flags.
fn states(node: &mut Node, flags: Flags) {
    if flags.contains(Flags::SELECTED) {
        node.set_selected(true);
    }
    if flags.contains(Flags::MIXED) {
        node.set_toggled(accesskit::Toggled::Mixed);
    } else if flags.contains(Flags::CHECKED) {
        node.set_toggled(accesskit::Toggled::True);
    }
    if flags.contains(Flags::EXPANDED) || flags.contains(Flags::OPEN) {
        node.set_expanded(true);
    }
    if flags.contains(Flags::INVALID) {
        node.set_invalid(accesskit::Invalid::True);
    }
    if flags.contains(Flags::BUSY) {
        node.set_busy();
    }
    if flags.contains(Flags::READ_ONLY) {
        node.set_read_only();
    }
}

fn actions(node: &mut Node, custom: &[(i32, crate::runtime::env::Label)]) {
    if custom.is_empty() {
        return;
    }
    node.add_action(Action::CustomAction);
    for (id, label) in custom {
        node.push_custom_action(accesskit::CustomAction { id: *id, description: label.as_str().into() });
    }
}

/// A sub-node as a run keeps it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BuiltSub {
    pub(crate) key: u64,
    /// The part the sub-node is for: an action on it reaches the run as a request on that part.
    pub(crate) part: PartRef,
    pub(crate) parent: Option<u64>,
    pub(crate) node: Node,
}

/// What a run keeps of its access facts.
pub(crate) struct Built {
    pub(crate) node: Node,
    pub(crate) subs: Vec<BuiltSub>,
}

/// An assistive technology's action on a node, as a component sees it in [`crate::contract::rt::Cx::input`].
/// Click arrives the same way (`Cx::clicked`); Focus is served by the runtime and never queued.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessAction {
    /// The action.
    pub action: Action,
    /// Its payload: the value of SetValue, the id of a custom action, the unit of a scroll.
    pub data: Option<ActionData>,
    /// The sub-node it names (the key's hash), `None` for the component's own node.
    pub sub: Option<u64>,
}

impl AccessAction {
    /// The numeric value of a SetValue, whether the platform sent a number or a text.
    pub fn numeric_value(&self) -> Option<f64> {
        match (&self.action, &self.data) {
            (Action::SetValue, Some(ActionData::NumericValue(v))) => Some(*v),
            (Action::SetValue, Some(ActionData::Value(text))) => text.trim().parse().ok(),
            _ => None,
        }
    }
}

impl super::Ui<'_, '_> {
    /// Announces `text` to assistive technology in this window: the texts of one frame become one
    /// live status node. The same text made twice in a frame is read once; nothing is kept while
    /// the window's access tree is off.
    pub fn announce(&mut self, text: impl Into<String>, politeness: Politeness) {
        self.cx.announce(
            text,
            match politeness {
                Politeness::Polite => craft_ui_rt::AnnouncePoliteness::Polite,
                Politeness::Assertive => craft_ui_rt::AnnouncePoliteness::Assertive,
            },
        );
    }
}
