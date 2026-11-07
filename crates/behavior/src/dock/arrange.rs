//! Window › Arrange: the document arrangements over the floating document windows and
//! the document tab groups, and the rects they produce.

use emath::{Rect, Vec2, pos2, vec2};

/// One Window › Arrange entry. The n-up entries split the document area into that many
/// cells and put one document group in each; the rest act on windows and views.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Arrangement {
    /// Tiles every document window across the document area, side by side.
    TileAllVertically,
    /// Tiles every document window across the document area, stacked.
    TileAllHorizontally,
    /// Two cells, side by side.
    TwoUpVertical,
    /// Two cells, stacked.
    TwoUpHorizontal,
    /// Three cells in a row.
    ThreeUpVertical,
    /// Three cells in a column.
    ThreeUpHorizontal,
    /// Three cells: one large cell beside two stacked cells.
    ThreeUpStacked,
    /// Four cells in a 2 by 2 grid.
    FourUp,
    /// Six cells in a 3 by 2 grid.
    SixUp,
    /// Every document back into one tab group.
    ConsolidateAllToTabs,
    /// Floating windows stepped from the top left ([`cascade_rects`]).
    Cascade,
    /// Floating windows in an even grid.
    Tile,
    /// The active document into a floating window.
    FloatInWindow,
    /// Every document into its own floating window.
    FloatAllInWindows,
    /// A second view of the active document.
    NewWindowForDocument,
    /// Sets every document view to the active view's zoom.
    MatchZoom,
    /// Scrolls every document view to the active view's location.
    MatchLocation,
    /// Sets every document view to the active view's rotation.
    MatchRotation,
    /// Matches zoom, location and rotation together.
    MatchAll,
}

impl Arrangement {
    /// The command id (`window.arrange.tileAllVertically`).
    pub fn command_id(self) -> &'static str {
        match self {
            Self::TileAllVertically => "window.arrange.tileAllVertically",
            Self::TileAllHorizontally => "window.arrange.tileAllHorizontally",
            Self::TwoUpVertical => "window.arrange.twoUpVertical",
            Self::TwoUpHorizontal => "window.arrange.twoUpHorizontal",
            Self::ThreeUpVertical => "window.arrange.threeUpVertical",
            Self::ThreeUpHorizontal => "window.arrange.threeUpHorizontal",
            Self::ThreeUpStacked => "window.arrange.threeUpStacked",
            Self::FourUp => "window.arrange.fourUp",
            Self::SixUp => "window.arrange.sixUp",
            Self::ConsolidateAllToTabs => "window.arrange.consolidateAllToTabs",
            Self::Cascade => "window.arrange.cascade",
            Self::Tile => "window.arrange.tile",
            Self::FloatInWindow => "window.arrange.floatInWindow",
            Self::FloatAllInWindows => "window.arrange.floatAllInWindows",
            Self::NewWindowForDocument => "window.arrange.newWindowForDocument",
            Self::MatchZoom => "window.arrange.matchZoom",
            Self::MatchLocation => "window.arrange.matchLocation",
            Self::MatchRotation => "window.arrange.matchRotation",
            Self::MatchAll => "window.arrange.matchAll",
        }
    }

    /// The entry for a command id, `None` for other ids.
    pub fn from_command_id(id: &str) -> Option<Self> {
        match id.strip_prefix("window.arrange.")? {
            "tileAllVertically" => Some(Self::TileAllVertically),
            "tileAllHorizontally" => Some(Self::TileAllHorizontally),
            "twoUpVertical" => Some(Self::TwoUpVertical),
            "twoUpHorizontal" => Some(Self::TwoUpHorizontal),
            "threeUpVertical" => Some(Self::ThreeUpVertical),
            "threeUpHorizontal" => Some(Self::ThreeUpHorizontal),
            "threeUpStacked" => Some(Self::ThreeUpStacked),
            "fourUp" => Some(Self::FourUp),
            "sixUp" => Some(Self::SixUp),
            "consolidateAllToTabs" => Some(Self::ConsolidateAllToTabs),
            "cascade" => Some(Self::Cascade),
            "tile" => Some(Self::Tile),
            "floatInWindow" => Some(Self::FloatInWindow),
            "floatAllInWindows" => Some(Self::FloatAllInWindows),
            "newWindowForDocument" => Some(Self::NewWindowForDocument),
            "matchZoom" => Some(Self::MatchZoom),
            "matchLocation" => Some(Self::MatchLocation),
            "matchRotation" => Some(Self::MatchRotation),
            "matchAll" => Some(Self::MatchAll),
            _ => None,
        }
    }

    /// How many document groups an n-up or tile arrangement wants, `None` for the window and
    /// view entries.
    pub fn group_count(self, documents: usize) -> Option<usize> {
        match self {
            Self::TileAllVertically | Self::TileAllHorizontally | Self::Tile => Some(documents),
            Self::TwoUpVertical | Self::TwoUpHorizontal => Some(2),
            Self::ThreeUpVertical | Self::ThreeUpHorizontal | Self::ThreeUpStacked => Some(3),
            Self::FourUp => Some(4),
            Self::SixUp => Some(6),
            _ => None,
        }
    }

    /// Whether the entry is enabled with `documents` open and `floating` floating windows:
    /// n-up needs at least two documents, Float in Window a docked one, Consolidate a second
    /// group or a floating window, the Match entries at least two documents.
    pub fn enabled(self, documents: usize, groups: usize, floating: usize) -> bool {
        match self {
            Self::ConsolidateAllToTabs => groups > 1 || floating > 0,
            Self::FloatInWindow => documents > floating,
            Self::NewWindowForDocument => documents > 0,
            Self::TileAllVertically | Self::TileAllHorizontally | Self::Cascade | Self::Tile | Self::FloatAllInWindows => documents > 0,
            _ => documents > 1,
        }
    }
}

