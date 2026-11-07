//! Built-in words of the shell family.

// App shell and canvas preset.
craft_ui::words! {
    /// The words of the app shell and its canvas preset: the names of the regions of the window.
    pub struct AppShellWords {
        window: "shell.window" = "Window";
        document: "shell.document" = "Document";
        document_tabs: "shell.documentTabs" = "Document Tabs";
        options: "shell.options" = "Options";
        tools: "shell.tools" = "Tools";
        status: "shell.status" = "Status";
        screen_mode: "shell.screenMode" = "Screen Mode";
    }
}

// Start screen.
craft_ui::words! {
    /// The words of the start screen. The labels of the recent files' and pinned folders' commands are the commands'.
    pub struct StartWords {
        new: "start.new" = "New…";
        open: "start.open" = "Open…";
        recent: "start.recent" = "Recent";
        no_recent: "start.noRecent" = "Files you open show here.";
        view: "start.view" = "View";
        grid_view: "start.gridView" = "Grid";
        list_view: "start.listView" = "List";
        sort: "start.sort" = "Sort";
        sort_opened: "start.sortOpened" = "Last Opened";
        sort_name: "start.sortName" = "Name";
        sort_created: "start.sortCreated" = "Created";
        pinned: "start.pinned" = "Pinned Folders";
        no_pinned: "start.noPinned" = "Pin the folders you open often.";
        pin_folder: "start.pinFolder" = "Pin Folder…";
        templates: "start.templates" = "Templates";
        recent_menu: "start.recentMenu" = "Recent file";
        pinned_menu: "start.pinnedMenu" = "Pinned folder";
    }
}
