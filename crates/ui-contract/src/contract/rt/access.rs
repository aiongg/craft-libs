//! What a component says to assistive technology, on craft-ui-rt (docs/contract-rt.md sections 5, 7
//! and 10).
//!
//! A component's `access` hook fills an [`Access`] in its run. The run turns it into one AccessKit
//! node and its sub-nodes and keeps them with the node's other outputs; ui-rt sends a node only
//! when it differs from what it sent, and builds none while the window's access tree is off. A
//! node out of view (outside a clip above it) is not in the tree; ui-rt applies that rule when it
//! walks. Actions arrive as queued input ([`AccessAction`]) on the node or sub-node they name.

use std::hash::Hash;

use accesskit::{Action, ActionData, Node, Role as NodeRole};
use craft_ui_rt::{Rect, Sense, hash_key};

use crate::contract::Id;

pub use crate::runtime::access::{GridPlace, Numeric, Politeness, PopupKind, SetPosition};

/// What a component is to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Role {
    /// No node: assistive technology passes over the component and reads its children.
    None,
    /// A node of this AccessKit role.
    Node(NodeRole),
}

/// How a node or sub-node is named.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Name {
    /// The component's label ([`Access::label`]).
    #[default]
    FromLabel,
    /// This text.
    Text(String),
    /// These texts, joined with a comma.
    Joined(Vec<String>),
    /// The text of the component with this `Id`.
    LabelledBy(Id),
    /// No name: the node is hidden from assistive technology.
    Decorative,
}

/// The facts a component adds to its AccessKit node: what the pipeline cannot derive from the role
/// and the sense.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Access {
    role: Option<Role>,
    label: Option<String>,
    name: Name,
    description: Option<String>,
    placeholder: Option<String>,
    value: Option<String>,
    numeric: Option<Numeric>,
    expanded: Option<bool>,
    has_popup: Option<PopupKind>,
    level: Option<usize>,
    position: Option<SetPosition>,
    grid: GridPlace,
    required: bool,
    url: Option<String>,
    shortcut: Option<String>,
    access_key: Option<char>,
    live: Option<Politeness>,
    busy: bool,
    disabled: bool,
    selected: Option<bool>,
    toggled: Option<accesskit::Toggled>,
    focusable: bool,
    scroll_into_view: bool,
    pans: (bool, bool),
    custom_actions: Vec<(i32, String)>,
    relations: Relations,
    nodes: Vec<SubNode>,
}

/// The components a node relates to, by `Id`. The access node carries the ids as placeholders; the
/// node's own phase resolves them to nodes ([`super::ids`]).
#[derive(Clone, Debug, Default)]
struct Relations {
    labelled_by: Vec<Id>,
    described_by: Vec<Id>,
    controls: Vec<Id>,
    flow_to: Vec<Id>,
}

impl Access {
    /// The role of this instance, over the component's `ROLE`.
    pub fn role(&mut self, role: Role) -> &mut Self {
        self.role = Some(role);
        self
    }

    /// The component `id` names this one. The target may be a later sibling.
    pub fn labelled_by(&mut self, id: Id) -> &mut Self {
        self.relations.labelled_by.push(id);
        self
    }

    /// The component `id` describes this one.
    pub fn described_by(&mut self, id: Id) -> &mut Self {
        self.relations.described_by.push(id);
        self
    }

    /// This node controls the component `id` (the popup of an open combo box).
    pub fn controls(&mut self, id: Id) -> &mut Self {
        self.relations.controls.push(id);
        self
    }

    /// Reading continues at the component `id`.
    pub fn flow_to(&mut self, id: Id) -> &mut Self {
        self.relations.flow_to.push(id);
        self
    }

    /// The component's label: the name when [`Name::FromLabel`].
    pub fn label(&mut self, text: impl Into<String>) -> &mut Self {
        self.label = Some(text.into());
        self
    }

    /// How the node is named, when the label is not the name.
    pub fn name(&mut self, name: Name) -> &mut Self {
        self.name = name;
        self
    }

    /// A description.
    pub fn description(&mut self, text: impl Into<String>) -> &mut Self {
        self.description = Some(text.into());
        self
    }

    /// The hint text of an empty input.
    pub fn placeholder(&mut self, text: impl Into<String>) -> &mut Self {
        self.placeholder = Some(text.into());
        self
    }

    /// The text value of a text component.
    pub fn value(&mut self, text: impl Into<String>) -> &mut Self {
        self.value = Some(text.into());
        self
    }

    /// The numeric value of a value component. SetValue is declared with it; a step also declares
    /// Increment and Decrement, each only where the value can move that way.
    pub fn numeric(&mut self, numeric: Numeric) -> &mut Self {
        self.numeric = Some(numeric);
        self
    }

    /// Whether a disclosure is open.
    pub fn expanded(&mut self, on: bool) -> &mut Self {
        self.expanded = Some(on);
        self
    }

