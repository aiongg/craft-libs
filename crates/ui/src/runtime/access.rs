//! `Role`, the access facts of a component and the AccessKit emitter (chapter 12).
//!
//! This module decides what every node says (name, description, actions, rule checks, scope registration) and is
//! the only one that calls `runtime::egui::access`, which writes nodes and makes the parent and focus calls. One
//! `report` per node kind fills a node; `runtime::access::{item, text}` add theirs beside `report` and `report_sub`.

mod emit;
pub(crate) mod items;
mod live;
pub(crate) mod scope;
pub(crate) mod text;

use crate::runtime::egui::access::{self as adapter, NodeRole};
use crate::runtime::env::Label;
use craft_behavior::PartRef;
use craft_theme::{Flags, Tone};
use egui::Id;
use std::ops::RangeInclusive;

pub(crate) use emit::{FaceFacts, Own, PopupShown, SubDecl, report, report_layer, report_sub, sub_node_id};
pub use live::announce;
pub(crate) use live::{begin_pass as begin_live_pass, end_pass as end_live_pass};
pub use scope::{FieldLabel, FieldLabelRole};

/// Makes `parent` the node `id` attaches to. Call it before the node of `id` is created.
pub(crate) fn register_parent(ctx: &egui::Context, id: Id, parent: Id) {
    adapter::register_parent(ctx, id, parent);
}

/// `builder` with `parent` as the node its `Ui`'s AccessKit nodes attach to.
pub(crate) fn with_parent(builder: egui::UiBuilder, parent: Id) -> egui::UiBuilder {
    adapter::with_parent(builder, parent)
}

/// Creates the node of `id` empty if it has none. False while AccessKit is off or the widget is invisible (a
/// popup in its first-show sizing pass).
pub(crate) fn create_node(ctx: &egui::Context, id: Id) -> bool {
    adapter::create_node(ctx, id)
}

/// Reports `id` as the focused node of this pass.
pub(crate) fn set_focus(ctx: &egui::Context, id: Id) {
    adapter::set_focus(ctx, id);
}

/// What a component is to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    /// No node.
    None,
    /// The node merges into its container's node.
    Merged,
    /// A node of this AccessKit role.
    Node(NodeRole),
}

impl Role {
    /// The role of a message of `tone`: status for the neutral tones, alert for a warning or a failure.
    pub const fn of_tone(tone: Tone) -> Role {
        match tone {
            Tone::Warn | Tone::Bad => Role::ALERT,
            _ => Role::STATUS,
        }
    }
}

/// How a node or sub-node is named.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Name {
    /// The component's label (an enclosing field label replaces it).
    #[default]
    FromLabel,
    /// This text.
    Text(String),
    /// These texts, joined with a comma.
    Joined(Vec<String>),
    /// The text of the node with this id.
    LabelledBy(Id),
    /// No name: the node is hidden from assistive technology.
    Decorative,
}

/// A numeric value.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Numeric {
    /// The value, `None` while it has none (a mixed selection).
    pub value: Option<f64>,
    /// The range; a slider needs a finite one.
    pub range: RangeInclusive<f64>,
    /// The step of an increment or decrement; it declares both actions.
    pub step: Option<f64>,
    /// The value as the component shows it, when that differs from the number.
    pub text: Option<String>,
}

impl Numeric {
    /// A value in a range.
    pub fn new(value: f64, range: RangeInclusive<f64>) -> Self {
        Numeric { value: Some(value), range, step: None, text: None }
    }

    /// A range with no value.
    pub fn empty(range: RangeInclusive<f64>) -> Self {
        Numeric { value: None, range, step: None, text: None }
    }

    /// The step of an increment or decrement.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// The value as shown.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

/// A node's place in its set, on the closed list of items it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SetPosition {
    /// The zero-based index.
    pub index: usize,
    /// The number of items, when known.
    pub size: Option<usize>,
    /// The depth in a tree.
    pub level: Option<usize>,
    /// The number of columns, when the item is a row of a grid.
    pub columns: Option<usize>,
}

