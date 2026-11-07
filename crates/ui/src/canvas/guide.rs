//! Guide data: what a `Guides` control shows and reports. Data only; the control is a canvas control.

use craft_behavior::kurbo::{Point, Vec2};
use craft_behavior::{GesturePhase, PartKey, PartRef};
use craft_keymap::Mods;
use craft_theme::Rgba;

/// The identity of one guide among those shown together: the host's name for it, and the index of its part
/// (`GuideKey::part`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GuideKey(pub u64);

impl GuideKey {
    /// The part of the guide in a component that declares `part` for its guides.
    pub const fn part(self, part: PartKey) -> PartRef {
        part.at(self.0)
    }
}

/// Which way a guide runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuideAxis {
    /// Across the document's x axis, at a y position.
    Horizontal,
    /// Along the document's y axis, at an x position.
    Vertical,
}

impl GuideAxis {
    /// The other axis.
    pub const fn flipped(self) -> GuideAxis {
        match self {
            GuideAxis::Horizontal => GuideAxis::Vertical,
            GuideAxis::Vertical => GuideAxis::Horizontal,
        }
    }

    /// The document direction a guide moves in: x for a vertical guide. `resize_cursor` takes it for the arrow
    /// across a guide.
    pub fn across(self) -> Vec2 {
        match self {
            GuideAxis::Horizontal => Vec2::new(0.0, 1.0),
            GuideAxis::Vertical => Vec2::new(1.0, 0.0),
        }
    }

    /// The coordinate of `p` a guide is placed by: x for a vertical guide.
    pub const fn position(self, p: Point) -> f64 {
        match self {
            GuideAxis::Horizontal => p.y,
            GuideAxis::Vertical => p.x,
        }
    }
}

/// How a guide's line is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[expect(missing_docs, reason = "the names are the line styles")]
pub enum GuideLine {
    #[default]
    Solid,
    Dashed,
    Dots,
}

/// A guide of the host's document.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Guide {
    /// The host's name for the guide, unique among the guides shown together.
    pub key: GuideKey,
    /// Which way the guide runs.
    pub axis: GuideAxis,
    /// Document coordinate across the guide: x for a vertical guide.
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
    pub const fn new(key: GuideKey, axis: GuideAxis, position: f64) -> Self {
        Self { key, axis, position, span: None, color: None, line: GuideLine::Solid, locked: false }
    }
}

/// A guide being created or dragged, as the ruler drag or the guide's own part reports it. A guide drag from a
/// ruler starts a guide on the axis the ruler runs along.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideDrag {
    /// Which way the guide runs.
    pub axis: GuideAxis,
    /// Where the gesture stands in this pass.
    pub phase: GesturePhase,
    /// The document coordinate across the guide; `None` when the pointer is off the canvas, which removes the guide
    /// on release.
    pub at: Option<f64>,
    /// The modifiers held (Shift snaps to the ruler's ticks).
    pub mods: Mods,
}

impl GuideDrag {
    /// A drag of a guide on `axis` in `phase`, at `at`, with `mods` held.
    pub const fn new(axis: GuideAxis, phase: GesturePhase, at: Option<f64>, mods: Mods) -> Self {
        Self { axis, phase, at, mods }
    }
}
