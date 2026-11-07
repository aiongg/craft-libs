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