impl SetPosition {
    /// Position `index`.
    pub const fn new(index: usize) -> Self {
        SetPosition { index, size: None, level: None, columns: None }
    }

    /// In a grid of `columns` columns.
    #[must_use]
    pub const fn columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns);
        self
    }

    /// In a set of `size`.
    #[must_use]
    pub const fn of(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }

    /// At depth `level`.
    #[must_use]
    pub const fn level(mut self, level: usize) -> Self {
        self.level = Some(level);
        self
    }
}

/// The indexes and counts of a grid, table, row or cell node (12.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct GridPlace {
    /// The zero-based row.
    pub row_index: Option<usize>,
    /// The number of rows.
    pub row_count: Option<usize>,
    /// The zero-based column.
    pub column_index: Option<usize>,
    /// The number of columns.
    pub column_count: Option<usize>,
}

impl GridPlace {
    /// A grid of `rows` rows and `columns` columns.
    pub const fn grid(rows: usize, columns: usize) -> Self {
        GridPlace { row_index: None, row_count: Some(rows), column_index: None, column_count: Some(columns) }
    }

    /// The cell at `row` and `column`.
    pub const fn cell(row: usize, column: usize) -> Self {
        GridPlace { row_index: Some(row), row_count: None, column_index: Some(column), column_count: None }
    }

    /// The facts a rule reads.
    pub(crate) const fn facts(&self) -> craft_behavior::GridFacts {
        craft_behavior::GridFacts::new()
            .row_index(self.row_index.is_some())
            .row_count(self.row_count.is_some())
            .column_index(self.column_index.is_some())
            .column_count(self.column_count.is_some())
    }
}

/// How urgently a live announcement is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Politeness {
    /// After the current speech.
    Polite,
    /// At once.
    Assertive,
}

/// The kind of popup a component opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PopupKind {
    /// A menu.
    Menu,
    /// A list box.
    Listbox,
    /// A tree.
    Tree,
    /// A grid.
    Grid,
    /// A dialog.
    Dialog,
}

/// Nodes a node relates to, by id.
#[derive(Clone, Debug, Default)]
pub(crate) struct Relations {
    pub labelled_by: Vec<Id>,
    pub described_by: Vec<Id>,
    pub controls: Vec<Id>,
    pub flow_to: Vec<Id>,
}

/// The facts a component adds to its AccessKit node: what the pipeline cannot derive from the label, the flags
/// and the parts.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Access {
    pub(crate) role: Option<Role>,
    pub(crate) name: Name,
    pub(crate) description: Option<String>,
    pub(crate) placeholder: Option<String>,
    pub(crate) value: Option<String>,
    pub(crate) numeric: Option<Numeric>,
    pub(crate) expanded: Option<bool>,
    pub(crate) has_popup: Option<PopupKind>,
    pub(crate) level: Option<usize>,
    pub(crate) position: Option<SetPosition>,
    pub(crate) grid: GridPlace,
    pub(crate) required: bool,
    pub(crate) url: Option<String>,
    pub(crate) shortcut: Option<String>,
    pub(crate) access_key: Option<char>,
    pub(crate) relations: Relations,
    pub(crate) custom_actions: Vec<(i32, Label)>,
    pub(crate) live: Option<Politeness>,
    pub(crate) busy: bool,
    pub(crate) nodes: Vec<SubNode>,
}

impl Access {
    /// The role of this instance, one of the component's `ALT_ROLES`.
    pub fn role(&mut self, role: Role) -> &mut Self {
        self.role = Some(role);
        self
    }

    /// How the node is named, when the label is not the name.
    pub fn name(&mut self, name: Name) -> &mut Self {
        self.name = name;
        self
    }

    /// A description, over the bound command's hint and the tooltip.
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

