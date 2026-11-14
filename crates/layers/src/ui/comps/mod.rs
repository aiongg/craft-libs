//! `LayerCompsPanel`: the Layer Comps panel, a list of recorded layer states with sweepable
//! option toggles, rename, reorder and the footer actions.

use craft_behavior::EditEvent;
use craft_ui::contract::{Bind, CommandSet, NoValue, Outcome};
use craft_ui::runtime::env::Label;
use egui::{Response, Ui};

/// What a layer comp records of a layer: the options a comp row toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompOption {
    /// Whether the layers are shown.
    Visibility,
    /// Where the layers are.
    Position,
    /// The layers' appearance (styles, blending).
    Appearance,
}

/// One comp as the panel draws it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayerCompRow {
    /// The comp's id.
    pub id: u64,
    /// The name.
    pub name: String,
    /// The comment, shown as the row's tooltip.
    pub comment: String,
    /// The comp the document state matches.
    pub applied: bool,
    /// The recorded layers deleted since capture; above 0 the warning shows.
    pub missing: usize,
    /// Whether the comp records visibility.
    pub visibility: bool,
    /// Whether the comp records position.
    pub position: bool,
    /// Whether the comp records appearance.
    pub appearance: bool,
}

impl LayerCompRow {
    /// A row for comp `id` named `name`, recording nothing, applied nowhere.
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self { id, name: name.into(), comment: String::new(), applied: false, missing: 0, visibility: false, position: false, appearance: false }
    }

    /// The comment, the row's tooltip.
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }

    /// Whether the document state matches this comp.
    pub fn applied(mut self, applied: bool) -> Self {
        self.applied = applied;
        self
    }

    /// The recorded layers deleted since capture.
    pub fn missing(mut self, missing: usize) -> Self {
        self.missing = missing;
        self
    }

    /// Whether the comp records visibility.
    pub fn visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    /// Whether the comp records position.
    pub fn position(mut self, position: bool) -> Self {
        self.position = position;
        self
    }

    /// Whether the comp records appearance.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
}

/// What the user did in the Layer Comps panel, in order.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LayerCompsEvent {
    /// The fixed Last Document State row was activated.
    RestoreLastDocumentState,
    /// A row was selected.
    Select(u64),
    /// A comp was applied (double click or its apply mark).
    Apply(u64),
    /// A rename ended.
    #[non_exhaustive]
    Rename {
        /// The comp.
        comp: u64,
        /// The new name.
        name: String,
    },
    /// A comp was dragged or moved by key to `index`.
    #[non_exhaustive]
    Reorder {
        /// The comp.
        comp: u64,
        /// Its new place.
        index: usize,
    },
    /// An option toggle of one or more comps, one gesture per sweep (`Update`s, a `Commit` on
    /// release, a `Cancel` carrying the earlier state).
    #[non_exhaustive]
    SetOption {
        /// The comps the gesture touches.
        comps: Vec<u64>,
        /// The option.
        option: CompOption,
        /// The edit.
        edit: EditEvent<bool>,
    },
    /// The warning on this comp was cleared.
    ClearWarning(u64),
    /// Apply the previous comp.
    Previous,
    /// Apply the next comp.
    Next,
    /// Update this comp from the document.
    Update(u64),
    /// Create a comp.
    New,
    /// Delete this comp.
    Delete(u64),
}

craft_ui::words! {
    /// The words of the Layer Comps panel.
    pub struct LayerCompsWords {
        /// The fixed first row.
        last_state: "layerComps.lastDocumentState" = "Last Document State";
        /// The tooltip of the warning on a comp that lost layers.
        missing: "layerComps.missing" = "Layers of this comp were deleted";
    }
}

/// The Layer Comps panel: a list with three sweepable toggle columns, rename, reorder (Last
/// Document State fixed first), a warning trailing part and the footer actions.
#[must_use = "show the panel with `show`"]
#[derive(craft_ui::contract::Pattern)]
pub struct LayerCompsPanel<'a> {
    label: Label,
    rows: &'a [LayerCompRow],
    selected: Bind<'a, Option<u64>>,
    last_state_applied: bool,
    document: Option<u64>,
    empty: Option<Label>,
}

/// What [`LayerCompsPanel::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app applies the panel's events"]
pub struct LayerCompsOutput {
    /// The panel.
    pub response: Response,
    /// What the user did this pass, in order.
    pub events: Vec<LayerCompsEvent>,
}

impl Outcome for LayerCompsOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl<'a> LayerCompsPanel<'a> {
    /// The panel named `label` showing `rows`, with `selected` the selected comp.
    pub fn new(label: impl Into<Label>, rows: &'a [LayerCompRow], selected: Bind<'a, Option<u64>>) -> Self {
        Self { label: label.into(), rows, selected, last_state_applied: false, document: None, empty: None }
    }

    /// Whether the document state is the last document state (the fixed row's mark).
    pub fn last_state_applied(mut self, applied: bool) -> Self {
        self.last_state_applied = applied;
        self
    }

    /// The document the panel shows, so the list's own state is per document.
    pub fn document(mut self, key: impl std::hash::Hash) -> Self {
        use std::hash::Hasher as _;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        self.document = Some(hasher.finish());
        self
    }

    /// The text the list shows with no document.
    pub fn empty(mut self, text: impl Into<Label>) -> Self {
        self.empty = Some(text.into());
        self
    }

    /// Draws the panel over the available rect.
    pub fn show(self, ui: &mut Ui) -> LayerCompsOutput {
        let Self { label, rows, selected, last_state_applied, document, empty } = self;
        let _ = (ui, label, rows, selected, last_state_applied, document, empty);
        todo!("pn.C43")
    }
}

/// The row menu's commands: move the selected comp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CommandSet)]
#[command(prefix = "layerCompsPanel")]
pub enum LayerCompsRowCommands {
    /// Move the comp up.
    #[command(label = "Move Up")]
    MoveUp,
    /// Move the comp down.
    #[command(label = "Move Down")]
    MoveDown,
}
