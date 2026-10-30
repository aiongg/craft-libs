//! Crop frame composition guides and the crop shield's fill: pure geometry and rules, no drawing.
//!
//! [`CropGuide::guides`] gives the polylines of one guide in the frame's unit square (0..1 along
//! its top edge, 0..1 down its left edge), so one geometry maps onto an upright frame or a turned
//! one. `w` and `h` are the frame's side lengths in screen points; Grid and Diagonal depend on the
//! screen aspect. [`GuideShow`] says when the guide shows. [`CropShield::fill`] resolves the shield
//! options to the colour painted over the area outside the frame.

use egui::Color32;

/// A guide polyline in the frame's unit square.
pub type Polyline = Vec<[f32; 2]>;

/// Golden section: 1/phi.
const INV_PHI: f32 = 0.618_034;
/// Target cell side of the Grid guide in screen points: the default of `crop.grid_step`.
pub const GRID_STEP: f32 = 32.0;
/// Most Grid guide lines per axis: the default of `crop.grid_max`.
pub const GRID_MAX: usize = 96;
/// Default shield opacity: `crop.shield_opacity`.
pub const SHIELD_OPACITY: f32 = 0.75;
/// Share of the opacity kept during a gesture with Auto Adjust Opacity: `crop.shield_editing`.
pub const SHIELD_EDITING: f32 = 0.5;

/// The composition guide drawn inside a crop frame, in Photoshop's menu order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum CropGuide {
    #[default]
    Thirds,
    Grid,
    Diagonal,
    Triangle,
    GoldenRatio,
    GoldenSpiral,
}

impl CropGuide {
    /// Every guide, in the order `next` cycles through.
    pub const ALL: [CropGuide; 6] = [Self::Thirds, Self::Grid, Self::Diagonal, Self::Triangle, Self::GoldenRatio, Self::GoldenSpiral];

