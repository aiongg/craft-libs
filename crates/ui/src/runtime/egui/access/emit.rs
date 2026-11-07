//! Step 16's fill of the nodes step 8 created, and of the sub-nodes below them (12.2).

use super::NodeRole;
use super::rules::{RoleFacts, role_violations};
use crate::runtime::access::{Access, Name, Numeric, Politeness, PopupKind, Role, SetPosition, SubNode, scope};
use crate::runtime::env::Label;
use crate::runtime::inspect::{self, Finding};
use crate::runtime::{keys, memory::sub};
use accesskit::Action;
use craft_behavior::PartRef;
use craft_theme::Flags;
use egui::{Context, Id, Rect};

/// What step 16 knows about a component's own node.
pub(crate) struct Own<'a> {
    pub id: Id,
    pub component: &'static str,
    pub role: NodeRole,
    pub rect: Rect,
    pub label: &'a str,
    pub tooltip: Option<&'a str>,
    /// The full text of the first elided run, the name when nothing else names the node.
    pub elided: Option<&'a str>,
    pub description: Option<&'a str>,
    /// What the control's check says while it warns or refuses: the description after `Common.description`,
    /// until a field container's label scope shows it as the field's error text.
    pub validity: Option<&'a str>,
    /// The bound command's hint.
    pub hint: Option<&'a str>,
    /// The bound command's shortcut.
    pub shortcut: Option<&'a str>,
    /// A display component describes itself with its tooltip.
    pub display: bool,
    pub flags: Flags,
    pub enabled: bool,
    pub read_only: bool,
    pub focusable: bool,
    pub clickable: bool,
    pub keeps_name: bool,
    pub access: &'a Access,
}

/// A sub-node and what the pipeline knows about its part.
pub(crate) struct SubDecl<'a> {
    pub owner: Id,
    pub component: &'static str,
    pub node: &'a SubNode,
    pub rect: Rect,
    pub enabled: bool,
    pub focusable: bool,
    pub clickable: bool,
    /// The part is the component's active part: AccessKit focus goes to it.
    pub active: bool,
    pub face: Option<FaceFacts<'a>>,
    /// The component's label, the name of a sub-node with `Name::FromLabel` and no face.
    pub label: &'a str,
    /// The node the sub-node attaches to.
    pub parent: Id,
}

/// What the face painted for a part adds to the part's sub-node.
pub(crate) struct FaceFacts<'a> {
    pub name: &'a str,
    pub flags: Flags,
    pub numeric: Option<&'a Numeric>,
}

/// The id of the sub-node of `part` below the component `id`.
pub(crate) fn sub_node_id(id: Id, part: PartRef) -> Id {
    sub(id, ("node", part))
}

/// The name, or the node that names it.
struct Named {
    text: Option<String>,
    labelled_by: Vec<Id>,
    hidden: bool,
}

fn named(name: &Name, label: &str, fallbacks: &[Option<&str>]) -> Named {
    let (text, labelled_by, hidden) = match name {
        Name::FromLabel => (label.to_owned(), Vec::new(), false),
        Name::Text(text) => (text.clone(), Vec::new(), false),
        Name::Joined(parts) => (parts.join(", "), Vec::new(), false),
        Name::LabelledBy(id) => (String::new(), vec![*id], false),
        Name::Decorative => (String::new(), Vec::new(), true),
    };
    let text = if text.is_empty() && labelled_by.is_empty() && !hidden {
        fallbacks.iter().flatten().find(|text| !text.is_empty()).map_or(text, |text| (*text).to_owned())
    } else {
        text
    };
    Named { text: (!text.is_empty()).then_some(text), labelled_by, hidden }
}

fn bounds(rect: Rect) -> accesskit::Rect {
    accesskit::Rect { x0: f64::from(rect.min.x), y0: f64::from(rect.min.y), x1: f64::from(rect.max.x), y1: f64::from(rect.max.y) }
}

fn ids(ids: &[Id]) -> Vec<accesskit::NodeId> {
    ids.iter().map(|id| id.accesskit_id()).collect()
}

const CHECKABLE: &[accesskit::Role] = &[
    accesskit::Role::CheckBox,
    accesskit::Role::Switch,
    accesskit::Role::RadioButton,
    accesskit::Role::MenuItemCheckBox,
    accesskit::Role::MenuItemRadio,
];
const SELECTABLE: &[accesskit::Role] = &[
    accesskit::Role::Tab,
    accesskit::Role::ListItem,
    accesskit::Role::ListBoxOption,
    accesskit::Role::TreeItem,
    accesskit::Role::Row,
    accesskit::Role::Cell,
];

