//! Built-in words of the docking family.

craft_ui::words! {
    /// The words of the dock's handles and chrome.
    pub struct DockWords {
        resize: "docking.resize" = "Resize panels";
        /// The name of the dock area for assistive technology.
        panels: "docking.panels" = "Panels";
        resize_window: "docking.resizeWindow" = "Resize panel window";
        /// The name of a group's drag bar and top border. `{name}` stands for the label of the panel the group shows.
        move_group: template "docking.moveGroup" = "Move tab group";
        /// The name of a column bar. `{name}` stands for the labels of the panels the column's groups show, joined
        /// by ", ".
        move_column: template "docking.moveColumn" = "Move panel column";
        collapse_column: "docking.collapseColumn" = "Collapse column to icons";
        expand_column: "docking.expandColumn" = "Expand column";
        move_window: "docking.moveWindow" = "Move panel window";
        close_window: "docking.closeWindow" = "Close panel window";
        /// Shown under the pointer while a drag offers a docking target and `DockConfig::float_hint` is on. `{key}`
        /// stands for the modifier that floats the drop instead.
        float_hint: template "docking.floatHint" = "Hold {key} to undock";
        /// The tooltip and accessible name of the panel-menu button.
        panel_menu: "docking.panelMenu" = "Panel Menu";

        /// The name of the workspace switcher's select.
        choose_workspace: "workspaceSwitcher.choose" = "Choose a workspace";
        /// The name of the switcher's command menu.
        workspace_menu: "workspaceSwitcher.menu" = "Workspace";

        // PaneArea
        /// The name of a pane's menu.
        pane_menu: "paneArea.menu" = "Pane options";
    }
}