/// The cells of an n-up or Tile All arrangement inside `area`, in reading order. `Tile All
/// Vertically` gives `n` columns, `Horizontally` `n` rows, 3-up Stacked one left cell and two
/// stacked on the right, 4-up and 6-up grids of 2×2 and 3×2.
pub fn arrange_rects(arrangement: Arrangement, area: Rect, n: usize) -> Option<Vec<Rect>> {
    if n < 2 {
        return None;
    }
    let grid = |cols: usize, rows: usize, count: usize| -> Vec<Rect> {
        let (w, h) = (area.width() / cols as f32, area.height() / rows as f32);
        (0..count.min(cols * rows)).map(|i| Rect::from_min_size(area.min + vec2((i % cols) as f32 * w, (i / cols) as f32 * h), vec2(w, h))).collect()
    };
    Some(match arrangement {
        Arrangement::TileAllVertically => grid(n, 1, n),
        Arrangement::TileAllHorizontally => grid(1, n, n),
        Arrangement::TwoUpVertical => grid(2, 1, 2),
        Arrangement::TwoUpHorizontal => grid(1, 2, 2),
        Arrangement::ThreeUpVertical => grid(3, 1, 3),
        Arrangement::ThreeUpHorizontal => grid(1, 3, 3),
        Arrangement::ThreeUpStacked => {
            let (w, h) = (area.width() / 2.0, area.height() / 2.0);
            let right = area.min + vec2(w, 0.0);
            vec![
                Rect::from_min_size(area.min, vec2(w, area.height())),
                Rect::from_min_size(right, vec2(w, h)),
                Rect::from_min_size(right + vec2(0.0, h), vec2(w, h)),
            ]
        }
        Arrangement::FourUp => grid(2, 2, 4),
        Arrangement::SixUp => grid(3, 2, 6),
        Arrangement::Tile => {
            let cols = (n as f32).sqrt().ceil() as usize;
            grid(cols, n.div_ceil(cols), n)
        }
        _ => return None,
    })
}

/// Cascade: the first window at the area's top left, each next one `step` points right and down
/// from the previous one, an axis that would leave `area` starting over at its edge. The host passes its cascade
/// step as `step`.
pub fn cascade_rects(area: Rect, size: Vec2, n: usize, step: f32) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(n);
    // Per axis: a step past the area's edge starts over at the edge.
    let axis = |prev: f32, len: f32, lo: f32, hi: f32| {
        let pos = prev + step;
        if pos < lo || pos + len > hi { lo } else { pos }
    };
    let mut at = area.min;
    for i in 0..n {
        if i > 0 {
            at = pos2(axis(at.x, size.x, area.min.x, area.max.x), axis(at.y, size.y, area.min.y, area.max.y));
        }
        rects.push(Rect::from_min_size(at, size));
    }
    rects
}
