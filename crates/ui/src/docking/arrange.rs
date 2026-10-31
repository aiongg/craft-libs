//! Window › Arrange: Photoshop's document arrangements over the floating document windows and
//! the document tab groups, and the rects they produce.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Rect;

use crate::menu::MenuEntry;

/// One Window › Arrange entry. The n-up entries split the document area into that many
/// cells and put one document group in each; the rest act on windows and views.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Arrangement {
    TileAllVertically,
    TileAllHorizontally,
    TwoUpVertical,
    TwoUpHorizontal,
    ThreeUpVertical,
    ThreeUpHorizontal,
    ThreeUpStacked,
    FourUp,
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
    MatchZoom,
    MatchLocation,
    MatchRotation,
    MatchAll,
}

impl Arrangement {
    /// The command id Photoshop and PhotoCraft use (`window.arrange.tileAllVertically`).
    // pc: crates/ui-egui/src/menu_catalog.rs:707-729.
    pub fn command_id(self) -> &'static str {
        todo!()
    }

    /// The entry for a command id, `None` for other ids.
    pub fn from_command_id(id: &str) -> Option<Self> {
        todo!()
    }

    /// How many document groups an n-up or tile arrangement wants, `None` for the window and
    /// view entries.
    pub fn group_count(self, documents: usize) -> Option<usize> {
        todo!()
    }

    /// Whether the entry is enabled with `documents` open and `floating` floating windows:
    /// n-up needs at least two documents, Float in Window a docked one, Consolidate a second
    /// group or a floating window, the Match entries at least two documents.
    // pc: crates/ui-egui/src/menus/view_cmds/dispatch.rs (the `window.arrange.*` arms) and
    // Photoshop's Window › Arrange greying.
    pub fn enabled(self, documents: usize, groups: usize, floating: usize) -> bool {
        todo!()
    }
}

/// The cells of an n-up or Tile All arrangement inside `area`, in reading order. `Tile All
/// Vertically` gives `n` columns, `Horizontally` `n` rows, 3-up Stacked one left cell and two
/// stacked on the right, 4-up and 6-up grids of 2×2 and 3×2.
// pc: crates/ui-egui/src/menus/view_cmds/arrange.rs:126 `cells`.
pub fn arrange_rects(arrangement: Arrangement, area: Rect, n: usize) -> Option<Vec<Rect>> {
    todo!()
}

/// Cascade: each window `step` points right and down from the previous one, wrapping to the top
/// when it would leave `area`.
// pc: crates/ui-egui/src/menus/view_cmds/arrange.rs (the `cascade` arm); the step is
// craft_window::CASCADE_STEP and the placement craft_window::cascade.
pub fn cascade_rects(area: Rect, size: egui::Vec2, n: usize, step: f32) -> Vec<Rect> {
    todo!()
}

/// The Window › Arrange submenu rows in Photoshop's order, with separators between the groups,
/// each enabled per [`Arrangement::enabled`].
pub fn arrange_entries(labels: &ArrangeLabels, documents: usize, groups: usize, floating: usize) -> Vec<MenuEntry<Arrangement>> {
    todo!()
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
