//! Behaviour and labels of a dock area, apart from how it looks ([`super::DockLook`]).

use super::{Sides, ZoneRule};
use crate::localized::{Labels as _, Words};

/// Behaviour and labels of a [`super::DockArea`]. Body renderers retain their own UI style and
/// contents.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DockConfig {
    /// Localize these action labels in the application adapter.
    pub float_label: String,
    pub close_label: String,
    /// Tab menu item that moves one panel into another group (submenu of target panels).
    pub move_panel_to_label: String,
    pub panels_label: String,
    pub resize_label: String,
    pub resize_window_label: String,
    /// Accessibility label of a group's drag bar and top border.
    pub move_group_label: String,
    pub close_group_label: String,
    /// Tooltip and accessible name of the panel-menu button ([`Self::panel_menu_button`]).
    pub panel_menu_label: String,
    pub minimize_label: String,
    pub expand_label: String,
    pub float_group_label: String,
    pub move_group_up_label: String,
    pub move_group_down_label: String,
    /// Group menu item that moves the whole group into another group (submenu of targets).
    pub move_group_to_label: String,
    /// Narrowest a tab gets before the tabs that do not fit go to the overflow menu. `None`:
    /// about three characters and the "…" at the look's font size, plus the tab's padding (52.75 pt
    /// at the Label role's 11.5 pt font with the Comfortable tab padding of 32 pt), so it
    /// follows the font. Tabs whose names are shorter keep their natural width.
    pub min_tab_width: Option<f32>,
    /// Width of the band along the layout's outer edge that splits at the root (28 pt). It is
    /// at most a quarter of the layout's width and height. Not used with
    /// [`Self::side_edge_zones`], whose band is 8 pt.
    pub edge_band: f32,
    /// The split-tree profile. The outer edge band also runs along the left and right edges,
    /// where a drop adds a full-height column. A group's side zones are trapezoids
    /// [`crate::docking::SIDE_EDGE_ZONE_DEPTH`] deep (see [`crate::docking::drop_zone`]), a drag
    /// outlines them, and a split previews the half of the group it takes. Off: only the top and
    /// bottom edges add a full-width row, a group's zones are [`Self::side_zone_fraction`] of its
    /// body, and every preview is a line or a tab outline.
    pub side_edge_zones: bool,
    /// Which sides of the layout's outer edge, and of a group that takes no tabs (a canvas)
    /// without [`Self::new_dock_zones`], take a drop (all four). With [`Self::new_dock_zones`]
    /// a canvas body takes no drop, and without [`Self::side_edge_zones`] the outer left and
    /// right edges are new docks, so only top and bottom apply there. A dock-column app whose
    /// palettes dock only at the left and right sets [`crate::docking::Sides::NONE`] with
    /// [`Self::new_dock_zones`] on. Not used with [`Self::side_edge_zones`]. Groups that take
    /// tabs keep all four sides.
    pub drop_edges: super::Sides,
    /// The share of a group's body, below the tab strip, that means "beside" on each side: of
    /// its width from the left and right edges, of its height from the top and bottom edges
    /// (one third, at most one half). The middle joins the group as a tab. Not used with
    /// [`Self::side_edge_zones`].
    pub side_zone_fraction: f32,
    /// Width of the line that a drop on a band along the layout's outer edge (a new dock, a new
    /// outermost column or the outer edge band), a drop between the icon groups of an iconic
    /// column, and every dock drop with [`Self::side_edge_zones`] preview with, and of the line
    /// [`crate::docking::ZoneRule::Fraction`] gives hosts as a side's preview (4 pt, more than
    /// zero). Other drops beside a group, column or dock preview the rect the moved panels
    /// take. Border bands reach 4 pt past this line into a tab strip.
    pub drop_line_width: f32,
    /// [D] The docking profile. The docked area's left and right edges, where no dock is yet,
    /// take a drop as a new dock (see [`Self::new_dock_band`]). Without
    /// [`Self::side_edge_zones`], the body of a frame or floating group that takes no tabs (a
    /// canvas) takes no drop: a release there floats as over no target.
    pub new_dock_zones: bool,
    /// [D] Height of the band centred on the seam between two stacked groups in a dock column
    /// (28 pt).
    pub seam_band: f32,
    /// [D] Width of the band centred on each edge of a dock column, where a drop makes a new
    /// column (28 pt).
    pub column_edge_band: f32,
    /// [D] Width of the band along the docked area's left and right edges, and inside an empty
    /// dock, that makes a new dock (28 pt).
    pub new_dock_band: f32,
    /// [D] Column menu item that toggles Auto-Collapse Iconic Panels.
    pub auto_collapse_label: String,
    /// [D] Lay docks out right to left: `Side::Start` is the right edge. egui has no app-wide
    /// layout direction, so the host sets this from its locale. Splits keep their sides.
    pub right_to_left: bool,
    /// [D] Accessibility label of a column bar.
    pub move_column_label: String,
    /// [D] Column menu and group menu items.
    pub collapse_to_icons_label: String,
    pub expand_panels_label: String,
    /// [D] Accessibility labels of the column bar's »/« button, distinct from the menu items.
    pub collapse_column_label: String,
    pub expand_column_label: String,
    pub move_window_label: String,
    pub close_window_label: String,
    /// Group menu items Close Other Panels in Group, and Maximize Frame / Restore Frame Size
    /// for groups of the split tree, each shown when its flag is on.
    pub close_others_label: String,
    pub close_others_item: bool,
    pub maximize_label: String,
    pub restore_frame_label: String,
    pub maximize_item: bool,
    /// Shown under the pointer while a drag offers a docking target, when [`Self::float_hint`]
    /// is on. `{key}` stands for the modifier that floats the drop instead: ⌘ on macOS, Ctrl
    /// elsewhere.
    pub float_hint_label: String,
    pub float_hint: bool,
    /// Ctrl/Cmd held when a tab drag of a split-tree group starts makes its drop float, even
    /// when the key is let go during the drag. Off: only the key held at the drop counts.
    pub command_press_floats: bool,
    /// A drag released over no drop target inside the window floats what it carries, such as
    /// a panel dropped on a protected canvas. Off: that release cancels the drag; only
    /// Ctrl/Cmd or a release outside the window floats.
    pub float_on_empty_drop: bool,
    /// Dragging a floating window's title bar onto a drop target docks the whole window there.
    /// Off: the title bar only moves the window; tabs and drag bars dock.
    pub title_bar_docks: bool,
    /// Show the built-in Close items in tab and group menus (Close Panel, Close Tab Group,
    /// Close Other Panels in Group) for closable panels. A host that draws its own Close items
    /// turns this off and keeps [`Permissions::closable`], so its close actions still apply.
    pub close_menu_items: bool,
    /// End every group's tab strip with a ≡ button that opens the selected panel's menu, the one
    /// its tab opens. The host's group header sits before it.
    pub panel_menu_button: bool,
    /// Show × on the title bar of a floating window whose panels are all closable.
    pub close_button: bool,
    /// Show × on the tabs of closable panels in docked and floating groups, for tabs that
    /// act as documents. Off: panels close from their menu. Accordion headers and iconic
    /// columns never show it.
    pub tab_close_buttons: bool,
    /// The mouse wheel over a tab strip selects the next or previous tab, one per notch.
    pub wheel_selects_tabs: bool,
    /// A middle click on a tab closes its panel when the panel is closable.
    pub middle_click_closes_tabs: bool,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            float_label: String::new(),
            close_label: String::new(),
            move_panel_to_label: String::new(),
            panels_label: String::new(),
            resize_label: String::new(),
            resize_window_label: String::new(),
            move_group_label: String::new(),
            close_group_label: String::new(),
            panel_menu_label: String::new(),
            minimize_label: String::new(),
            expand_label: String::new(),
            float_group_label: String::new(),
            move_group_up_label: String::new(),
            move_group_down_label: String::new(),
            move_group_to_label: String::new(),
            min_tab_width: None,
            edge_band: 28.0,
            side_edge_zones: false,
            drop_edges: Sides::ALL,
            side_zone_fraction: 1.0 / 3.0,
            drop_line_width: 4.0,
            new_dock_zones: true,
            seam_band: 28.0,
            column_edge_band: 28.0,
            new_dock_band: 28.0,
            auto_collapse_label: String::new(),
            right_to_left: false,
            move_column_label: String::new(),
            collapse_to_icons_label: String::new(),
            expand_panels_label: String::new(),
            collapse_column_label: String::new(),
            expand_column_label: String::new(),
            move_window_label: String::new(),
            close_window_label: String::new(),
            close_others_label: String::new(),
            close_others_item: false,
            maximize_label: String::new(),
            restore_frame_label: String::new(),
            maximize_item: false,
            float_hint_label: String::new(),
            float_hint: false,
            command_press_floats: false,
            float_on_empty_drop: true,
            title_bar_docks: true,
            close_menu_items: true,
            panel_menu_button: false,
            close_button: true,
            tab_close_buttons: false,
            wheel_selects_tabs: false,
            middle_click_closes_tabs: false,
        }
        .localized_in(&Words::english())
    }
}

