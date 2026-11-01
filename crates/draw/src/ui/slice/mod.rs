//! The Slice and Slice Select tools ([`SliceTool`]), the slice overlay ([`SliceOverlay`]), their
//! options bar ([`SliceBar`]) and the Slice Options and Divide Slices dialogs.
//!
//! ```no_run
//! use craft_draw::ui::slice::{SliceCommand, SliceEvent, SliceItem, SliceMode, SliceTool};
//! use craft_ui::canvas::{CanvasId, CanvasView, ToolHost};
//! # fn demo(ui: &mut egui::Ui, view: &CanvasView, canvas: &egui::Response, id: CanvasId, tool: &mut SliceTool, slices: &[SliceItem], selected: Option<usize>, locked: bool) {
//! tool.mode(SliceMode::Select).items(slices).selected(selected).locked(locked);
//! let _ = ToolHost::new(id).show(ui, view, canvas, tool);
//! for event in tool.take_events() {
//!     match event {
//!         SliceEvent::Command(SliceCommand::Delete) => { /* slice.delete */ }
//!         SliceEvent::Refused(_) => { /* "Slices are locked (View › Lock Slices)" in the status bar */ }
//!         _ => {}
//!     }
//! }
//! # }
//! ```

use egui::Modifiers;
use kurbo::Rect;

use craft_ui::canvas::Constraint;
use craft_ui::edit::EditEvent;

mod bar;
#[cfg(test)]
mod bar_tests;
mod dialogs;
#[cfg(test)]
mod dialogs_tests;
mod labels;
mod overlay;
pub(crate) mod stories;
mod style;
mod tool;
#[cfg(test)]
mod tool_tests;

pub use bar::{SliceBar, SliceBarOutput};
pub use dialogs::{DivideBy, DivideSlices, DivideSlicesDialog, SliceKind, SliceOptions, SliceOptionsDialog};
pub use labels::{SliceLabels, SliceNames};
pub use overlay::SliceOverlay;
pub use style::SliceStyle;
pub use tool::SliceTool;

/// Slice draws new slices; Select picks, moves and resizes them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SliceMode {
    #[default]
    Slice,
    Select,
}

/// Where a slice comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SliceOrigin {
    /// Drawn by the user (or promoted): moves and resizes.
    #[default]
    User,
    /// Made by the app to fill the space between user slices: drawn dashed, never moves, hidden
    /// with Hide Auto Slices.
    Auto,
    /// Follows a layer's bounds: never moves; its badge carries "▪".
    Layer,
}

/// One slice as the tool sees it this pass: a read-only snapshot of the app's resolved slices.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceItem {
    /// The slice's display number (1, 2, …), how events and the badge name it.
    pub number: usize,
    /// The stored slice's id; `None` for an auto slice.
    pub id: Option<u32>,
    /// In document pixels.
    pub rect: Rect,
    pub origin: SliceOrigin,
}

/// Which product's rules the tools follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SliceRules {
    /// ⇧ makes a new slice square and ⌥ draws it from the centre.
    pub modifiers: bool,
    /// ⇧-click in Select adds or removes a slice from the selection.
    pub multi_select: bool,
}

impl SliceRules {
    /// Photoshop's (PhotoCraft's): no modifiers, one selected slice.
    pub const PHOTOSHOP: SliceRules = SliceRules { modifiers: false, multi_select: false };
    /// Illustrator's (VectorCraft's): ⇧ square, ⌥ from the centre, ⇧-click multi-select.
    pub const ILLUSTRATOR: SliceRules = SliceRules { modifiers: true, multi_select: true };
}

impl Default for SliceRules {
    fn default() -> Self {
        SliceRules::PHOTOSHOP
    }
}

/// A command a Slice tool or [`SliceBar`] asks the app for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SliceCommand {
    /// Delete the selected slice (Delete or Backspace in Select, or the bar's Delete).
    Delete,
    /// Open [`SliceOptionsDialog`] for the selected slice (double click, Enter, the bar's button).
    Options,
    /// Make the selected auto or layer slice a user slice.
    Promote,
    /// Open [`DivideSlicesDialog`] for the selected slice.
    Divide,
    /// Make slices from the guides.
    FromGuides,
    /// Hide Auto Slices turned on or off.
    HideAuto(bool),
}

/// Why a tool did nothing with a press.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SliceRefusal {
    /// View › Lock Slices is on: the app shows "Slices are locked (View › Lock Slices)".
    Locked,
}

/// What [`SliceTool`] reports, drained with [`SliceTool::take_events`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SliceEvent {
    /// A new slice: `Begin` past the click slop (or at the press for Fixed Size), `Update`,
    /// `Commit` with the slice's pixel rectangle, `Cancel`.
    Draw(EditEvent<Rect>),
    /// The slice `number` (and the whole slice when the press hit a covered piece of it) moving to
    /// a new rectangle.
    Move {
        number: usize,
        event: EditEvent<Rect>,
    },
    /// The user slice `number` resized by a handle.
    Resize {
        number: usize,
        event: EditEvent<Rect>,
    },
    /// The selection changed: `number` picked (`None`: nothing), `add` for an Illustrator ⇧-click
    /// that adds or removes it.
    Select {
        number: Option<usize>,
        add: bool,
        modifiers: Modifiers,
    },
    Command(SliceCommand),
    Refused(SliceRefusal),
}

/// The Slice tool's persisted options: its Style, Width and Height (as a marquee's
/// [`Constraint`], pixels for Fixed Size) and Hide Auto Slices.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SliceToolSettings {
    pub style: Constraint,
    pub hide_auto: bool,
}

impl SliceToolSettings {
    /// Settings with `style`, auto slices shown.
    pub fn new(style: Constraint) -> Self {
        SliceToolSettings { style, hide_auto: false }
    }
}
