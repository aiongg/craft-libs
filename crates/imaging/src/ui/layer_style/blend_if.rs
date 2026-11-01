//! [`BlendIfBar`]: one Blend If slider (This Layer or Underlying Layer) with its four handles
//! (task AD-10).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-10)")]

use egui::{AsIdSalt, Color32, IdSalt, Ui};

use craft_ui::edit::{EditPolicy, EditResponse};

use super::labels::BlendIfBarLabels;
use crate::layer_style::BlendIfRange;

/// The face of the black handles: grey 20, an image tone (not a token).
// pc: crates/ui-egui/src/dialogs/layer_style.rs:1442.
pub(crate) const BLACK_FACE: Color32 = Color32::from_gray(20);
/// The face of the white handles: grey 240, an image tone (not a token).
// pc: crates/ui-egui/src/dialogs/layer_style.rs:1443.
pub(crate) const WHITE_FACE: Color32 = Color32::from_gray(240);

/// A Blend If slider: a black-to-white ramp (`blend_if.ramp` high, `blend_if.inset` in from each
/// side, the bar's width the available width clamped to `blend_if.width_min..=width_max`,
/// 120..=320; a 32-strip grey ramp with a `field.border` outline) with a black and a white
/// handle under it, each split into two halves by Alt (the layer fades between them). A split
/// pair shows two half triangles; an unsplit pair one whole triangle (`blend_if.handle_width` ×
/// `blend_if.handle_height`, faces grey 20 and grey 240, edges `text.muted`). The
/// value readout ("0 / 20    255": a split side as "low / high") sits above the bar at the
/// trailing end. The ramp never mirrors in a right-to-left interface: it is a tone scale.
///
/// - Pointer: a press picks the nearest handle; between two coinciding halves, the one on the
///   pointer's side. A drag moves it by [`BlendIfRange::moved`]: alone with Alt held, else the
///   pair keeping its spread; the order black low ≤ black high ≤ white low ≤ white high is kept by
///   pushing the others. One gesture per drag (`Begin`, `Update`…, `Commit`; Esc cancels).
/// - Keys (Photoshop has none; lead call L17): the four handles are Tab stops in order. ← and →
///   move the focused handle's pair one level, ⇧ ten levels; ⌥←, ⌥→ move only the focused half.
///   Each press is one `Set`.
/// - AccessKit: each handle is a `Slider` 0..=255 named
///   [`BlendIfBarLabels::handle`] ("Black low, This Layer"), its value the level.
#[must_use = "show the bar with `show`"]
pub struct BlendIfBar {
    id_salt: IdSalt,
    layer: String,
    range: BlendIfRange,
    labels: Option<BlendIfBarLabels>,
    policy: EditPolicy,
    enabled: bool,
}

impl BlendIfBar {
    /// A bar named `layer` ("This Layer", "Underlying Layer": the host's words without the colon)
    /// over `range`.
    pub fn new(id_salt: impl AsIdSalt, layer: impl Into<String>, range: BlendIfRange) -> Self {
        BlendIfBar { id_salt: IdSalt::new(id_salt), layer: layer.into(), range, labels: None, policy: EditPolicy::default(), enabled: true }
    }

    /// The handles' accessible names; the installed words by default.
    pub fn labels(mut self, labels: BlendIfBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// How a drag reports; realtime by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the readout and the bar and senses the handles.
    pub fn show(self, ui: &mut Ui) -> EditResponse<BlendIfRange> {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:1333-1344 (readout above the bar),
        // :1368-1454 `blend_if_bar` (ramp, nearest handle, drag rules, triangles).
        // new: keys, Tab stops and AccessKit (lead call L17); look from `BlendIfBar::style`.
        todo!()
    }
}
