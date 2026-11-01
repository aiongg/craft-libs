//! [`SymmetryMenu`]: the brush options bar's painting symmetry button and its menu.

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-S3)")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::model::SymmetryMode;

craft_ui::words! {
    /// The words of painting symmetry: the menu, the transform's bar and its AccessKit names,
    /// English by default.
    #[non_exhaustive]
    pub struct SymmetryLabels {
        button: "symmetry.button" = "Set painting symmetry options";
        off: "symmetry.off" = "Symmetry Off";
        vertical: "symmetry.vertical" = "Vertical";
        horizontal: "symmetry.horizontal" = "Horizontal";
        dual: "symmetry.dual" = "Dual Axis";
        diagonal: "symmetry.diagonal" = "Diagonal";
        work_path: "symmetry.workPath" = "Work Path";
        transform: "symmetry.transform" = "Transform Symmetry";
        commit: "symmetry.commit" = "Commit transform";
        cancel: "symmetry.cancel" = "Cancel transform";
        group: "symmetry.group" = "Symmetry";
        center: "symmetry.center" = "Center";
        rotation_handle: "symmetry.rotationHandle" = "Rotation handle";
    }
}

/// The symmetry in force, as the menu marks it.
#[derive(Clone, Debug, PartialEq)]
pub enum SymmetryChoice {
    Off,
    /// A straight symmetry of this mode.
    Mode(SymmetryMode),
    /// Symmetry from the path keyed as [`SymmetryPathRow::key`].
    Path(String),
}

/// A path the symmetry can come from (PhotoCraft: the saved paths, the Work Path, the layer's
/// path), in the app's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymmetryPathRow {
    /// The app's key ("work", "layer", or the saved path's name).
    pub key: String,
    /// The row's text: the path's name, or [`SymmetryLabels`]' Work Path.
    pub label: String,
}

/// What the user picked in the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymmetryCommand {
    /// Symmetry Off.
    Off,
    /// A straight mode at the document's centre (the app's command sets it).
    Mode(SymmetryMode),
    /// Symmetry from the path keyed so.
    FromPath(String),
    /// Transform Symmetry: the app opens [`SymmetryTransform`](super::SymmetryTransform).
    Transform,
}

/// What [`SymmetryMenu::show`] reports.
#[derive(Clone, Debug)]
pub struct SymmetryMenuResponse {
    /// The button's response.
    pub response: Response,
    /// The row picked this pass.
    pub command: Option<SymmetryCommand>,
}

/// The painting symmetry button: an icon button (`Icon::PaintingSymmetry`, 24 pt with a 14 pt
/// glyph) named "Set painting symmetry options", pressed while a symmetry is on, with the current
/// mode's or path's name as its value. It opens a [`Menu`](craft_ui::menu::Menu) at least the menu
/// token wide: Symmetry Off (checked while off, disabled then), a separator, Vertical,
/// Horizontal, Dual Axis and Diagonal as radio rows with their glyphs
/// (`Icon::SymmetryVertical` …), a separator and one row per path (checked when it is the source)
/// when there are paths, a separator, then Transform Symmetry (enabled only with
/// [`can_transform`](Self::can_transform)). Enter or Space runs a row; Esc closes the menu.
///
/// ```no_run
/// use craft_draw::ui::symmetry::{SymmetryChoice, SymmetryMenu};
/// # fn demo(ui: &mut egui::Ui, choice: &SymmetryChoice) {
/// if let Some(command) = SymmetryMenu::new("symmetry").current(choice).can_transform(true).show(ui).command {
///     # let _ = command;
/// }
/// # }
/// ```
pub struct SymmetryMenu<'a> {
    id_salt: IdSalt,
    current: Option<&'a SymmetryChoice>,
    paths: &'a [SymmetryPathRow],
    can_transform: bool,
    labels: Option<&'a SymmetryLabels>,
}

impl<'a> SymmetryMenu<'a> {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), current: None, paths: &[], can_transform: false, labels: None }
    }

    /// The symmetry in force; default: off.
    pub fn current(mut self, choice: &'a SymmetryChoice) -> Self {
        self.current = Some(choice);
        self
    }

    /// The paths a symmetry can come from.
    pub fn paths(mut self, rows: &'a [SymmetryPathRow]) -> Self {
        self.paths = rows;
        self
    }

    /// Whether Transform Symmetry is enabled: a straight symmetry is on and a painting tool (Brush,
    /// Pencil, Eraser) is picked with nothing else being edited. The app decides.
    pub fn can_transform(mut self, can: bool) -> Self {
        self.can_transform = can;
        self
    }

    /// The words; default: [`SymmetryLabels`] of the installed words.
    pub fn labels(mut self, labels: &'a SymmetryLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The button, and the menu while it is open.
    pub fn show(self, ui: &mut Ui) -> SymmetryMenuResponse {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:168 `menu`, :151 `mode_button` (min width 220,
        // the menu token's).
        todo!()
    }
}