/// The expanded state of a node: an explicit value, else open when `OPEN` or `EXPANDED` is shown this pass. A
/// component that can be open and is closed says so with `Access::expanded`; a node with no state is not
/// expandable, so `Expand` and `Collapse` are declared only where the state exists.
fn expanded_of(flags: Flags, explicit: Option<bool>) -> Option<bool> {
    explicit.or_else(|| flags.intersects(Flags::OPEN | Flags::EXPANDED).then_some(true))
}

/// The state flags of a node.
pub(crate) fn write_flags(node: &mut accesskit::Node, role: accesskit::Role, flags: Flags, enabled: bool, read_only: bool) {
    if !enabled {
        node.set_disabled();
    }
    if flags.contains(Flags::MIXED) {
        node.set_toggled(accesskit::Toggled::Mixed);
    } else if flags.contains(Flags::CHECKED) || CHECKABLE.contains(&role) {
        node.set_toggled(accesskit::Toggled::from(flags.contains(Flags::CHECKED)));
    }
    if flags.contains(Flags::SELECTED) || SELECTABLE.contains(&role) {
        node.set_selected(flags.contains(Flags::SELECTED));
    }
    if flags.contains(Flags::INVALID) {
        node.set_invalid(accesskit::Invalid::True);
    }
    if flags.contains(Flags::BUSY) {
        node.set_busy();
    }
    if read_only || flags.contains(Flags::READ_ONLY) {
        node.set_read_only();
    }
}

fn write_numeric(node: &mut accesskit::Node, numeric: &Numeric) {
    if let Some(value) = numeric.value {
        node.set_numeric_value(value);
    }
    node.set_min_numeric_value(*numeric.range.start());
    node.set_max_numeric_value(*numeric.range.end());
    if let Some(step) = numeric.step {
        node.set_numeric_value_step(step);
    }
    if let Some(text) = &numeric.text {
        node.set_value(text.clone());
    }
}

fn write_position(node: &mut accesskit::Node, position: Option<SetPosition>, level: Option<usize>) {
    if let Some(position) = position {
        node.set_position_in_set(position.index);
        if let Some(size) = position.size {
            node.set_size_of_set(size);
        }
    }
    if let Some(level) = level.or_else(|| position.and_then(|position| position.level)) {
        node.set_level(level);
    }
}

fn custom_actions(node: &mut accesskit::Node, actions: &[(i32, Label)]) {
    if actions.is_empty() {
        return;
    }
    node.add_action(Action::CustomAction);
    node.set_custom_actions(
        actions.iter().map(|(id, label)| accesskit::CustomAction { id: *id, description: label.as_str().to_owned() }).collect::<Vec<_>>(),
    );
}

fn report_violations(ctx: &Context, component: &'static str, facts: &RoleFacts<'_>) {
    for missing in role_violations(facts) {
        if cfg!(debug_assertions) {
            log::warn!("{component}: a node of role {:?} needs {missing}", facts.role);
        }
        inspect::finding(ctx, Finding { rule: "RoleRule", component, subject: format!("{:?}: {missing}", facts.role) });
    }
}

