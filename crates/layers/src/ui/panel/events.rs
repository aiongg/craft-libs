//! `LayersEvent` and the types it carries (`LayerSelect`, `LayerToggle`, `LayerDrop`, `LayerMove`,
//! `SubRowMove`, `RowTransfer`).
//!
//! The panel reports what the user did in the order it happened; the app applies each event with
//! its own commands and undo, in the pass it is reported.

use craft_theme::LabelColor;
use craft_ui::behavior::{DropPosition, SelectionPick};
use craft_ui::edit::EditEvent;
use craft_ui::list::{OpenChange, ToggleEdit};
use egui::Modifiers;

use super::footer::{LayersFooterAction, LayersFooterButton};
use crate::ui::rows::LayerPropertyEdit;
use crate::ui::tree::{LayerPart, LayerRowKey, SubRowGroup};

/// What the Layers panel reports.
///
/// ```ignore
/// for event in out.events {
///     match event {
///         LayersEvent::Select(s) => app.select(s.layer, s.pick, s.part),
///         LayersEvent::Toggle(t) => app.toggle(t),          // one undo step per gesture
///         LayersEvent::Properties(p) => app.set_properties(p),
///         LayersEvent::Drop(EditEvent::Commit { value, .. }) => app.drop_rows(value),
///         _ => {}
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LayersEvent<Id> {
    /// A click on a row (Cmd/Ctrl toggles, Shift extends), a selection key, or a click on the
    /// empty space below the rows (`Clear`). A right-click outside the selection selects the row
    /// first. The app sets its selection by the pick and its target from `part`.
    Select(LayerSelect<Id>),
    /// An eye or lock cell: a click is a `Set`, a drag down the column a gesture that sets every
    /// row crossed to the first row's new state. One undo step per gesture.
    Toggle(LayerToggle<Id>),
    /// Alt-click on an eye: show only this layer, or restore when it already is (Photoshop).
    ShowOnly(LayerRowKey<Id>),
    /// A pick in the eye cell's colour menu (None and the label colours): for every selected
    /// layer when the row is in a multi-selection, else for the row.
    LabelColor {
        /// The layers to colour.
        rows: Vec<Id>,
        /// The colour; `None` for none.
        color: Option<LabelColor>,
    },
    /// A group asked to open or close: its triangle (Alt for every group), ← and →, a screen
    /// reader, or the reveal of the active layer. A view change: no undo step.
    Open(OpenChange<Id>),
    /// A layer's effects and Smart Filter rows asked to open or close: the `fx` triangle, or
    /// Shift+→ and Shift+← on the lead; `all` (Alt) for every layer.
    FxOpen {
        /// The layer.
        layer: Id,
        /// Open or close.
        open: bool,
        /// Every layer takes the state.
        all: bool,
    },
    /// A rename committed: never empty after trimming, never the unchanged name.
    Rename {
        /// The layer.
        layer: Id,
        /// The new name, trimmed.
        text: String,
    },
    /// A double click or Enter where it isn't a rename: the app opens Layer Style, a fill's or
    /// adjustment's options, a smart object's contents, edits the type, or converts the
    /// Background.
    Activate {
        /// The row: a layer, or an effects or Smart Filter row (Layer Style on that effect, the
        /// filter's options).
        row: LayerRowKey<Id>,
        /// What was double-clicked.
        on: ActivateOn,
    },
    /// A thumbnail click that isn't a select: Cmd/Ctrl-click loads it as a selection (Shift
    /// adds, Alt subtracts, Shift+Alt intersects; `craft_ui::canvas::SelectionOperation::from_modifiers`),
    /// Shift-click on a mask toggles it, Alt-click on a mask views it, Alt-click on the layer
    /// thumbnail fits the layer on screen.
    ThumbClick {
        /// The layer.
        layer: Id,
        /// The thumbnail.
        part: LayerPart,
        /// The modifiers held.
        modifiers: Modifiers,
    },
    /// The chain before a mask thumbnail: link or unlink the mask.
    MaskLink {
        /// The layer.
        layer: Id,
        /// The mask.
        part: LayerPart,
        /// The state asked for.
        linked: bool,
    },
    /// An edit in the blend, opacity, lock and fill rows: the app applies it to every selected
    /// layer.
    Properties(LayerPropertyEdit),
    /// A drag of rows: `Begin`, then `Commit` with the drop or `Cancel` (Esc, focus lost, released
    /// elsewhere, a dragged row removed); a stacking key (Cmd/Ctrl+] and [, Shift to the ends) is
    /// one `Set`. One undo step per drop.
    Drop(EditEvent<LayerDrop<Id>>),
    /// A click on a footer button.
    Footer(LayersFooterAction),
    /// Rows dropped on a footer button: Delete deletes layers, clears effects, removes filters,
    /// deletes a mask (the app asks first); New Layer duplicates layers; New Group groups them.
    FooterDrop {
        /// The button.
        button: LayersFooterButton,
        /// What was dropped: layer rows, effects or filter rows, or one [`LayerRowKey::Mask`].
        rows: Vec<LayerRowKey<Id>>,
    },
    /// A pick in the host's row menu ([`LayersPanel::row_menu`](super::LayersPanel::row_menu)).
    Menu {
        /// Where the menu was opened.
        target: MenuTarget<Id>,
        /// The host's command.
        command: String,
    },
}

