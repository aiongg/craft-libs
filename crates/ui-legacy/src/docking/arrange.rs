//! Window › Arrange: Photoshop's document arrangements over the floating document windows and
//! the document tab groups, and the rects they produce.

use egui::Rect;

use crate::menu::{MenuEntry, MenuItem};

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
    /// The command id Photoshop and PhotoCraft use (`window.arrange.tileAllVertically`).
    // pc: crates/ui-egui/src/menu_catalog.rs:707-729.
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
    // pc: crates/ui-egui/src/menus/view_cmds/dispatch.rs (the `window.arrange.*` arms) and
    // Photoshop's Window › Arrange greying.
    pub fn enabled(self, documents: usize, groups: usize, floating: usize) -> bool {
        match self {
            Self::ConsolidateAllToTabs => groups > 1 || floating > 0,
            Self::FloatInWindow => documents > floating,
            Self::NewWindowForDocument => documents > 0,
            Self::TileAllVertically | Self::TileAllHorizontally | Self::Cascade | Self::Tile | Self::FloatAllInWindows => documents > 0,
            _ => documents > 1,
        }
    }

    /// The Window › Arrange submenu rows in Photoshop's order, with separators between the groups,
    /// each enabled per [`Arrangement::enabled`].
    pub fn menu_entries(labels: &ArrangeLabels, documents: usize, groups: usize, floating: usize) -> Vec<MenuEntry<Arrangement>> {
        use Arrangement as A;
        let groups_of: [&[(A, &str)]; 5] = [
            &[
                (A::TileAllVertically, &labels.tile_all_vertically),
                (A::TileAllHorizontally, &labels.tile_all_horizontally),
                (A::TwoUpVertical, &labels.two_up_vertical),
                (A::TwoUpHorizontal, &labels.two_up_horizontal),
                (A::ThreeUpVertical, &labels.three_up_vertical),
                (A::ThreeUpHorizontal, &labels.three_up_horizontal),
                (A::ThreeUpStacked, &labels.three_up_stacked),
                (A::FourUp, &labels.four_up),
                (A::SixUp, &labels.six_up),
            ],
            &[(A::ConsolidateAllToTabs, &labels.consolidate), (A::Cascade, &labels.cascade), (A::Tile, &labels.tile)],
            &[(A::FloatInWindow, &labels.float_in_window), (A::FloatAllInWindows, &labels.float_all)],
            &[(A::NewWindowForDocument, &labels.new_window)],
            &[
                (A::MatchZoom, &labels.match_zoom),
                (A::MatchLocation, &labels.match_location),
                (A::MatchRotation, &labels.match_rotation),
                (A::MatchAll, &labels.match_all),
            ],
        ];
        let mut entries = Vec::new();
        for group in groups_of {
            if !entries.is_empty() {
                entries.push(MenuEntry::Separator);
            }
            entries.extend(group.iter().map(|&(a, label)| MenuEntry::from(MenuItem::new(label, a).enabled(a.enabled(documents, groups, floating)))));
        }
        entries
    }
}

/// The cells of an n-up or Tile All arrangement inside `area`, in reading order. `Tile All
/// Vertically` gives `n` columns, `Horizontally` `n` rows, 3-up Stacked one left cell and two
/// stacked on the right, 4-up and 6-up grids of 2×2 and 3×2.
// pc: crates/ui-egui/src/menus/view_cmds/arrange.rs:126 `cells`.
pub fn arrange_rects(arrangement: Arrangement, area: Rect, n: usize) -> Option<Vec<Rect>> {
    if n < 2 {
        return None;
    }
    let grid = |cols: usize, rows: usize, count: usize| -> Vec<Rect> {
        let (w, h) = (area.width() / cols as f32, area.height() / rows as f32);
        (0..count.min(cols * rows))
            .map(|i| Rect::from_min_size(area.min + egui::vec2((i % cols) as f32 * w, (i / cols) as f32 * h), egui::vec2(w, h)))
            .collect()
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
            let right = area.min + egui::vec2(w, 0.0);
            vec![
                Rect::from_min_size(area.min, egui::vec2(w, area.height())),
                Rect::from_min_size(right, egui::vec2(w, h)),
                Rect::from_min_size(right + egui::vec2(0.0, h), egui::vec2(w, h)),
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
/// from the previous one, an axis that would leave `area` starting over at its edge (the rule of
/// `craft_window::cascade`, whose `CASCADE_STEP` hosts pass as `step`).
pub fn cascade_rects(area: Rect, size: egui::Vec2, n: usize, step: f32) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(n);
    // Per axis, as `craft_window::cascade`: a step past the area's edge starts over at the edge.
    let axis = |prev: f32, len: f32, lo: f32, hi: f32| {
        let pos = prev + step;
        if pos < lo || pos + len > hi { lo } else { pos }
    };
    let mut at = area.min;
    for i in 0..n {
        if i > 0 {
            at = egui::pos2(axis(at.x, size.x, area.min.x, area.max.x), axis(at.y, size.y, area.min.y, area.max.y));
        }
        rects.push(Rect::from_min_size(at, size));
    }
    rects
}

crate::words! {
    /// Window › Arrange labels, English by default.
    #[non_exhaustive]
    pub struct ArrangeLabels {
        arrange: "window.arrange" = "Arrange";
        tile_all_vertically: "window.arrange.tileAllVertically" = "Tile All Vertically";
        tile_all_horizontally: "window.arrange.tileAllHorizontally" = "Tile All Horizontally";
        two_up_vertical: "window.arrange.twoUpVertical" = "2-up Vertical";
        two_up_horizontal: "window.arrange.twoUpHorizontal" = "2-up Horizontal";
        three_up_vertical: "window.arrange.threeUpVertical" = "3-up Vertical";
        three_up_horizontal: "window.arrange.threeUpHorizontal" = "3-up Horizontal";
        three_up_stacked: "window.arrange.threeUpStacked" = "3-up Stacked";
        four_up: "window.arrange.fourUp" = "4-up";
        six_up: "window.arrange.sixUp" = "6-up";
        consolidate: "window.arrange.consolidateAllToTabs" = "Consolidate All to Tabs";
        cascade: "window.arrange.cascade" = "Cascade";
        tile: "window.arrange.tile" = "Tile";
        float_in_window: "window.arrange.floatInWindow" = "Float in Window";
        float_all: "window.arrange.floatAllInWindows" = "Float All in Windows";
        new_window: "window.arrange.newWindowForDocument" = "New Window for Document";
        match_zoom: "window.arrange.matchZoom" = "Match Zoom";
        match_location: "window.arrange.matchLocation" = "Match Location";
        match_rotation: "window.arrange.matchRotation" = "Match Rotation";
        match_all: "window.arrange.matchAll" = "Match All";
    }
}
