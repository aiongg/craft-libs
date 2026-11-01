//! Tone zones ([`ToneZone`], [`ScopeZoneEdit`]): Camera Raw's and Lightroom's draggable ranges
//! over the histogram.

use craft_ui::edit::EditEvent;

use super::labels::ScopePanelLabels;

/// One tone zone over the plot: a range of the plot's width (0..=1) that edits one of the host's
/// controls. Zones are data (lead call L11): PhotoCraft's Camera Raw and LightCraft's Basic panel
/// disagree, so each app passes its own set until a Photoshop reference decides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToneZone {
    /// A stable key the host maps to its control ("blacks", "shadows", "exposure", "highlights",
    /// "whites"); also LightCraft's control id.
    pub key: &'static str,
    /// Where the zone starts, as a share of the plot width.
    pub from: f32,
    /// Where it ends.
    pub to: f32,
}

impl ToneZone {
    /// Camera Raw's five zones: Blacks 0–0.10, Shadows 0.10–0.25, Exposure 0.25–0.75, Highlights
    /// 0.75–0.90, Whites 0.90–1.
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:59-65 `ToneZone::range`.
    pub const CAMERA_RAW: [ToneZone; 5] = [
        ToneZone { key: "blacks", from: 0.0, to: 0.10 },
        ToneZone { key: "shadows", from: 0.10, to: 0.25 },
        ToneZone { key: "exposure", from: 0.25, to: 0.75 },
        ToneZone { key: "highlights", from: 0.75, to: 0.90 },
        ToneZone { key: "whites", from: 0.90, to: 1.0 },
    ];

    /// Lightroom's five zones: Blacks 0–0.15, Shadows 0.15–0.35, Exposure 0.35–0.65, Highlights
    /// 0.65–0.85, Whites 0.85–1.
    // lightcraft: crates/ui-egui/src/panels/histogram.rs:33-39.
    pub const LIGHTROOM: [ToneZone; 5] = [
        ToneZone { key: "blacks", from: 0.0, to: 0.15 },
        ToneZone { key: "shadows", from: 0.15, to: 0.35 },
        ToneZone { key: "exposure", from: 0.35, to: 0.65 },
        ToneZone { key: "highlights", from: 0.65, to: 0.85 },
        ToneZone { key: "whites", from: 0.85, to: 1.0 },
    ];

    /// The zone's name: the [`ScopePanelLabels`] word for the five standard keys, else the key.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn label<'l>(&self, labels: &'l ScopePanelLabels) -> &'l str {
        // new: the blacks, shadows, exposure, highlights and whites words; any other key returns `self.key`.
        todo!()
    }

    /// The zone at `x`, a share of the plot width (clamped to 0..=1): the first zone whose `to`
    /// lies past `x`, the last one at 1.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn at(zones: &[ToneZone], x: f32) -> Option<usize> {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:33 `ToneZone::at`.
        todo!()
    }
}

/// What the user did to one zone.
#[derive(Clone, Debug, PartialEq)]
pub enum ZoneChange {
    /// A drag over the plot, one gesture: `Begin` at the press, `Update` per pass with the signed
    /// share of the plot width moved since the press (Shift drags a tenth as far: LightCraft's
    /// `FINE = 0.1`), `Commit` on release, `Cancel` with Esc. The host maps the share to its
    /// control (PhotoCraft: `share × limit × 2`, limit 5 for Exposure, else 100) and makes one
    /// undo step for the drag.
    Drag(EditEvent<f32>),
    /// Arrow keys with the plot focused, or a screen reader's Increment and Decrement: this many
    /// of the control's own steps (Right and Up +1, Left and Down −1, Shift ×10). One undo step
    /// each.
    Step(i32),
    /// A double-click on the zone: the host resets the control to its default. One undo step.
    Reset,
    /// A screen reader's SetValue, in the control's own units.
    SetValue(f64),
}

/// A tone-zone edit, reported in [`ScopePanelOutput::zone`](super::ScopePanelOutput::zone).
#[derive(Clone, Debug, PartialEq)]
pub struct ScopeZoneEdit {
    /// The zone's index in the slice passed to [`ScopePanel::zones`](super::ScopePanel::zones).
    pub zone: usize,
    /// The zone's key.
    pub key: &'static str,
    /// What happened.
    pub change: ZoneChange,
}

/// The zones' input and look over `plot`: the hovered zone (or the pressed one during a drag)
/// shows its band in `text.normal` at `zone_alpha`, the `ResizeHorizontal` cursor and the tooltip
/// [`ScopePanelLabels::zone_tooltip`] with the host's value text; a press locks the zone under it
/// for the whole drag (LightCraft); the top `warning_size` strip belongs to the warning buttons.
/// AccessKit: a `Group` named [`ScopePanelLabels::tone_zones`] holding one `Slider` per zone,
/// named by its label, value text the host's.
#[expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
pub(super) fn zones(
    ui: &mut egui::Ui,
    response: &egui::Response,
    plot: egui::Rect,
    zones: &[ToneZone],
    values: &[String],
    labels: &ScopePanelLabels,
    style: &super::style::ScopePanelStyle,
) -> Option<ScopeZoneEdit> {
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:412-437 (hover band, tooltip, double-click reset, drag);
    // lightcraft: crates/ui-egui/src/panels/histogram.rs:41-44 (press locks the zone, Shift is a tenth).
    todo!()
}
