//! The Artboard tool ([`ArtboardTool`]), its options bar ([`ArtboardBar`]) and the Properties
//! section of an artboard ([`ArtboardProperties`]). Frames and names draw through the kernel's
//! `Pasteboard` (`PageFrame::active` for the active board); the tool draws the handles, the "+"
//! buttons ([`AddBesideHandles`](super::add_beside::AddBesideHandles)) and the drag preview.
//!
//! ```no_run
//! use craft_draw::ui::artboard::{ArtboardEvent, ArtboardItem, ArtboardRules, ArtboardTool};
//! use craft_ui::canvas::{CanvasId, CanvasView, ToolHost};
//! use craft_ui::edit::EditEvent;
//! # fn demo(ui: &mut egui::Ui, view: &CanvasView, canvas: &egui::Response, id: CanvasId, tool: &mut ArtboardTool, boards: &[ArtboardItem], active: usize) {
//! tool.items(boards).selected(&[active]).rules(ArtboardRules::PHOTOSHOP);
//! let _ = ToolHost::new(id).show(ui, view, canvas, tool);
//! for event in tool.take_events() {
//!     match event {
//!         ArtboardEvent::Draw(EditEvent::Commit { value, .. }) => { /* layer.new.artboard */ let _ = value; }
//!         ArtboardEvent::AddBeside { index, side, copy } => { /* layer.artboard.addBeside */ let _ = (index, side, copy); }
//!         _ => {}
//!     }
//! }
//! # }
//! ```

use craft_theme::Srgba;
use egui::Modifiers;
use kurbo::{Point, Rect, Size, Vec2};

use super::add_beside::AddSide;
use craft_ui::edit::EditEvent;

mod bar;
#[cfg(test)]
mod bar_tests;
mod labels;
mod properties;
pub(crate) mod stories;
mod style;
mod tool;
#[cfg(test)]
mod tool_tests;

pub use bar::{ArtboardBar, ArtboardBarOutput};
pub use labels::{ArtboardLabels, ArtboardToolLabels};
pub use properties::{ArtboardProperties, ArtboardPropertiesOutput};
pub use style::ArtboardStyle;
pub use tool::ArtboardTool;

/// An artboard's background.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ArtboardBackground {
    #[default]
    White,
    Black,
    Transparent,
    /// Other…: a colour asked for with `ColorRequest`.
    Custom(Srgba),
}

/// One artboard as the tool sees it this pass: a read-only snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtboardItem {
    pub name: String,
    /// In document pixels.
    pub rect: Rect,
    /// A locked board selects but never moves or resizes (Illustrator).
    pub locked: bool,
    pub background: ArtboardBackground,
}

/// A size preset of the options bar and the Properties section: the app's list (PhotoCraft's
/// engine `PRESETS`); the library has no preset data.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtboardPreset {
    pub name: String,
    /// In document pixels.
    pub size: Size,
}

/// Which product's rules the tool follows.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ArtboardRules {
    /// The "+" buttons beside the selected board.
    pub plus_buttons: bool,
    /// ⇧-click toggles a board in the selection; ⇧-drag on the pasteboard is a marquee over
    /// boards; ⇧-drag inside a board draws a new one.
    pub multi_select: bool,
    /// A double click inside a board reports `OpenOptions` (Illustrator's Artboard Options).
    /// PhotoCraft has no double click on a board.
    pub open_on_double_click: bool,
    /// A click on an object outside the boards fits an artboard to it (`FitObject`).
    pub fit_object: bool,
    /// ⌥-drag inside a board moves a copy.
    pub alt_copies: bool,
    /// Esc with no drag leaves the tool (`Leave`).
    pub escape_leaves: bool,
    /// The arrow step in document pixels (the app's keyboard increment); ⇧ ten times.
    pub arrow_step: f64,
}

impl ArtboardRules {
    /// Photoshop's (PhotoCraft's): "+" buttons, one board selected.
    pub const PHOTOSHOP: ArtboardRules = ArtboardRules {
        plus_buttons: true,
        multi_select: false,
        open_on_double_click: false,
        fit_object: false,
        alt_copies: false,
        escape_leaves: false,
        arrow_step: 1.0,
    };
    /// Illustrator's (VectorCraft's): no "+" buttons, multi-select and marquee, double click opens
    /// Options, fit to an object, ⌥ copies, Esc leaves.
    pub const ILLUSTRATOR: ArtboardRules = ArtboardRules {
        plus_buttons: false,
        multi_select: true,
        open_on_double_click: true,
        fit_object: true,
        alt_copies: true,
        escape_leaves: true,
        arrow_step: 1.0,
    };

    /// These rules with the app's keyboard increment.
    pub fn arrow_step(mut self, step: f64) -> Self {
        self.arrow_step = step;
        self
    }
}

impl Default for ArtboardRules {
    fn default() -> Self {
        ArtboardRules::PHOTOSHOP
    }
}

/// What [`ArtboardTool`] reports, drained with [`ArtboardTool::take_events`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ArtboardEvent {
    /// A new board: `Begin` past the click slop, `Update`, `Commit` with its pixel rect, `Cancel`.
    Draw(EditEvent<Rect>),
    /// The selected boards moving by an offset in whole pixels (with their contents); `copy` for
    /// an Illustrator ⌥-drag.
    Move { indices: Vec<usize>, event: EditEvent<Vec2>, copy: bool },
    /// Board `index` resized by a handle (contents stay): its new pixel rect.
    Resize { index: usize, event: EditEvent<Rect> },
    /// A "+" button (or Insert on a picked board): a board of the same size beside board `index`;
    /// `copy` (⌥) with its contents.
    AddBeside { index: usize, side: AddSide, copy: bool },
    /// The selection changed to `indices` (empty: none).
    Select { indices: Vec<usize>, modifiers: Modifiers },
    /// Delete removed the selected boards.
    Delete { indices: Vec<usize> },
    /// Enter, or a double click inside a board under [`ArtboardRules::open_on_double_click`]:
    /// Artboard Options.
    OpenOptions { index: usize },
    /// Illustrator: a click on an object outside the boards: fit a board to the object at `at`.
    FitObject { at: Point },
    /// Illustrator: Esc with no drag: the app returns to its selection tool.
    Leave,
}