/// A selection change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerSelect<Id> {
    /// The layer picked; `None` for [`SelectionPick::Clear`]. A click on an effects or Smart
    /// Filter row picks its layer.
    pub layer: Option<Id>,
    /// How it was picked: `Replace`, `Toggle` (Cmd/Ctrl), `Range` (Shift), `Clear`.
    pub pick: SelectionPick,
    /// The thumbnail the click landed on, which becomes the target; `None` elsewhere on the row
    /// and for keys (the target stays).
    pub part: Option<LayerPart>,
}

/// An edit of an eye or lock cell.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerToggle<Id> {
    /// The row the gesture started on.
    pub row: LayerRowKey<Id>,
    /// Which column.
    pub column: LayerColumn,
    /// The list's toggle edit: `Set` for a click or a screen reader, `Begin`, `Update`s and
    /// `Commit` or `Cancel` for a sweep and for the visibility key on several rows. `Update` and
    /// `Set` carry the modifiers (VectorCraft's Ctrl-click on an eye).
    pub edit: EditEvent<ToggleEdit<LayerRowKey<Id>>>,
}

/// A toggle column of the rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerColumn {
    /// The eye.
    Visibility,
    /// The lock column ([`LayersPanel::lock_column`](super::LayersPanel::lock_column)), or the
    /// lock indicator's click (which only unlocks).
    Lock,
}

/// What a double click landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActivateOn {
    /// The name of a layer that isn't renamed in place (the Background).
    Name,
    /// A thumbnail.
    Thumbnail(LayerPart),
    /// Anywhere else on the row, or Enter.
    Row,
}

/// What a drag or a stacking key drops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerDrop<Id> {
    /// Layers moved or copied in the stack.
    Layers(LayerMove<Id>),
    /// A Smart Filter moved within its smart object.
    SubRows(SubRowMove<Id>),
    /// Effects or Smart Filters moved or copied onto another layer.
    Transfer(RowTransfer<Id>),
}

/// Layers moved or copied: `layers` go to `position` of `target` (`Into`: as the group's first
/// children).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerMove<Id> {
    /// The layers, in display order; a layer inside another moved group is not listed.
    pub layers: Vec<Id>,
    /// The layer the drop is relative to; it may be one of `layers`.
    pub target: Id,
    /// Before (above), after (below) or into the target.
    pub position: DropPosition,
    /// Alt: copy instead of move.
    pub copy: bool,
}

/// Sub-rows reordered within their layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubRowMove<Id> {
    /// The layer.
    pub layer: Id,
    /// Which rows (Smart Filters; effects keep their order).
    pub group: SubRowGroup,
    /// The row's index, top first.
    pub from: usize,
    /// The index it takes, top first, counted after it is taken out.
    pub to: usize,
    /// Alt: copy instead of move.
    pub copy: bool,
}

/// Effects or Smart Filters dropped on another layer's row (outlined while over it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowTransfer<Id> {
    /// The rows: an "Effects" header for all effects, single effects, or filters.
    pub rows: Vec<LayerRowKey<Id>>,
    /// The layer they go to.
    pub to: Id,
    /// Alt: copy instead of move.
    pub copy: bool,
}

/// Where a row menu was opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuTarget<Id> {
    /// The row.
    pub row: LayerRowKey<Id>,
    /// The thumbnail it was opened on (the pixel or vector mask menu); `None` elsewhere.
    pub part: Option<LayerPart>,
    /// The row was in a multi-selection: the menu acts on every selected layer.
    pub on_selection: bool,
}
