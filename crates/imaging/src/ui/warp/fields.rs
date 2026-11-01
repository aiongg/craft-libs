//! [`WarpFields`]: the Warp group of a transform's options bar: Split icons, Grid menu, warp style,
//! orientation and Bend, H and V (task TR-T5).

use egui::{AsIdSalt, IdSalt, Ui};

use craft_theme::ControlSize;
use craft_ui::controls::{IconButton, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::words;

use super::{WarpMesh, WarpSplit};

words! {
    /// The words of [`WarpFields`] (PhotoCraft's Warp options bar). `Default` is English.
    pub struct WarpFieldsLabels {
        split: "warpFields.split" = "Split:";
        split_crosswise: "warpFields.splitCrosswise" = "Split Warp Crosswise";
        split_vertically: "warpFields.splitVertically" = "Split Warp Vertically";
        split_horizontally: "warpFields.splitHorizontally" = "Split Warp Horizontally";
        grid: "warpFields.grid" = "Grid:";
        grid_name: "warpFields.gridName" = "Grid";
        custom: "warpFields.custom" = "Custom";
        default_grid: "warpFields.defaultGrid" = "Default";
        three: "warpFields.three" = "3 x 3";
        four: "warpFields.four" = "4 x 4";
        five: "warpFields.five" = "5 x 5";
        warp: "warpFields.warp" = "Warp:";
        warp_name: "warpFields.warpName" = "Warp";
        orientation: "warpFields.orientation" = "Change warp orientation";
        bend: "warpFields.bend" = "Bend:";
        horizontal: "warpFields.horizontal" = "H:";
        vertical: "warpFields.vertical" = "V:";
        horizontal_name: "warpFields.horizontalName" = "Horizontal distortion";
        vertical_name: "warpFields.verticalName" = "Vertical distortion";
    }
}

/// How far a knot may lie from an even one and still read as even (PhotoCraft `warp_grid_id`).
const EVEN_KNOT: f64 = 1e-4;

/// The range of Bend, H and V in percent.
const DISTORTION_RANGE: std::ops::RangeInclusive<f64> = -100.0..=100.0;

/// One warp style of the style select, as the app lists them (Photoshop's 15 styles, None and
/// Custom; the styles are the app's warp model).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarpStyleEntry {
    /// The app's id ("arc", "custom").
    pub key: String,
    /// The translated name ("Arc").
    pub label: String,
    /// A preset style: the orientation and Bend, H and V act on it.
    pub preset: bool,
    /// The Custom style (the mesh edited by hand): hidden over a type layer.
    pub custom: bool,
}

/// The Grid menu's reading of a mesh (PhotoCraft `warp_grid_id`): Default is one patch, the
/// numbered entries uniform n × n patches, anything split by hand Custom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WarpGridChoice {
    Custom,
    #[default]
    Default,
    Three,
    Four,
    Five,
}

impl WarpGridChoice {
    /// What `mesh` reads as: as many columns as rows with even knots (within 1e-4), 1, 3, 4 or 5
    /// of them; otherwise Custom.
    pub fn of(mesh: &WarpMesh) -> WarpGridChoice {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:480 (`warp_grid_id`).
        let columns = mesh.us.len().saturating_sub(1);
        let rows = mesh.vs.len().saturating_sub(1);
        let even = |knots: &[f64]| {
            let n = knots.len().saturating_sub(1);
            n > 0 && knots.iter().enumerate().all(|(i, v)| (v - i as f64 / n as f64).abs() < EVEN_KNOT)
        };
        if columns != rows || !even(&mesh.us) || !even(&mesh.vs) {
            return WarpGridChoice::Custom;
        }
        match columns {
            1 => WarpGridChoice::Default,
            3 => WarpGridChoice::Three,
            4 => WarpGridChoice::Four,
            5 => WarpGridChoice::Five,
            _ => WarpGridChoice::Custom,
        }
    }

