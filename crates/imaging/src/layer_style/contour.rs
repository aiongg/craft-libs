//! Contours: the curves that shape a glow, a shadow, a bevel's gloss and Satin.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-7)")]

/// A contour point, input and output in 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContourPoint {
    pub input: f32,
    pub output: f32,
    /// A corner point: the curve runs straight into and out of it.
    pub corner: bool,
}

/// A named contour: at least two points sorted by input, at most [`Contour::MAX_POINTS`].
/// Built-in presets carry their preset name; an edited preset is named "Custom"; imported
/// contours keep their own name and points.
#[derive(Clone, Debug, PartialEq)]
pub struct Contour {
    pub name: String,
    pub points: Vec<ContourPoint>,
}

impl Contour {
    /// The most points a contour holds.
    // pc: crates/engine/src/layer_style.rs:370 `MAX_CONTOUR_POINTS`.
    pub const MAX_POINTS: usize = 64;

    /// The identity contour "Linear": `(0, 0)` to `(1, 1)`.
    pub fn linear() -> Contour {
        // pc: crates/engine/src/layer_style.rs:121.
        Contour {
            name: "Linear".into(),
            points: vec![ContourPoint { input: 0.0, output: 0.0, corner: false }, ContourPoint { input: 1.0, output: 1.0, corner: false }],
        }
    }

    /// The built-in presets in the order of Photoshop's default contour picker, each Photoshop's
    /// own curve as PhotoCraft measured it in Photoshop 27.11 (every inner point a corner):
    /// Linear, Cone, Cone - Inverted, Cove - Deep, Cove - Shallow, Gaussian, Half Round, Ring,
    /// Ring - Double, Rolling Slope - Descending, Rounded Steps, Sawtooth 1. Copy the point
    /// lists from the cited table.
    pub fn presets() -> Vec<Contour> {
        // pc: crates/engine/src/layer_style.rs:120-360 `CONTOURS`.
        todo!()
    }

    /// The preset called `name`, also accepting the names earlier PhotoCraft versions offered
    /// (Cone (Inverted), Domed, Domed (Inverted), Diagonal (Descending)); "linear" in any case is
    /// [`linear`](Self::linear). `None` for a name not shipped, so the host keeps an imported
    /// contour as it is.
    pub fn preset(name: &str) -> Option<Contour> {
        // pc: crates/engine/src/layer_style.rs:362-367 `LEGACY_CONTOURS`, :383-388 `builtin_contour`.
        todo!()
    }

    /// Whether `self` is a built-in preset unchanged (its name and points match the preset).
    pub fn is_preset(&self) -> bool {
        // pc: crates/engine/src/layer_style.rs:397 (a preset is stored by its name).
        todo!()
    }

    /// `points` made valid: `None` for fewer than two or more than [`MAX_POINTS`](Self::MAX_POINTS)
    /// points or a non-finite value; else clamped to 0..=1, sorted by input, points within 1e-6
    /// of the same input collapsed (the first kept), `None` if fewer than two remain.
    pub fn from_points(name: impl Into<String>, points: Vec<ContourPoint>) -> Option<Contour> {
        // pc: crates/engine/src/layer_style.rs:415-444 `contour_from_param`.
        todo!()
    }

    /// The contour after an edit in the Contour Editor: a built-in preset's name becomes
    /// "Custom" (an empty name too), so a saved file never calls the edited curve by the preset's
    /// name; another name is kept.
    pub fn edited(self) -> Contour {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:255-266 `store`.
        todo!()
    }
}
