//! Built-in words of the menu family.

craft_ui::words! {
    /// The words of the menu bar.
    pub struct MenuBarWords {
        more: "menu.barMore" = "…";
        all: "menu.barAll" = "Menu";
        search: "menu.barSearch" = "Search menus";
        no_matches: "menu.barNoMatches" = "No matching commands";
    }
}
craft_ui::words! {
    /// The dock's own panel, tab-group, column and frame menu rows.
    pub struct PanelMenuWords {
        float: "menu.panelFloat" = "Float";
        move_to: "menu.panelMoveTo" = "Move Panel To";
        close: "menu.panelClose" = "Close";
        auto_collapse: "menu.panelAutoCollapse" = "Auto-Collapse Iconic Panels";
        close_group: "menu.panelCloseGroup" = "Close Tab Group";
        minimize: "menu.panelMinimize" = "Minimize";
        expand: "menu.panelExpand" = "Expand";
        float_group: "menu.panelFloatGroup" = "Float Tab Group";
        move_group_up: "menu.panelMoveGroupUp" = "Move Group Up";
        move_group_down: "menu.panelMoveGroupDown" = "Move Group Down";
        move_group_to: "menu.panelMoveGroupTo" = "Move Group To";
        collapse_to_icons: "menu.panelCollapseToIcons" = "Collapse to Icons";
        expand_panels: "menu.panelExpandPanels" = "Expand Panels";
        close_others: "menu.panelCloseOthers" = "Close Other Panels in Group";
        maximize: "menu.panelMaximize" = "Maximize Frame";
        restore_frame: "menu.panelRestoreFrame" = "Restore Frame Size";
    }
}