    /// The patches per side a pick asks for: Default 1, Three 3, Four 4, Five 5, Custom none.
    pub fn size(self) -> Option<usize> {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:443.
        match self {
            WarpGridChoice::Custom => None,
            WarpGridChoice::Default => Some(1),
            WarpGridChoice::Three => Some(3),
            WarpGridChoice::Four => Some(4),
            WarpGridChoice::Five => Some(5),
        }
    }
}

/// What the group shows: the app's warp as the fields read it.
#[derive(Clone, Debug, PartialEq)]
pub struct WarpFieldValues {
    /// The current style's [`WarpStyleEntry::key`].
    pub style: String,
    /// Vertical orientation (the orientation button pressed).
    pub vertical: bool,
    /// Bend, H and V distortion in percent, −100 to 100.
    pub bend: f64,
    pub horizontal: f64,
    pub vertical_distortion: f64,
    /// The Grid menu's reading of the mesh.
    pub grid: WarpGridChoice,
}

/// What the user changed in the group. The app applies it to its warp and reports the result to
/// the session (`TransformEdit::Warp`).
#[derive(Clone, Debug, PartialEq)]
pub enum WarpFieldsEdit {
    /// A Split icon armed or disarmed; `None` disarms. View state, not a warp change.
    Split(Option<WarpSplit>),
    /// A Grid pick: an even n × n mesh over the box (Custom is never picked, only read).
    Grid(usize),
    /// A style pick, by [`WarpStyleEntry::key`]. PhotoCraft's rule for the app: Custom keeps the
    /// current surface as a one-patch mesh, None resets to the box, a preset keeps the bend when
    /// coming from a preset and takes 50 otherwise.
    Style(String),
    /// The orientation button.
    Orientation { vertical: bool },
    /// The Bend field: a scrub or typing run, or a Set.
    Bend(EditEvent<f64>),
    /// The H field.
    Horizontal(EditEvent<f64>),
    /// The V field.
    Vertical(EditEvent<f64>),
}

/// What the group reported this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WarpFieldsOutput {
    /// The edits in the order they happened.
    pub edits: Vec<WarpFieldsEdit>,
}

/// The Warp group of a transform's options bar, shown by the draw library's `TransformBar` in its
/// Warp phase through its `warp_fields` closure.
///
/// - In order: "Split:" and three `IconButton`s (Crosswise, Vertically, Horizontally; the armed
///   one pressed, a press on it disarms), a divider, "Grid:" and a `Select` (Custom, Default,
///   3 x 3, 4 x 4, 5 x 5), a divider, "Warp:" and the style `Select`, the orientation
///   `IconButton` (pressed when vertical), and Bend, H, V `NumberField`s (−100 to 100 %).
/// - [`on_type`](Self::on_type): Split and Grid draw disabled and Custom is not offered (PhotoCraft
///   #3004).
/// - The orientation and the three fields are always there and enabled only on a preset style.
/// - Every control is an ordinary focusable control in that order; the group reports and never
///   changes the warp.
pub struct WarpFields<'a> {
    id_salt: IdSalt,
    styles: &'a [WarpStyleEntry],
    values: WarpFieldValues,
    labels: WarpFieldsLabels,
    split: Option<WarpSplit>,
    on_type: bool,
}

impl<'a> WarpFields<'a> {
    /// The group for the app's `styles` showing `values`.
    pub fn new(id_salt: impl AsIdSalt, styles: &'a [WarpStyleEntry], values: WarpFieldValues) -> Self {
        Self { id_salt: IdSalt::new(id_salt), styles, values, labels: WarpFieldsLabels::default(), split: None, on_type: false }
    }