    /// What kind of popup the component opens.
    pub fn has_popup(&mut self, kind: PopupKind) -> &mut Self {
        self.has_popup = Some(kind);
        self
    }

    /// The heading level or tree depth.
    pub fn level(&mut self, level: usize) -> &mut Self {
        self.level = Some(level);
        self
    }

    /// The place in a closed set.
    pub fn position(&mut self, position: SetPosition) -> &mut Self {
        self.position = Some(position);
        self
    }

    /// The row and column counts or indexes of a grid, row or cell.
    pub fn grid(&mut self, grid: GridPlace) -> &mut Self {
        self.grid = grid;
        self
    }

    /// The input must be filled in.
    pub fn required(&mut self, on: bool) -> &mut Self {
        self.required = on;
        self
    }

    /// The target of a link.
    pub fn url(&mut self, url: impl Into<String>) -> &mut Self {
        self.url = Some(url.into());
        self
    }

    /// The keyboard shortcut.
    pub fn shortcut(&mut self, text: impl Into<String>) -> &mut Self {
        self.shortcut = Some(text.into());
        self
    }

    /// The mnemonic letter.
    pub fn access_key(&mut self, key: char) -> &mut Self {
        self.access_key = Some(key);
        self
    }

    /// Makes the node a live region.
    pub fn live(&mut self, politeness: Politeness) -> &mut Self {
        self.live = Some(politeness);
        self
    }

    /// The node is working.
    pub fn busy(&mut self, on: bool) -> &mut Self {
        self.busy = on;
        self
    }

    /// The node is disabled: it declares no action.
    pub fn disabled(&mut self, on: bool) -> &mut Self {
        self.disabled = on;
        self
    }

    /// Whether the node is selected in its set.
    pub fn selected(&mut self, on: bool) -> &mut Self {
        self.selected = Some(on);
        self
    }

    /// A check box, switch or toggle button's state.
    pub fn toggled(&mut self, state: accesskit::Toggled) -> &mut Self {
        self.toggled = Some(state);
        self
    }

    /// The node takes key focus: it declares Focus.
    pub fn focusable(&mut self) -> &mut Self {
        self.focusable = true;
        self
    }

    /// The component serves ScrollIntoView (a row of a list it scrolls).
    pub fn scroll_into_view(&mut self) -> &mut Self {
        self.scroll_into_view = true;
        self
    }

    /// The component pans its content along the vertical and horizontal axes and serves the scroll
    /// actions on them.
    pub fn pans(&mut self, vertical: bool, horizontal: bool) -> &mut Self {
        self.pans = (vertical, horizontal);
        self
    }

    /// An action the component serves itself; it arrives as [`AccessAction`] with
    /// `Action::CustomAction` and the id in `data`.
    pub fn custom_action(&mut self, id: i32, label: impl Into<String>) -> &mut Self {
        self.custom_actions.push((id, label.into()));
        self
    }

    /// A node below this component's node, keyed by `key` (a content painter keys lines and cells
    /// as it keys its parts). Its id and its actions' `sub` are the key's hash.
    pub fn node(&mut self, key: impl Hash, role: NodeRole) -> &mut SubNode {
        self.nodes.push(SubNode {
            key: hash_key(&key),
            under: None,
            bounds: None,
            access: Access { role: Some(Role::Node(role)), ..Access::default() },
        });
        let at = self.nodes.len() - 1;
        &mut self.nodes[at]
    }

    /// The node and sub-nodes this run keeps: `None` for no node. `sense` and the declared facts
    /// give the actions.
    pub(crate) fn finish(mut self, default: NodeRole, sense: Sense) -> Option<Built> {
        let role = match self.role {
            Some(Role::None) => return None,
            Some(Role::Node(role)) => role,
            None => default,
        };
        let mut subs = Vec::with_capacity(self.nodes.len());
        for sub in std::mem::take(&mut self.nodes) {
            let SubNode { key, under, bounds, access } = sub;
            let role = match access.role {
                Some(Role::Node(role)) => role,
                _ => NodeRole::GenericContainer,
            };
            let mut node = access.write(role, false);
            if let Some(b) = bounds {
                node.set_bounds(accesskit::Rect::new(f64::from(b.min.x), f64::from(b.min.y), f64::from(b.max.x), f64::from(b.max.y)));
            }
            subs.push(BuiltSub { key, parent: under, node });
        }
        let node = self.write(role, sense.takes_press());
        Some(Built { node, subs })
    }