    /// The English name, for the host to translate.
    pub fn name(self) -> &'static str {
        match self {
            Self::Thirds => "Rule of Thirds",
            Self::Grid => "Grid",
            Self::Diagonal => "Diagonal",
            Self::Triangle => "Triangle",
            Self::GoldenRatio => "Golden Ratio",
            Self::GoldenSpiral => "Golden Spiral",
        }
    }

    /// How many distinct orientations `next_orientation` steps through: the Triangle's diagonal
    /// runs either way, the Golden Spiral winds into any of the four corners; the others are
    /// symmetric.
    pub fn orientations(self) -> u8 {
        match self {
            Self::Triangle => 2,
            Self::GoldenSpiral => 4,
            _ => 1,
        }
    }

    /// The next guide (O); the orientation starts over at 0.
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL.get((i + 1) % Self::ALL.len()).copied().unwrap_or_default()
    }

    /// The guide's next orientation (Shift+O) after `orientation`; a symmetric guide stays at 0
    /// and a stray value comes back in range.
    pub fn next_orientation(self, orientation: u8) -> u8 {
        let n = self.orientations().max(1);
        (orientation % n + 1) % n
    }

    /// The guide's lines for a frame `w` x `h` screen points, in its unit square, with the Grid
    /// guide at the default cell size. `orientation` is taken modulo [`orientations`](Self::orientations).
    /// Sizes under 1 point or non-finite give no lines.
    pub fn guides(self, orientation: u8, w: f32, h: f32) -> Vec<Polyline> {
        self.guides_with(orientation, w, h, GRID_STEP, GRID_MAX)
    }

    /// [`guides`](Self::guides) with the Grid guide's target cell side (`crop.grid_step`) and
    /// most lines per axis (`crop.grid_max`) given.
    pub fn guides_with(self, orientation: u8, w: f32, h: f32, grid_step: f32, grid_max: usize) -> Vec<Polyline> {
        if !(w.is_finite() && h.is_finite()) || w < 1.0 || h < 1.0 {
            return Vec::new();
        }
        let seg = |a: [f32; 2], b: [f32; 2]| vec![a, b];
        let cross = |fs: &[f32]| -> Vec<Polyline> { fs.iter().flat_map(|&f| [seg([f, 0.0], [f, 1.0]), seg([0.0, f], [1.0, f])]).collect() };
        let lines = match self {
            Self::Thirds => cross(&[1.0 / 3.0, 2.0 / 3.0]),
            Self::GoldenRatio => cross(&[1.0 - INV_PHI, INV_PHI]),
            Self::Grid => {
                let axis = |len: f32, vertical: bool| -> Vec<Polyline> {
                    // Square cells of about `grid_step` on screen, at most `grid_max` lines per axis.
                    let n = ((len / grid_step.max(1.0)) as usize).clamp(2, grid_max.max(2));
                    (1..n)
                        .map(|i| {
                            let f = i as f32 / n as f32;
                            if vertical { seg([f, 0.0], [f, 1.0]) } else { seg([0.0, f], [1.0, f]) }
                        })
                        .collect()
                };
                let mut v = axis(w, true);
                v.extend(axis(h, false));
                v
            }
            Self::Diagonal => {
                // 45 degrees on screen from each corner until the line meets the far edge.
                let m = w.min(h);
                let (u, v) = (m / w, m / h);
                vec![seg([0.0, 0.0], [u, v]), seg([1.0, 0.0], [1.0 - u, v]), seg([0.0, 1.0], [u, 1.0 - v]), seg([1.0, 1.0], [1.0 - u, 1.0 - v])]
            }
            Self::Triangle => {
                // The top-left to bottom-right diagonal, and the perpendiculars to it (on screen)
                // from the other two corners; their feet sit at `t` and `s` along it.
                // t = w^2 / (w^2 + h^2) and s = 1 - t, from the smaller ratio so huge sizes can't overflow.
                let (t, s) = if w >= h {
                    let t = 1.0 / (1.0 + (h / w).powi(2));
                    (t, 1.0 - t)
                } else {
                    let s = 1.0 / (1.0 + (w / h).powi(2));
                    (1.0 - s, s)
                };
                vec![seg([0.0, 0.0], [1.0, 1.0]), seg([1.0, 0.0], [t, t]), seg([0.0, 1.0], [s, s])]
            }
            Self::GoldenSpiral => vec![golden_spiral()],
        };
        let o = orientation % self.orientations().max(1);
        let (fx, fy) = (o & 1 != 0, o & 2 != 0);
        lines.into_iter().map(|l| l.into_iter().map(|[x, y]| [if fx { 1.0 - x } else { x }, if fy { 1.0 - y } else { y }]).collect()).collect()
    }
}

/// The golden spiral: quarter arcs through nested golden rectangles, squares cut off the left,
/// top, right and bottom in turn, winding in towards the lower right. Stretched to the unit square.
fn golden_spiral() -> Polyline {
    const PHI: f32 = 1.0 + INV_PHI;
    const STEPS: usize = 12;
    // The remaining rectangle [x0, y0, x1, y1] in a phi x 1 space, and the spiral's current point.
    let mut r = [0.0f32, 0.0, PHI, 1.0];
    let mut p = [0.0f32, 1.0];
    let mut pts = vec![p];
    for i in 0..12 {
        let (w, h) = (r[2] - r[0], r[3] - r[1]);
        let s = w.min(h);
        if s < 1e-4 {
            break;
        }
        // The square cut off, the arc's centre (a corner of it) and the end point; the rest stays.
        let (centre, end) = match i % 4 {
            0 => {
                r[0] += s;
                ([r[0], r[3]], [r[0], r[1]])
            }
            1 => {
                r[1] += s;
                ([r[0], r[1]], [r[2], r[1]])
            }
            2 => {
                r[2] -= s;
                ([r[2], r[1]], [r[2], r[3]])
            }
            _ => {
                r[3] -= s;
                ([r[2], r[3]], [r[0], r[3]])
            }
        };
        let a0 = (p[1] - centre[1]).atan2(p[0] - centre[0]);
        let a1 = (end[1] - centre[1]).atan2(end[0] - centre[0]);
        let mut da = a1 - a0;
        if da > std::f32::consts::PI {
            da -= std::f32::consts::TAU;
        } else if da < -std::f32::consts::PI {
            da += std::f32::consts::TAU;
        }
        for k in 1..=STEPS {
            let a = a0 + da * k as f32 / STEPS as f32;
            pts.push([centre[0] + s * a.cos(), centre[1] + s * a.sin()]);
        }
        p = end;
    }
    pts.into_iter().map(|[x, y]| [(x / PHI).clamp(0.0, 1.0), y.clamp(0.0, 1.0)]).collect()
}

