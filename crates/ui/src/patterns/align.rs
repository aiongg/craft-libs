#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`AlignBar`]: the Move tool's Align and Distribute controls in the options bar (F17.03).
//! Photoshop shows them whenever the Move tool is active; they enable once enough layers are
//! selected.

use egui::{AsIdSalt, IdSalt, Response, Ui};

use crate::icons::Icon;

crate::words! {
    /// The align bar's words, English by default (Photoshop's Layer › Align and Distribute
    /// menus). A button's tooltip is "Align: Left Edges".
    #[non_exhaustive]
    pub struct AlignLabels {
        align: "align.align" = "Align";
        distribute: "align.distribute" = "Distribute";
        /// The ⋯ button's name and tooltip.
        more: "align.more" = "Align and distribute";
        /// `{group}: {action}`.
        tooltip: "align.tooltip" = "{group}: {action}";
        left_edges: "align.leftEdges" = "Left Edges";
        horizontal_centers: "align.horizontalCenters" = "Horizontal Centers";
        right_edges: "align.rightEdges" = "Right Edges";
        top_edges: "align.topEdges" = "Top Edges";
        vertical_centers: "align.verticalCenters" = "Vertical Centers";
        bottom_edges: "align.bottomEdges" = "Bottom Edges";
        horizontally: "align.horizontally" = "Horizontally";
        vertically: "align.vertically" = "Vertically";
    }
}

/// One align or distribute command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlignAction {
    AlignLeftEdges,
    AlignHorizontalCenters,
    AlignRightEdges,
    AlignTopEdges,
    AlignVerticalCenters,
    AlignBottomEdges,
    DistributeTopEdges,
    DistributeVerticalCenters,
    DistributeBottomEdges,
    DistributeLeftEdges,
    DistributeHorizontalCenters,
    DistributeRightEdges,
    /// Equal gaps between the layers, left to right.
    DistributeHorizontally,
    /// Equal gaps between the layers, top to bottom.
    DistributeVertically,
}

impl AlignAction {
    /// The six align buttons, in Photoshop's order.
    // pc: crates/ui-egui/src/tools/align_bar.rs:11 ALIGN
    pub const ALIGN: [AlignAction; 6] = [
        AlignAction::AlignLeftEdges,
        AlignAction::AlignHorizontalCenters,
        AlignAction::AlignRightEdges,
        AlignAction::AlignTopEdges,
        AlignAction::AlignVerticalCenters,
        AlignAction::AlignBottomEdges,
    ];

    /// The eight distribute commands, in PhotoCraft's menu order.
    // pc: crates/ui-egui/src/tools/align_bar.rs:20 DISTRIBUTE
    pub const DISTRIBUTE: [AlignAction; 8] = [
        AlignAction::DistributeTopEdges,
        AlignAction::DistributeVerticalCenters,
        AlignAction::DistributeBottomEdges,
        AlignAction::DistributeLeftEdges,
        AlignAction::DistributeHorizontalCenters,
        AlignAction::DistributeRightEdges,
        AlignAction::DistributeHorizontally,
        AlignAction::DistributeVertically,
    ];

    /// The action's icon: `align_left_edges`, `align_horizontal_centers`, …,
    /// `distribute_horizontally`, `distribute_vertically` (the variant names in snake case). The
    /// icon set has none of them yet: the icons task adds the 14 to `ui/icons/icons.toml` (Lucide's
    /// `align-start-vertical` … `align-vertical-space-around`, Phosphor's `align-left` …, as the
    /// file's other rows pair sets; PhotoCraft draws its own `align-edge-left` … `distribute-vertical`).
    pub fn icon(self) -> Icon {
        todo!()
    }

    /// The action's own name ("Left Edges"); its group is [`is_align`](Self::is_align).
    pub fn label(self, labels: &AlignLabels) -> &str {
        todo!()
    }

    pub fn is_align(self) -> bool {
        Self::ALIGN.contains(&self)
    }
}

/// Where the distribute commands sit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DistributeButtons {
    /// In the ⋯ menu only, with the align commands (PhotoCraft, Photoshop's narrow bar).
    #[default]
    Menu,
    /// Also as a second group of eight buttons after a divider (Photoshop's wide bar; PdfCraft's
    /// and FilmCraft's arrange bars).
    Row,
}

/// What the bar did this pass.
#[derive(Debug)]
pub struct AlignBarResponse {
    /// The command picked, from a button or the menu.
    pub picked: Option<AlignAction>,
    pub response: Response,
}

/// The six align icon buttons (24 pt, 16 pt icons, no gaps), then a ⋯ button whose menu lists
/// Align and Distribute as sections, each item with its icon and name; with
/// [`DistributeButtons::Row`] the distribute buttons follow too. The buttons are the library's
/// icon buttons and the menu its `PressMenu` (the closing press reaches nothing else, lead call
/// L21); a pick in the menu is reported in the pass it happens.
///
/// Each button's tooltip and AccessKit name is "Align: Left Edges"; a disabled command's button
/// and menu item are disabled. Every button is a Tab stop in the options bar's order; the menu
/// opens on click, Enter or Down and closes on a pick or Esc.
///
/// ```ignore
/// let bar = AlignBar::new("move-align").enabled(|a| session.is_enabled(command_of(a))).show(ui);
/// // Words other than the installed catalog's: AlignBar::new(..).labels(AlignLabels { more: "More".into(), ..AlignLabels::of(ui) })
/// if let Some(action) = bar.picked { app.run(command_of(action), json!({})); }
/// ```
#[must_use = "call show"]
pub struct AlignBar<'a> {
    id_salt: IdSalt,
    enabled: Box<dyn Fn(AlignAction) -> bool + 'a>,
    distribute: DistributeButtons,
    labels: Option<AlignLabels>,
}

impl<'a> AlignBar<'a> {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), enabled: Box::new(|_| true), distribute: DistributeButtons::Menu, labels: None }
    }

    /// The words; [`AlignLabels::of`] the `Ui` when not called.
    pub fn labels(mut self, labels: AlignLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Which commands are enabled now; all of them by default.
    pub fn enabled(mut self, enabled: impl Fn(AlignAction) -> bool + 'a) -> Self {
        self.enabled = Box::new(enabled);
        self
    }

    pub fn distribute(mut self, distribute: DistributeButtons) -> Self {
        self.distribute = distribute;
        self
    }

    // pc: crates/ui-egui/src/tools/align_bar.rs:32 options
    pub fn show(self, ui: &mut Ui) -> AlignBarResponse {
        todo!()
    }
}