    /// The AccessKit node for these facts.
    fn write(&self, role: NodeRole, clickable: bool) -> Node {
        let mut node = Node::new(role);
        match &self.name {
            Name::FromLabel => {
                if let Some(label) = &self.label {
                    node.set_label(label.as_str());
                }
            }
            Name::Text(text) => node.set_label(text.as_str()),
            Name::Joined(texts) => node.set_label(texts.join(", ")),
            Name::LabelledBy(id) => node.push_labelled_by(accesskit::NodeId(id.value())),
            Name::Decorative => node.set_hidden(),
        }
        let r = &self.relations;
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
        if let Some(text) = &self.description {
            node.set_description(text.as_str());
        }
        if let Some(text) = &self.placeholder {
            node.set_placeholder(text.as_str());
        }
        if let Some(text) = &self.value {
            node.set_value(text.as_str());
        }
        if let Some(n) = &self.numeric {
            if let Some(value) = n.value {
                node.set_numeric_value(value);
            }
            node.set_min_numeric_value(*n.range.start());
            node.set_max_numeric_value(*n.range.end());
            if let Some(text) = &n.text {
                node.set_value(text.as_str());
            }
            if let Some(step) = n.step {
                node.set_numeric_value_step(step);
            }
        }
        if let Some(on) = self.expanded {
            node.set_expanded(on);
        }
        if let Some(kind) = self.has_popup {
            node.set_has_popup(match kind {
                PopupKind::Menu => accesskit::HasPopup::Menu,
                PopupKind::Listbox => accesskit::HasPopup::Listbox,
                PopupKind::Tree => accesskit::HasPopup::Tree,
                PopupKind::Grid => accesskit::HasPopup::Grid,
                PopupKind::Dialog => accesskit::HasPopup::Dialog,
            });
        }
        if let Some(level) = self.level {
            node.set_level(level);
        }
        if let Some(p) = &self.position {
            node.set_position_in_set(p.index);
            if let Some(size) = p.size {
                node.set_size_of_set(size);
            }
            if let Some(level) = p.level {
                node.set_level(level);
            }
        }
        let g = &self.grid;
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
        if self.required {
            node.set_required();
        }
        if let Some(url) = &self.url {
            node.set_url(url.as_str());
        }
        if let Some(text) = &self.shortcut {
            node.set_keyboard_shortcut(text.as_str());
        }
        if let Some(key) = self.access_key {
            node.set_access_key(key.to_string());
        }
        if let Some(politeness) = self.live {
            node.set_live(match politeness {
                Politeness::Polite => accesskit::Live::Polite,
                Politeness::Assertive => accesskit::Live::Assertive,
            });
        }
        if self.busy {
            node.set_busy();
        }
        if let Some(on) = self.selected {
            node.set_selected(on);
        }
        if let Some(state) = self.toggled {
            node.set_toggled(state);
        }
        if self.disabled {
            node.set_disabled();
            return node;
        }
        if clickable {
            node.add_action(Action::Click);
        }
        if self.focusable {
            node.add_action(Action::Focus);
        }
        if self.scroll_into_view {
            node.add_action(Action::ScrollIntoView);
        }
        if let Some(n) = &self.numeric {
            node.add_action(Action::SetValue);
            if let (Some(_), Some(value)) = (n.step, n.value) {
                if n.wraps || value < *n.range.end() {
                    node.add_action(Action::Increment);
                }
                if n.wraps || value > *n.range.start() {
                    node.add_action(Action::Decrement);
                }
            } else if n.step.is_some() {
                node.add_action(Action::Increment);
                node.add_action(Action::Decrement);
            }
        }
        let (vertical, horizontal) = self.pans;
        if vertical {
            node.add_action(Action::ScrollUp);
            node.add_action(Action::ScrollDown);
        }
        if horizontal {
            node.add_action(Action::ScrollLeft);
            node.add_action(Action::ScrollRight);
        }
        if !self.custom_actions.is_empty() {
            node.add_action(Action::CustomAction);
            for (id, label) in &self.custom_actions {
                node.push_custom_action(accesskit::CustomAction { id: *id, description: label.clone() });
            }
        }
        node
    }
}

/// A node below a component's node: it carries every fact an [`Access`] does.
#[derive(Clone, Debug)]
pub struct SubNode {
    key: u64,
    under: Option<u64>,
    bounds: Option<Rect>,
    access: Access,
}

impl SubNode {
    /// Nests this sub-node below another sub-node of the same component.
    pub fn under(&mut self, key: impl Hash) -> &mut Self {
        self.under = Some(hash_key(&key));
        self
    }

    /// The node's rectangle in the component's own coordinates.
    pub fn bounds(&mut self, rect: Rect) -> &mut Self {
        self.bounds = Some(rect);
        self
    }
}

impl std::ops::Deref for SubNode {
    type Target = Access;
    fn deref(&self) -> &Access {
        &self.access
    }
}

impl std::ops::DerefMut for SubNode {
    fn deref_mut(&mut self) -> &mut Access {
        &mut self.access
    }
}

/// A sub-node as a run keeps it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BuiltSub {
    pub(crate) key: u64,
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