    pub fn labels(mut self, labels: WarpFieldsLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The armed Split tool, shown pressed.
    pub fn split(mut self, split: Option<WarpSplit>) -> Self {
        self.split = split;
        self
    }

    /// The warp is on a type layer: Split and Grid disabled, no Custom.
    pub fn on_type(mut self, on_type: bool) -> Self {
        self.on_type = on_type;
        self
    }

    /// Draws the group into the options bar's `ui`.
    pub fn show(self, ui: &mut Ui) -> WarpFieldsOutput {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:358 (`warp_fields`), :410
        // (`warp_mesh_fields`), :501 (`split_icon`).
        let Self { id_salt, styles, values, labels, split, on_type } = self;
        let sm = ControlSize::Sm;
        let numbers = LocaleNumbers::of(ui);
        let mut out = WarpFieldsOutput::default();
        ui.push_id(id_salt, |ui| {
            OptionsBar::group(ui, !on_type, |ui| {
                OptionsBar::cell(ui, &labels.split, CellWidth::Content, |ui, _| {
                    let tools = [
                        (WarpSplit::Cross, Icon::SplitCross, &labels.split_crosswise),
                        (WarpSplit::Vertical, Icon::SplitVertical, &labels.split_vertically),
                        (WarpSplit::Horizontal, Icon::SplitHorizontal, &labels.split_horizontally),
                    ];
                    for (tool, image, name) in tools {
                        let armed = split == Some(tool);
                        let button = IconButton::new(image, name.as_str()).size(sm).selected(armed).id_salt(("split", tool as u8));
                        if ui.add(button).clicked() {
                            out.edits.push(WarpFieldsEdit::Split(if armed { None } else { Some(tool) }));
                        }
                    }
                });
                OptionsBar::divider(ui);
                OptionsBar::cell(ui, &labels.grid, CellWidth::Content, |ui, _| {
                    let mut grid = values.grid;
                    let pick = Select::new("grid", labels.grid_name.as_str(), &mut grid)
                        .option(WarpGridChoice::Custom, labels.custom.as_str())
                        .option(WarpGridChoice::Default, labels.default_grid.as_str())
                        .option(WarpGridChoice::Three, labels.three.as_str())
                        .option(WarpGridChoice::Four, labels.four.as_str())
                        .option(WarpGridChoice::Five, labels.five.as_str())
                        .size(sm)
                        .show(ui);
                    if let Some(&picked) = pick.event.as_ref().and_then(EditEvent::value)
                        && picked != values.grid
                        && let Some(size) = picked.size()
                    {
                        out.edits.push(WarpFieldsEdit::Grid(size));
                    }
                });
            });
            OptionsBar::divider(ui);
            OptionsBar::cell(ui, &labels.warp, CellWidth::Content, |ui, _| {
                let mut style = values.style.clone();
                let select = styles
                    .iter()
                    .filter(|entry| !(on_type && entry.custom))
                    .fold(Select::new("style", labels.warp_name.as_str(), &mut style), |select, entry| {
                        select.option(entry.key.clone(), entry.label.as_str())
                    });
                let pick = select.size(sm).show(ui);
                if let Some(picked) = pick.event.as_ref().and_then(EditEvent::value)
                    && *picked != values.style
                {
                    out.edits.push(WarpFieldsEdit::Style(picked.clone()));
                }
            });
            // The orientation and the fields are always there and act only on a preset style.
            let preset = styles.iter().any(|entry| entry.key == values.style && entry.preset);
            OptionsBar::group(ui, preset, |ui| {
                let button =
                    IconButton::new(Icon::SwapHorizontal, labels.orientation.as_str()).size(sm).selected(values.vertical).id_salt("orientation");
                if ui.add(button).clicked() {
                    out.edits.push(WarpFieldsEdit::Orientation { vertical: !values.vertical });
                }
                let fields: [(&str, &str, &str, f64, fn(EditEvent<f64>) -> WarpFieldsEdit); 3] = [
                    ("bend", &labels.bend, labels.bend.trim_end_matches(':'), values.bend, WarpFieldsEdit::Bend),
                    ("horizontal", &labels.horizontal, &labels.horizontal_name, values.horizontal, WarpFieldsEdit::Horizontal),
                    ("vertical", &labels.vertical, &labels.vertical_name, values.vertical_distortion, WarpFieldsEdit::Vertical),
                ];
                for (salt, caption, name, value, edit) in fields {
                    let field = NumberField::new(salt, name, value, &numbers).range(DISTORTION_RANGE).decimals(0..=0).unit(Unit::new("{n}%"));
                    if let Some(event) = OptionsBar::number_cell(ui, caption, CellWidth::Sm, field).event {
                        out.edits.push(edit(event));
                    }
                }
            });
        });
        out
    }
}