impl DockConfig {
    /// The narrowest tab: [`Self::min_tab_width`], or the default that follows `look`'s font.
    pub(super) fn tab_width_floor(&self, look: &super::DockLook) -> f32 {
        self.min_tab_width.unwrap_or(look.font.size * 2.5 + look.tab_padding)
    }

    /// The sides [`Self::drop_edges`] gives in this profile: all four with
    /// [`Self::side_edge_zones`].
    pub(super) fn drop_sides(&self) -> Sides {
        if self.side_edge_zones { Sides::ALL } else { self.drop_edges }
    }

    /// The sides of a frame or floating group that takes no tabs (a canvas) whose body takes a
    /// drop: none in the docking profile ([`Self::new_dock_zones`] without
    /// [`Self::side_edge_zones`]), where only the bands along borders dock beside it, else
    /// [`Self::drop_sides`].
    pub(super) fn canvas_sides(&self) -> Sides {
        if self.side_edge_zones || !self.new_dock_zones { self.drop_sides() } else { Sides::NONE }
    }

    /// The outer edges that take a drop: all four with [`Self::side_edge_zones`], else the top
    /// and bottom of [`Self::drop_edges`].
    pub(super) fn outer_edges(&self) -> Sides {
        let sides = self.drop_sides();
        if self.side_edge_zones { sides } else { Sides { left: false, right: false, ..sides } }
    }

    /// How a group's body divides into drop zones: [`crate::docking::SIDE_EDGE_ZONE_DEPTH`] deep
    /// with [`Self::side_edge_zones`], else [`Self::side_zone_fraction`] of it, with
    /// [`Self::drop_line_width`] previews. Pass it to [`crate::docking::resolve_drop_by`].
    pub fn zone_rule(&self) -> ZoneRule {
        if self.side_edge_zones {
            ZoneRule::Depth(super::SIDE_EDGE_ZONE_DEPTH)
        } else {
            ZoneRule::Fraction { fraction: self.side_zone_fraction, line: self.drop_line_width }
        }
    }

    /// Sizes finite and within 0..=4096, the fraction at most one half, and the line width
    /// positive. `look` supplies the font the default minimum tab width follows.
    pub(super) fn valid(&self, look: &super::DockLook) -> bool {
        [
            self.tab_width_floor(look),
            self.edge_band,
            self.side_zone_fraction,
            self.drop_line_width,
            self.seam_band,
            self.column_edge_band,
            self.new_dock_band,
        ]
        .iter()
        .all(|v| v.is_finite() && (0.0..=4096.0).contains(v))
            && self.drop_line_width > 0.0
            && self.side_zone_fraction <= 0.5
    }
}
