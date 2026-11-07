//! Guide data: what a `Guides` control shows and reports. Data only; the control is a canvas control.

use craft_behavior::{Axis, GesturePhase, PartKey, PartRef};
use craft_keymap::Mods;
use craft_theme::Rgba;

/// The identity of one guide among those shown together: the host's name for it, and the index of its part
/// (`GuideKey::part`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GuideKey(u64);

impl GuideKey {
    /// The key of the guide the host calls `id`.
    pub const fn new(id: u64) -> Self {
        GuideKey(id)
    }

    /// The host's number for the guide.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The part of the guide in a component that declares `part` for its guides.
    pub const fn part(self, part: PartKey) -> PartRef {
        part.at(self.0)
    }
}

/// How a guide's line is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GuideLine {
    /// An unbroken line.
    #[default]
    Solid,
    /// A line of dashes.
    Dashed,
    /// A line of dots.
    Dots,
}

/// A guide of the host's document.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Guide {
    /// The host's name for the guide, unique among the guides shown together.
    pub key: GuideKey,
    /// The axis the guide runs along: `Axis::X` for a horizontal guide, at a y position.
    pub along: Axis,
    /// Document coordinate across the guide: x for a vertical guide (along `Axis::Y`).
    pub position: f64,
    /// Where the guide runs along its length, from and to in document units; `None` across the whole view
    /// (PhotoCraft).
    pub span: Option<(f64, f64)>,
    /// The host's colour for this guide, in place of the theme's guide colour (PhotoCraft's New Guide colour and
    /// preferences). The selected guide takes the selection colour whatever it is.
    pub color: Option<Rgba>,
    /// How the line is drawn.
    pub line: GuideLine,
    /// A locked guide is drawn but takes no pointer or key.
    pub locked: bool,
}

impl Guide {
    /// A solid, unlocked guide across the view in the theme's colour.
    pub const fn new(key: GuideKey, along: Axis, position: f64) -> Self {
        Self { key, along, position, span: None, color: None, line: GuideLine::Solid, locked: false }
    }
}

/// A guide being created or dragged, as the ruler drag or the guide's own part reports it. Build one with
/// [`GuideDrag::new_guide`] for a guide dragged out of a ruler and [`GuideDrag::moving`] for an existing guide.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideDrag {
    /// The guide being moved; `None` for a new guide dragged out of a ruler.
    pub key: Option<GuideKey>,
    /// The axis the guide runs along.
    pub along: Axis,
    /// Where the gesture stands in this pass.
    pub phase: GesturePhase,
    /// The document coordinate across the guide; `None` when the pointer is off the canvas, which removes the guide
    /// on release.
    pub at: Option<f64>,
    /// The modifiers held (Shift snaps to the ruler's ticks).
    pub mods: Mods,
}

impl GuideDrag {
    /// A new guide running along `along`, dragged out of a ruler: in `phase`, at `at`, with `mods` held.
    pub const fn new_guide(along: Axis, phase: GesturePhase, at: Option<f64>, mods: Mods) -> Self {
        Self { key: None, along, phase, at, mods }
    }

    /// The existing guide `key`, running along `along`, dragged: in `phase`, at `at`, with `mods` held.
    pub const fn moving(key: GuideKey, along: Axis, phase: GesturePhase, at: Option<f64>, mods: Mods) -> Self {
        Self { key: Some(key), along, phase, at, mods }
    }
}