/// Fills the node step 8 created for `own`, registers it for action routing and in the label scope.
pub(crate) fn fill_own(ctx: &Context, own: &Own<'_>) {
    let access = own.access;
    let name = named(&access.name, own.label, &[own.tooltip, own.elided]);
    let description = access
        .description
        .as_deref()
        .or(own.description)
        .or(own.validity)
        .or(own.hint)
        .or_else(|| own.tooltip.filter(|tip| own.display || name.text.as_deref() != Some(*tip)));
    let shortcut = access.shortcut.as_deref().or(own.shortcut);
    let expanded = expanded_of(own.flags, access.expanded);
    let mut relations = access.relations.clone();
    relations.labelled_by.extend(name.labelled_by.iter().copied());
    let numeric = access.numeric.as_ref();
    ctx.accesskit_node_builder(own.id, |node| {
        node.set_role(own.role);
        node.set_bounds(bounds(own.rect));
        if name.hidden {
            node.set_hidden();
        }
        if let Some(text) = &name.text {
            node.set_label(text.clone());
        }
        if let Some(text) = description {
            node.set_description(text);
        }
        if let Some(text) = &access.placeholder {
            node.set_placeholder(text.clone());
        }
        if let Some(text) = &access.value {
            node.set_value(text.clone());
        }
        if let Some(numeric) = numeric {
            write_numeric(node, numeric);
        }
        if let Some(expanded) = expanded {
            node.set_expanded(expanded);
            node.add_action(if expanded { Action::Collapse } else { Action::Expand });
        }
        if let Some(popup) = access.has_popup {
            node.set_has_popup(match popup {
                PopupKind::Menu => accesskit::HasPopup::Menu,
                PopupKind::Listbox => accesskit::HasPopup::Listbox,
                PopupKind::Tree => accesskit::HasPopup::Tree,
                PopupKind::Grid => accesskit::HasPopup::Grid,
                PopupKind::Dialog => accesskit::HasPopup::Dialog,
            });
        }
        write_position(node, access.position, access.level);
        if access.required {
            node.set_required();
        }
        if let Some(url) = &access.url {
            node.set_url(url.clone());
        }
        if let Some(shortcut) = shortcut {
            node.set_keyboard_shortcut(shortcut);
        }
        if let Some(key) = access.access_key {
            node.set_access_key(key.to_string());
        }
        if !relations.labelled_by.is_empty() {
            node.set_labelled_by(ids(&relations.labelled_by));
        }
        if !relations.described_by.is_empty() {
            node.set_described_by(ids(&relations.described_by));
        }
        if !relations.controls.is_empty() {
            node.set_controls(ids(&relations.controls));
        }
        if !relations.flow_to.is_empty() {
            node.set_flow_to(ids(&relations.flow_to));
        }
        if let Some(politeness) = access.live {
            node.set_live(match politeness {
                Politeness::Polite => accesskit::Live::Polite,
                Politeness::Assertive => accesskit::Live::Assertive,
            });
        }
        if access.busy {
            node.set_busy();
        }
        write_flags(node, own.role, own.flags, own.enabled, own.read_only);
        if own.focusable {
            node.add_action(Action::Focus);
        }
        if own.clickable {
            node.add_action(Action::Click);
        }
        if numeric.is_some_and(|numeric| numeric.step.is_some()) {
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
        }
        if numeric.is_some() {
            node.add_action(Action::SetValue);
        }
        custom_actions(node, &access.custom_actions);
    });
    report_violations(
        ctx,
        own.component,
        &RoleFacts {
            role: own.role,
            named: name.text.is_some() || !relations.labelled_by.is_empty() || name.hidden,
            numeric,
            position: access.position,
            level: access.level,
            expanded,
            controls: !relations.controls.is_empty(),
        },
    );
    keys::register_node(ctx, own.id, own.id, None, own.focusable);
    scope::register(ctx, own.id, own.keeps_name);
}

/// Creates and fills the sub-node of `decl`.
pub(crate) fn fill_sub(ctx: &Context, decl: &SubDecl<'_>) {
    let node = decl.node;
    let id = sub_node_id(decl.owner, node.part);
    let Role::Node(role) = node.role else { return };
    let face_name = decl.face.as_ref().map(|face| face.name);
    let name = named(&node.name, face_name.unwrap_or(decl.label), &[]);
    if name.hidden {
        return;
    }
    let flags = node.flags | decl.face.as_ref().map_or(Flags::empty(), |face| face.flags);
    let numeric = node.numeric.as_ref().or_else(|| decl.face.as_ref().and_then(|face| face.numeric));
    let expanded = expanded_of(flags, None);
    super::register_parent(ctx, id, decl.parent);
    ctx.accesskit_node_builder(id, |out| {
        out.set_role(role);
        out.set_bounds(bounds(decl.rect));
        if let Some(text) = &name.text {
            out.set_label(text.clone());
        }
        if !name.labelled_by.is_empty() {
            out.set_labelled_by(ids(&name.labelled_by));
        }
        if let Some(text) = &node.description {
            out.set_description(text.clone());
        }
        if let Some(numeric) = numeric {
            write_numeric(out, numeric);
        }
        if let Some(expanded) = expanded {
            out.set_expanded(expanded);
        }
        write_position(out, node.position, None);
        write_flags(out, role, flags, decl.enabled, false);
        if decl.focusable {
            out.add_action(Action::Focus);
        }
        if decl.clickable {
            out.add_action(Action::Click);
        }
        if numeric.is_some_and(|numeric| numeric.step.is_some()) {
            out.add_action(Action::Increment);
            out.add_action(Action::Decrement);
        }
        if numeric.is_some() {
            out.add_action(Action::SetValue);
        }
        custom_actions(out, &node.custom_actions);
    });
    report_violations(
        ctx,
        decl.component,
        &RoleFacts {
            role,
            named: name.text.is_some() || !name.labelled_by.is_empty(),
            numeric,
            position: node.position,
            level: None,
            expanded,
            controls: false,
        },
    );
    keys::register_node(ctx, id, decl.owner, Some(node.part), decl.focusable);
    if decl.active {
        ctx.set_accesskit_focus(id);
    }
}
