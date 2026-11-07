//! Built-in words of the notices family.

craft_ui::words! {
    /// The words of notices, the settings notices and the notice center; each id is prefixed by its family.
    pub struct NoticeWords {
        dismiss: "notices.dismiss" = "Dismiss";
        dont_show_again: "notices.dontShowAgain" = "Don't Show Again";
        app_fallback: "notices.appFallback" = "The app";
        saved: "notices.saved" = "Preferences saved";
        cant_save_title: "notices.cantSaveTitle" = "Preferences can't be saved";
        reset_title: "notices.resetTitle" = "Some preferences were reset";
        save_failed: template "notices.saveFailed" = "Couldn't save preferences: {error}. Retrying…";
        cant_save_body: template "notices.cantSaveBody" = "{app} keeps retrying; until a save succeeds, preference changes are lost when it closes.";
        reset_body: template "notices.resetBody" = "The preferences file held values {app} can't read for {paths}. Those settings are back to their defaults; the rest of your preferences loaded.";
        moved_aside: template "notices.movedAside" = "Preferences were reset; the unreadable file was kept as {path}.";
        read_only: template "notices.readOnly" = "Preferences won't be saved: {reason}";
    }
}