    /// The numeric value of a value component; a step also declares increment and decrement.
    pub fn numeric(&mut self, numeric: Numeric) -> &mut Self {
        self.numeric = Some(numeric);
        self
    }

    /// Whether a disclosure is open, when no flag says it.
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

    /// The row and column counts of a grid or table node. A collection's item step fills them for its own node.
    pub fn grid(&mut self, grid: GridPlace) -> &mut Self {
        self.grid = grid;
        self
    }

    /// The place in a closed set.
    pub fn position(&mut self, position: SetPosition) -> &mut Self {
        self.position = Some(position);
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

    /// The keyboard shortcut, over the bound command's.
    pub fn shortcut(&mut self, text: impl Into<String>) -> &mut Self {
        self.shortcut = Some(text.into());
        self
    }

    /// The mnemonic letter.
    pub fn access_key(&mut self, key: char) -> &mut Self {
        self.access_key = Some(key);
        self
    }

    /// The node `id` names this one. Targets are `cx.key(..)` ids; one shown in a later frame follows the
    /// one-frame rule.
    pub fn labelled_by(&mut self, id: Id) -> &mut Self {
        self.relations.labelled_by.push(id);
        self
    }

    /// The node `id` describes this one.
    pub fn described_by(&mut self, id: Id) -> &mut Self {
        self.relations.described_by.push(id);
        self
    }

    /// This node controls the node `id` (the popup of an open combo box).
    pub fn controls(&mut self, id: Id) -> &mut Self {
        self.relations.controls.push(id);
        self
    }

    /// Reading continues at the node `id`.
    pub fn flow_to(&mut self, id: Id) -> &mut Self {
        self.relations.flow_to.push(id);
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

    /// A node below this component's node, keyed by a `PartRef` (content painters key lines and runs as
    /// `LINE.at(i)`), with the id `sub(id, ("node", part_ref))`.
    pub fn node(&mut self, part: impl Into<PartRef>, role: Role) -> &mut SubNode {
        self.nodes.push(SubNode::new(part.into(), role));
        let at = self.nodes.len() - 1;
        &mut self.nodes[at]
    }

    /// An action the component serves itself; it arrives through `cx.intents().take_access()`.
    pub fn custom_action(&mut self, id: i32, label: Label) -> &mut Self {
        self.custom_actions.push((id, label));
        self
    }
}

/// A node below a component's node, for a part, a cell or a line of a content painter.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SubNode {
    /// How the sub-node is named.
    pub name: Name,
    /// A description.
    pub description: Option<String>,
    /// A numeric value.
    pub numeric: Option<Numeric>,
    /// Whether the node's popup or region is open; the node is expandable when this or its face says.
    pub expanded: Option<bool>,
    /// The place in a closed set.
    pub position: Option<SetPosition>,
    /// The indexes of a cell.
    pub grid: GridPlace,
    pub(crate) sort: Option<bool>,
    pub(crate) part: PartRef,
    pub(crate) role: Role,
    pub(crate) under: Option<PartRef>,
    pub(crate) flags: Flags,
    pub(crate) custom_actions: Vec<(i32, Label)>,
}

impl SubNode {
    pub(crate) fn new(part: PartRef, role: Role) -> Self {
        SubNode {
            name: Name::FromLabel,
            description: None,
            numeric: None,
            expanded: None,
            position: None,
            grid: GridPlace::default(),
            sort: None,
            part,
            role,
            under: None,
            flags: Flags::empty(),
            custom_actions: Vec::new(),
        }
    }

    /// Nests this sub-node below another sub-node of the same component.
    pub fn under(&mut self, other: impl Into<PartRef>) -> &mut Self {
        self.under = Some(other.into());
        self
    }

    /// An action the component serves itself, targeted at this sub-node.
    pub fn custom_action(&mut self, id: i32, label: Label) -> &mut Self {
        self.custom_actions.push((id, label));
        self
    }
}
