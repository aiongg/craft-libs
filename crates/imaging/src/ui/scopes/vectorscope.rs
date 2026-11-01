//! [`VectorscopePlot`]: graticule, targets and the hover target ([`scope_point`]).

use egui::{AsIdSalt, IdSalt, Pos2, Rect, Response, Ui};

use super::labels::ScopePanelLabels;

/// The vectorscope's counts for one pass: a hue-major grid of `hues × saturations` cells
/// ([`Vectorscope::counts`](crate::scopes::Vectorscope::counts)) and the sample under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorscopeFrame<'a> {
    /// `hues × saturations` counts, hue-major.
    pub counts: &'a [u32],
    /// Hue cells ([`HUES`](crate::scopes::HUES)).
    pub hues: usize,
    /// Saturation cells ([`SATURATIONS`](crate::scopes::SATURATIONS)).
    pub saturations: usize,
    /// The hue (turns) and saturation (0..=1) of the pixel under the pointer: the hover target.
    pub hover: Option<[f32; 2]>,
}

impl VectorscopeFrame<'_> {
    /// The AccessKit value of the plot, [`ScopePanelLabels::vectorscope_value`] filled from the
    /// counts ([`Vectorscope::summary`](crate::scopes::Vectorscope::summary)'s rules); `None`
    /// when nothing was counted.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn summary(&self, labels: &ScopePanelLabels) -> Option<String> {
        // new: spec AccessKit "Vectorscope".
        todo!()
    }
}

/// Where hue `hue` (turns) at `saturation` (0..=1) sits in a vectorscope drawn in `rect`: the
/// radius is half the shorter side less 14, red at the 3 o'clock position when `red_right`, else
/// at −100°, hue turning counter-clockwise. The cloud, the targets and the hover target use it.
#[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
pub fn scope_point(rect: Rect, red_right: bool, hue: f32, saturation: f32) -> Pos2 {
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:443 `scope_point`.
    todo!()
}

/// The vectorscope plot, usable alone (EffectCraft, LightCraft) or under [`ScopePanel`](super::ScopePanel).
///
/// **What the user sees.** On `data.scope`, rounded `shape.radius.control`: rings at 0.25, 0.5,
/// 0.75 and 1 of the radius in `text.faint` × `scopes.vector_ring_alpha`, 0.7 pt; spokes to R, Y,
/// G, C, B, M in `text.faint` × 0.4, 0.6 pt, each ending in a 2 pt dot and its letter in
/// `scope_hue(h, 1)` (`data.*`); the skin-tone line at 23° in `text.muted` (unless off); the cloud
/// as square dots of half-side `radius / (saturations − 1)` clamped to `scopes.vector_dot_min..=
/// scopes.vector_dot_max`, coloured `scope_hue(h, s)` at alpha `0.2 + 0.8 × sqrt(count / peak)`;
/// the hover target as a 5 pt circle and a 9 pt cross, a 3 pt `data.scope` stroke under a 1.2 pt
/// white one. It never mirrors (`Unmirrored::Graph`).
///
/// **AccessKit.** An `Image` named [`ScopePanelLabels::vectorscope`] with the value text of
/// [`VectorscopeFrame::summary`]. A right-click on it opens the host's scope menu (the panel's).
#[must_use = "show the plot with `show`"]
#[expect(dead_code, reason = "skeleton: removed when the bodies land")]
pub struct VectorscopePlot<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) frame: &'a VectorscopeFrame<'a>,
    pub(super) red_right: bool,
    pub(super) skin_tone: bool,
    pub(super) height: Option<f32>,
    pub(super) labels: Option<ScopePanelLabels>,
}

impl<'a> VectorscopePlot<'a> {
    /// A plot of `frame`; red at −100° and the skin-tone line on by default.
    pub fn new(id_salt: impl AsIdSalt, frame: &'a VectorscopeFrame<'a>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), frame, red_right: false, skin_tone: true, height: None, labels: None }
    }

    /// Show Red at 3 o'clock.
    pub fn red_right(mut self, red_right: bool) -> Self {
        self.red_right = red_right;
        self
    }

    /// Show Skin Tone Indicator.
    pub fn skin_tone(mut self, skin_tone: bool) -> Self {
        self.skin_tone = skin_tone;
        self
    }

    /// The plot's height; without a call 30 % of the available height clamped to 100..=220
    /// (PhotoCraft's Camera Raw header).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The words; the installed words' [`ScopePanelLabels`] without a call.
    pub fn labels(mut self, labels: ScopePanelLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the plot across the available width.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn show(self, ui: &mut Ui) -> Response {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:450-496 `vectorscope`, :511 (height rule).
        todo!()
    }
}