/// When the guide shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum GuideShow {
    /// Only while the frame is being dragged (drawn, moved, resized or turned).
    #[default]
    Auto,
    Always,
    Never,
}

impl GuideShow {
    /// Every rule, in menu order.
    pub const ALL: [GuideShow; 3] = [Self::Auto, Self::Always, Self::Never];

    /// The English menu label, for the host to translate.
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "Auto Show Overlay",
            Self::Always => "Always Show Overlay",
            Self::Never => "Never Show Overlay",
        }
    }

    /// Whether the guide shows, `gesture` being a crop drag in progress.
    pub fn shows(self, gesture: bool) -> bool {
        match self {
            Self::Auto => gesture,
            Self::Always => true,
            Self::Never => false,
        }
    }
}

/// The shield's colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum ShieldColor {
    /// The canvas colour (`surface.canvas`, or the host's pasteboard).
    #[default]
    MatchCanvas,
    /// [`CropShield::custom_color`].
    Custom,
}

/// The crop gear menu's shield options. Photoshop's defaults: shown, enabled, Match Canvas, 75 %,
/// Auto Adjust on.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct CropShield {
    /// Show Cropped Area: off hides everything outside the frame behind the canvas colour.
    pub show_cropped_area: bool,
    /// Enable Crop Shield: off shows the area outside the frame undimmed.
    pub enabled: bool,
    pub color: ShieldColor,
    /// sRGB, used when `color` is `Custom`.
    pub custom_color: [u8; 3],
    /// 0..=1; [`opacity`](Self::opacity) clamps a stray value.
    pub opacity: f32,
    /// Auto Adjust Opacity: lighter while the frame is dragged.
    pub auto_adjust: bool,
    /// Classic mode, where Photoshop greys Show Cropped Area out: it then acts as on.
    pub classic_mode: bool,
}

impl Default for CropShield {
    fn default() -> Self {
        Self {
            show_cropped_area: true,
            enabled: true,
            color: ShieldColor::MatchCanvas,
            custom_color: [0, 0, 0],
            opacity: SHIELD_OPACITY,
            auto_adjust: true,
            classic_mode: false,
        }
    }
}

impl CropShield {
    /// Show Cropped Area as it acts: always on in Classic mode (the stored choice is kept for the
    /// default mode).
    pub fn shows_cropped_area(&self) -> bool {
        self.classic_mode || self.show_cropped_area
    }

    /// The opacity in 0..=1: a non-finite value is the default, an out-of-range one is clamped.
    pub fn opacity(&self) -> f32 {
        let o = if self.opacity.is_finite() { self.opacity } else { SHIELD_OPACITY };
        o.clamp(0.0, 1.0)
    }

    /// What to paint over the area outside the frame, `canvas` being the canvas colour and
    /// `editing` a crop drag in progress: the opaque canvas colour when Show Cropped Area is off,
    /// nothing when the shield is disabled or fully transparent, otherwise its colour at its
    /// opacity.
    pub fn fill(&self, canvas: Color32, editing: bool) -> Option<Color32> {
        let [r, g, b, _] = canvas.to_srgba_unmultiplied();
        if !self.shows_cropped_area() {
            return Some(Color32::from_rgb(r, g, b));
        }
        if !self.enabled {
            return None;
        }
        let [r, g, b] = match self.color {
            ShieldColor::MatchCanvas => [r, g, b],
            ShieldColor::Custom => self.custom_color,
        };
        let a = self.opacity() * if self.auto_adjust && editing { SHIELD_EDITING } else { 1.0 };
        let a = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        (a > 0).then(|| Color32::from_rgba_unmultiplied(r, g, b, a))
    }
}

#[cfg(test)]
#[path = "crop_guides_tests.rs"]
mod tests;
