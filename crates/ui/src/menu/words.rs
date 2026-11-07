//! Built-in words of the menu family.

craft_ui::words! {
    /// The words of the menu family: the menu bar's, then the dock's panel, tab-group, column and frame rows.
    pub struct MenuWords {
        bar_more: "menu.barMore" = "…";
        bar_all: "menu.barAll" = "Menu";
        bar_search: "menu.barSearch" = "Search menus";
        bar_no_matches: "menu.barNoMatches" = "No matching commands";
        panel_float: "menu.panelFloat" = "Float";
        panel_move_to: "menu.panelMoveTo" = "Move Panel To";
        panel_close: "menu.panelClose" = "Close";
        panel_auto_collapse: "menu.panelAutoCollapse" = "Auto-Collapse Iconic Panels";
        panel_close_group: "menu.panelCloseGroup" = "Close Tab Group";
        panel_minimize: "menu.panelMinimize" = "Minimize";
        panel_expand: "menu.panelExpand" = "Expand";
        panel_float_group: "menu.panelFloatGroup" = "Float Tab Group";
        panel_move_group_up: "menu.panelMoveGroupUp" = "Move Group Up";
        panel_move_group_down: "menu.panelMoveGroupDown" = "Move Group Down";
        panel_move_group_to: "menu.panelMoveGroupTo" = "Move Group To";
        panel_collapse_to_icons: "menu.panelCollapseToIcons" = "Collapse to Icons";
        panel_expand_panels: "menu.panelExpandPanels" = "Expand Panels";
        panel_close_others: "menu.panelCloseOthers" = "Close Other Panels in Group";
        panel_maximize: "menu.panelMaximize" = "Maximize Frame";
        panel_restore_frame: "menu.panelRestoreFrame" = "Restore Frame Size";
    }
}
